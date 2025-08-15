use std::path::{Path, PathBuf};
use std::env;
use std::fs;
use std::io::Read;
use notify::{EventKind, RecursiveMode};
use log::{info, debug, error};

use crate::config::Config;
use crate::parser;
use crate::ipc;
use crate::clipboard;
use super::watcher::FileProcessor;

#[derive(Clone)]
pub struct EmailProcessor;

impl EmailProcessor {
    pub fn new() -> Self {
        Self {}
    }

    fn read_emlx(&self, path: &Path) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut file = fs::File::open(path)?;
        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)?;

        let message = emlx::parse_emlx(&buffer)?;

        let content = String::from_utf8(message.message.to_vec())?;

        Ok(content)
    }
}

impl FileProcessor for EmailProcessor {
    fn get_watch_path(&self) -> PathBuf {
        let home_dir = env::var("HOME").expect("Failed to get HOME directory");
        PathBuf::from(&home_dir).join("Library/Mail/V10")
    }

    fn get_file_pattern(&self) -> &str {
        ".emlx"
    }

    fn get_recursive_mode(&self) -> RecursiveMode {
        RecursiveMode::Recursive
    }

    fn process_file(&self, path: &Path, event_kind: &EventKind) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if event_kind != &EventKind::Create(notify::event::CreateKind::File) {
            return Ok(());
        }

        if !path.to_str().unwrap().contains("INBOX.mbox") {
            return Ok(());
        }

        debug!("检测到新邮件被创建: {:?}", &path);

        let content = self.read_emlx(Path::new(&path.to_string_lossy().replace(".tmp", "")))?;

        debug!("邮件内容: {}", content);

        if let Some(code) = parser::extract_verification_code(&content) {
            info!("Found verification code in email: {}", code);
            
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
            debug!("No verification code found in email");
        }

        Ok(())
    }
}
