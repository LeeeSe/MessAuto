use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::env;
use notify::{EventKind, RecursiveMode};
use log::{info, error, debug, warn};

use crate::config::Config;
use crate::parser;
use crate::ipc;
use crate::clipboard;
use super::watcher::FileProcessor;

// 跟踪最后处理的消息ID
static LAST_PROCESSED_ROWID: Mutex<i64> = Mutex::new(0);

#[derive(Clone)]
pub struct MessageProcessor;

impl MessageProcessor {
    pub fn new() -> Self {
        if let Ok(rowid) = Self::get_latest_message_rowid() {
            let mut last_processed = LAST_PROCESSED_ROWID.lock().unwrap();
            *last_processed = rowid;
            info!("Initialized last processed ROWID to {}", rowid);
        }

        Self {}
    }

    // 获取数据库中最新的消息ROWID
    fn get_latest_message_rowid() -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let home_dir = env::var("HOME")?;
        let db_path = PathBuf::from(&home_dir).join("Library/Messages/chat.db");

        let output = std::process::Command::new("sqlite3")
            .arg(db_path.to_str().unwrap())
            .arg("SELECT MAX(ROWID) FROM message;")
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !output_str.is_empty() {
                return Ok(output_str.parse()?);
            }
        }

        Ok(0)
    }
}

impl FileProcessor for MessageProcessor {
    fn get_watch_path(&self) -> PathBuf {
        let home_dir = env::var("HOME").expect("Failed to get HOME directory");
        PathBuf::from(&home_dir).join("Library/Messages/NickNameCache")
    }

    fn get_file_pattern(&self) -> &str {
        ".db"
    }

    fn get_recursive_mode(&self) -> RecursiveMode {
        RecursiveMode::Recursive
    }

    fn process_file(&self, path: &Path, event_kind: &EventKind) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("process_file_event_kind: {:?}", event_kind);
        if event_kind != &EventKind::Modify(notify::event::ModifyKind::Metadata(notify::event::MetadataKind::Any)) {
            return Ok(());
        }

        debug!("Message file change detected: {:?}", path);
        debug!("检测到 NickNameCache 文件变化，可能有新消息");

        let home_dir = env::var("HOME")?;
        let db_path = PathBuf::from(&home_dir).join("Library/Messages/chat.db");
        debug!("Using database: {:?}", db_path);

        let last_rowid;
        {
            let last_processed = LAST_PROCESSED_ROWID.lock().unwrap();
            last_rowid = *last_processed;
            debug!("Last processed ROWID: {}", last_rowid);
        }

        let sql = format!(
            "SELECT m.ROWID, m.text, h.id as phone_number, datetime(m.date/1000000000 + strftime('%s', '2001-01-01'), 'unixepoch', 'localtime') as date_formatted
             FROM message m
             LEFT JOIN handle h ON m.handle_id = h.ROWID
             WHERE m.ROWID > {}
             ORDER BY m.ROWID DESC
             LIMIT 10;",
            last_rowid
        );
        debug!("SQL query: {}", sql);

        // 执行SQLite查询
        debug!("Executing SQLite query...");
        let output = std::process::Command::new("sqlite3")
            .arg(db_path.to_str().unwrap())
            .arg(sql)
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            debug!("SQLite output: {}", output_str);

            let messages = parse_sqlite_output(&output_str);
            debug!("Parsed {} messages", messages.len());

            for (i, message) in messages.iter().enumerate() {
                debug!("Processing message {}: {}", i, message);
                if let Some(code) = parser::extract_verification_code(message) {
                    info!("Found verification code in message: {}", code);
                    
                    let config = Config::load().unwrap_or_default();
                    
                    // 如果悬浮窗启用，只显示悬浮窗，不自动输入
                    if config.floating_window {
                        match ipc::spawn_floating_window(&code) {
                            Ok(_) => debug!("Floating window spawned successfully"),
                            Err(e) => error!("Failed to spawn floating window: {}", e),
                        }
                    } else {
                        // 悬浮窗关闭时，根据配置自动处理
                        if config.direct_input {
                            // 直接输入模式，不占用剪贴板
                            if let Err(e) = clipboard::auto_paste(true, &code) {
                                error!("Failed to direct input verification code: {}", e);
                            } else {
                                info!("Direct input verification code: {}", code);
                            }
                        } else if config.auto_copy {
                            // 剪贴板模式
                            if let Err(e) = clipboard::copy_to_clipboard(&code) {
                                error!("Failed to copy verification code to clipboard: {}", e);
                            } else {
                                info!("Auto-copied verification code to clipboard: {}", code);
                                
                                // 如果 auto_paste 启用，自动粘贴
                                if config.auto_paste {
                                    if let Err(e) = clipboard::auto_paste(false, &code) {
                                        error!("Failed to auto-paste verification code: {}", e);
                                    } else {
                                        info!("Auto-pasted verification code: {}", code);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    debug!("No verification code found in message");
                }
            }

            if !messages.is_empty() {
                if let Ok(rowid) = get_last_rowid(&output_str) {
                    debug!("Updating last processed ROWID to {}", rowid);
                    let mut last_processed = LAST_PROCESSED_ROWID.lock().unwrap();
                    *last_processed = rowid;
                } else {
                    warn!("Failed to get last ROWID from output");
                }
            } else {
                debug!("No new messages found");
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Error executing SQLite query: {}", stderr);
            debug!("Command status: {:?}", output.status);
        }

        Ok(())
    }
}

fn parse_sqlite_output(output: &str) -> Vec<String> {
    let mut result = Vec::new();

    for line in output.lines() {
        if !line.trim().is_empty() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 2 {
                let id = parts[0].trim();
                let text = parts[1].trim();
                debug!("New message found with ID {}: {}", id, text);
                result.push(text.to_string());
            }
        }
    }

    result
}

fn get_last_rowid(output: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    let lines: Vec<&str> = output.trim().lines().collect();
    if !lines.is_empty() && !lines[0].is_empty() {
        let parts: Vec<&str> = lines[0].split('|').collect();
        if !parts.is_empty() {
            return Ok(parts[0].parse()?);
        }
    }
    Err("No valid ROWID found".into())
}
