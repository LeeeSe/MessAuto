use email::MimeMessage;
use log::warn;
use log::{debug, error, info};
use notify::{EventKind, RecursiveMode};
use rust_i18n::t;
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::watcher::FileProcessor;
use crate::clipboard;
use crate::config::Config;
use crate::ipc;
use crate::parser;

rust_i18n::i18n!("../locales");

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

        let raw_content = String::from_utf8(message.message.to_vec())?;

        // 使用 email 库进一步解析邮件内容
        let mime_message = MimeMessage::parse(&raw_content)?;

        // 解析邮件 body，处理可能的编码
        let body_content = match mime_message.decoded_body_string() {
            Ok(decoded) => {
                info!("{}", t!("monitor.mail_content", content = &decoded));
                decoded
            }
            Err(_) => {
                // 如果解码失败，使用原始 body
                warn!("{}", t!("monitor.parse_body_failed", content = &mime_message.body));
                mime_message.body.clone()
            }
        };

        debug!(
            "{}",
            t!("monitor.email_subject", subject = format!("{:?}", mime_message.headers.get("Subject".to_string())))
        );
        debug!("{}", t!("monitor.email_body_length", length = body_content.len()));

        Ok(body_content)
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

    fn process_file(
        &self,
        path: &Path,
        event_kind: &EventKind,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if event_kind != &EventKind::Create(notify::event::CreateKind::File) {
            return Ok(());
        }

        if !path.to_str().unwrap().contains("INBOX.mbox") {
            return Ok(());
        }

        debug!("{}", t!("monitor.new_email_created", path = format!("{:?}", &path)));

        let content = self.read_emlx(Path::new(&path.to_string_lossy().replace(".tmp", "")))?;

        debug!("{}", t!("monitor.email_content", content = content));

        if let Some(code) = parser::extract_verification_code(&content) {
            info!("{}", t!("monitor.found_verification_code_email", code = code));
            info!("{}", t!("monitor.mail_content", content = &content));

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
                        error!("{}", t!("monitor.failed_to_direct_input", error = e));
                    } else {
                        info!("{}", t!("monitor.direct_input_verification_code", code = code));

                        // 如果 auto_enter 启用，在直接输入后立即按下回车键
                        if config.auto_enter {
                            if let Err(e) = clipboard::press_enter() {
                                error!("{}", t!("monitor.failed_to_press_enter", error = e));
                            } else {
                                info!("{}", t!("monitor.auto_pressed_enter"));
                            }
                        }
                    }
                } else {
                    // 剪贴板模式（默认行为）
                    if let Err(e) = clipboard::copy_to_clipboard(&code) {
                        error!("{}", t!("monitor.failed_to_copy_to_clipboard", error = e));
                    } else {
                        info!("{}", t!("monitor.auto_copied_to_clipboard", code = code));

                        // 如果 auto_paste 启用，自动粘贴
                        if config.auto_paste {
                            if let Err(e) = clipboard::auto_paste(false, &code) {
                                error!("{}", t!("monitor.failed_to_auto_paste", error = e));
                            } else {
                                info!("{}", t!("monitor.auto_pasted_verification_code", code = code));

                                // 如果 auto_enter 启用，在自动粘贴后立即按下回车键
                                if config.auto_enter {
                                    if let Err(e) = clipboard::press_enter() {
                                        error!("{}", t!("monitor.failed_to_press_enter", error = e));
                                    } else {
                                        info!("{}", t!("monitor.auto_pressed_enter"));
                                    }
                                }
                            }
                        } else {
                            // 如果 auto_paste 未启用但 auto_enter 启用，在复制后立即按下回车键
                            if config.auto_enter {
                                if let Err(e) = clipboard::press_enter() {
                                    error!("{}", t!("monitor.failed_to_press_enter", error = e));
                                } else {
                                    info!("{}", t!("monitor.auto_pressed_enter"));
                                }
                            }
                        }
                    }
                }
            }
        } else {
            debug!("{}", t!("monitor.no_verification_code_email"));
        }

        Ok(())
    }
}
