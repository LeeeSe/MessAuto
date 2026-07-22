use log::{error, info, warn};
use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tokio::task::JoinHandle;

use crate::clipboard;
use crate::config::Config;
use crate::ipc;
use crate::parser;
use crate::permissions;

static LAST_PROCESSED_REC_ID: Mutex<i64> = Mutex::new(0);

// 钉钉在 macOS 通知中心数据库 app 表中的 bundle identifier（小写存储）
const DINGTALK_BUNDLE_ID: &str = "com.alibaba.dingtalkmac";

// 轮询间隔。macOS 通知中心库是高频 WAL 库，notify(FSEvents) 文件监听对它不可靠
// （实测新通知写入不触发事件），故钉钉监听改为主动定时轮询，不依赖文件事件。
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// 钉钉验证码监听器：独立 tokio 轮询任务，周期性查通知中心库。
pub struct DingTalkPoller {
    task: Option<JoinHandle<()>>,
}

impl DingTalkPoller {
    pub fn start() -> Self {
        // 以当前最大 rec_id 为增量基线，避免把历史通知当新验证码
        if let Ok(rec_id) = get_latest_rec_id() {
            *LAST_PROCESSED_REC_ID.lock().unwrap() = rec_id;
            info!("Initialized last processed DingTalk rec_id to {}", rec_id);
        }

        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(POLL_INTERVAL);
            loop {
                interval.tick().await;
                // sqlite3 查询是阻塞调用，放到 blocking 线程池，避免卡住 async executor
                if let Err(e) = tokio::task::spawn_blocking(poll_once).await {
                    error!("DingTalk poll task join error: {}", e);
                }
            }
        });

        Self { task: Some(task) }
    }

    pub async fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            let _ = task.await;
        }
    }
}

impl Drop for DingTalkPoller {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

// macOS 通知中心数据库路径（钉钉横幅通知落盘于此）
fn notification_db_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let home_dir = env::var("HOME")?;
    Ok(PathBuf::from(&home_dir)
        .join("Library/Group Containers/group.com.apple.usernoted/db2/db"))
}

// 获取当前钉钉通知的最大 rec_id，作为增量基线
fn get_latest_rec_id() -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    let db_path = notification_db_path()?;
    let sql = format!(
        "SELECT MAX(r.rec_id) FROM record r JOIN app a ON r.app_id = a.app_id \
         WHERE lower(a.identifier) = '{}';",
        DINGTALK_BUNDLE_ID
    );

    let output = std::process::Command::new("sqlite3")
        .arg(db_path.to_str().unwrap())
        .arg(sql)
        .output()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !output_str.is_empty() {
            return Ok(output_str.parse()?);
        }
    }
    Ok(0)
}

// 一次轮询：查 rec_id 大于基线的新钉钉通知，解码正文并提取验证码
fn poll_once() {
    let db_path = match notification_db_path() {
        Ok(p) => p,
        Err(e) => {
            error!("DingTalk: failed to resolve notification db path: {}", e);
            return;
        }
    };

    let last_rec_id = *LAST_PROCESSED_REC_ID.lock().unwrap();

    // quote(data) 输出 X'62706c...' 十六进制串，避免 blob 二进制经 CLI 文本管道损坏
    let sql = format!(
        "SELECT r.rec_id, quote(r.data) FROM record r JOIN app a ON r.app_id = a.app_id \
         WHERE lower(a.identifier) = '{}' AND r.rec_id > {} \
         ORDER BY r.rec_id ASC LIMIT 20;",
        DINGTALK_BUNDLE_ID, last_rec_id
    );

    let output = match std::process::Command::new("sqlite3")
        .arg(db_path.to_str().unwrap())
        .arg(sql)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            error!("DingTalk: failed to run sqlite3: {}", e);
            return;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Error querying DingTalk notifications: {}", stderr);
        if stderr.contains("attempt to write a readonly database")
            || stderr.contains("permission denied")
            || stderr.contains("unable to open database")
        {
            warn!("Permission error detected when accessing notification database");
            if !permissions::check_full_disk_access() {
                permissions::show_permission_dialog();
            }
        }
        return;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let hit = output_str.lines().filter(|l| !l.trim().is_empty()).count();
    if hit > 0 {
        info!(
            "[dingtalk-diag] 轮询 rec_id>{}，新钉钉记录 {} 条",
            last_rec_id, hit
        );
    }

    let mut max_rec_id = last_rec_id;

    for line in output_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // 每行： rec_id|X'hex...'
        let (rec_id_str, quoted) = match line.split_once('|') {
            Some(v) => v,
            None => continue,
        };

        if let Ok(rec_id) = rec_id_str.trim().parse::<i64>() {
            if rec_id > max_rec_id {
                max_rec_id = rec_id;
            }
        }

        let content = match decode_notification_body(quoted.trim()) {
            Some(c) => c,
            None => continue,
        };

        info!("[dingtalk-diag] rec_id={} 正文: {}", rec_id_str, content);

        if let Some(code) = parser::extract_verification_code(&content) {
            info!("Found verification code in DingTalk notification: {}", code);
            handle_code(&code);
        }
    }

    if max_rec_id > last_rec_id {
        *LAST_PROCESSED_REC_ID.lock().unwrap() = max_rec_id;
    }
}

// 解析 quote(data) 的 X'..' 十六进制串为二进制 plist，抽取 titl + body 作为待匹配文本
fn decode_notification_body(quoted: &str) -> Option<String> {
    // 期望形如 X'62706c...'
    let hex = quoted.strip_prefix("X'")?.strip_suffix('\'')?;
    if hex.is_empty() {
        return None;
    }

    let bytes = hex_decode(hex)?;
    let value: plist::Value = plist::from_bytes(&bytes).ok()?;
    let dict = value.as_dictionary()?;
    let req = dict.get("req")?.as_dictionary()?;

    let titl = req.get("titl").and_then(|v| v.as_string()).unwrap_or("");
    let body = req.get("body").and_then(|v| v.as_string()).unwrap_or("");

    let combined = format!("{} {}", titl, body);
    if combined.trim().is_empty() {
        None
    } else {
        Some(combined)
    }
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16)?;
        let lo = (bytes[i + 1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(out)
}

// 与 message.rs / email.rs 一致的验证码处置逻辑（悬浮窗 / 直接输入 / 剪贴板）
fn handle_code(code: &str) {
    let config = Config::load().unwrap_or_default();

    if config.floating_window {
        match ipc::spawn_floating_window(code, "DingTalk") {
            Ok(_) => {}
            Err(e) => error!("Failed to spawn floating window: {}", e),
        }
        return;
    }

    if config.direct_input {
        if let Err(e) = clipboard::auto_paste(true, code) {
            error!("Failed to direct input verification code: {}", e);
        } else {
            info!("Direct input verification code: {}", code);
            if config.auto_enter {
                if let Err(e) = clipboard::press_enter() {
                    error!("Failed to press enter key: {}", e);
                }
            }
        }
        return;
    }

    // 剪贴板模式（默认）
    if let Err(e) = clipboard::copy_to_clipboard(code) {
        error!("Failed to copy verification code to clipboard: {}", e);
        return;
    }
    info!("Auto-copied verification code to clipboard: {}", code);

    if config.auto_paste {
        if let Err(e) = clipboard::auto_paste(false, code) {
            error!("Failed to auto-paste verification code: {}", e);
            return;
        }
        info!("Auto-pasted verification code: {}", code);
    }

    if config.auto_enter {
        if let Err(e) = clipboard::press_enter() {
            error!("Failed to press enter key: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_decode_roundtrip() {
        assert_eq!(hex_decode("62706c").unwrap(), vec![0x62, 0x70, 0x6c]);
        assert!(hex_decode("6").is_none()); // 奇数长度
    }

    // 解析一条真实钉钉通知 blob（可选：设 DING_BLOB_HEX 环境变量传入 quote(data) 的 hex）
    #[test]
    fn test_decode_real_notification_from_env() {
        if let Ok(hex) = std::env::var("DING_BLOB_HEX") {
            let quoted = format!("X'{}'", hex);
            let body = decode_notification_body(&quoted);
            println!("decoded body = {:?}", body);
            assert!(body.is_some(), "应能从真实钉钉通知 blob 解出文本");
        }
    }
}
