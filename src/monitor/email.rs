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

        let mime_message = MimeMessage::parse(&raw_content)?;

        // 尝试提取纯文本内容
        let body_content = match extract_plain_text_content(&mime_message) {
            Some(plain_text) => {
                info!("{}", t!("monitor.mail_content", content = &plain_text));
                plain_text
            }
            None => {
                // 没有找到 text/plain 部分，返回错误
                warn!("{}", t!("monitor.no_plain_text_found"));
                return Err("No plain text content found in email".into());
            }
        };

        debug!(
            "{}",
            t!(
                "monitor.email_subject",
                subject = format!("{:?}", mime_message.headers.get("Subject".to_string()))
            )
        );
        debug!("Extracted plain text length: {}", body_content.len());

        Ok(body_content)
    }
}

fn extract_plain_text_content(mime_message: &MimeMessage) -> Option<String> {
    // 策略1: 检查是否有解析好的子部分
    if !mime_message.children.is_empty() {
        for child in &mime_message.children {
            if let Some(content_type) = child.headers.get("Content-Type".to_string()) {
                if content_type
                    .to_string()
                    .to_lowercase()
                    .contains("text/plain")
                {
                    if let Ok(decoded) = child.decoded_body_string() {
                        let cleaned = decoded.trim().to_string();
                        if !cleaned.is_empty() {
                            return Some(cleaned);
                        }
                    }
                }
            }
        }
        // 如果有子部分但没有找到 text/plain，返回 None
        return None;
    }

    // 策略2: 如果没有子部分，尝试手动解析多部分内容
    let raw_content = mime_message
        .decoded_body_string()
        .unwrap_or_else(|_| mime_message.body.clone());

    extract_plain_text_from_multipart(&raw_content)
}

fn extract_plain_text_from_multipart(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut in_plain_text_section = false;
    let mut reading_headers = false;
    let mut found_plain_text_type = false;

    for line in lines {
        // 检测 MIME 边界
        if line.starts_with("------") {
            in_plain_text_section = false;
            reading_headers = true;
            found_plain_text_type = false;
            continue;
        }

        // 处理头部信息
        if reading_headers {
            // 空行表示头部结束
            if line.trim().is_empty() {
                reading_headers = false;
                in_plain_text_section = found_plain_text_type;
                continue;
            }

            // 检查是否是 text/plain 类型（只检查 Content-Type 头）
            if line.to_lowercase().starts_with("content-type:")
                && line.to_lowercase().contains("text/plain")
            {
                found_plain_text_type = true;
            }
            continue;
        }

        // 收集 text/plain 内容
        if in_plain_text_section {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                result.push(trimmed);
            }
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result.join(" "))
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

        debug!(
            "{}",
            t!("monitor.new_email_created", path = format!("{:?}", &path))
        );

        let content = match self.read_emlx(Path::new(&path.to_string_lossy().replace(".tmp", ""))) {
            Ok(content) => content,
            Err(e) => {
                debug!("Failed to extract plain text from email: {}", e);
                return Ok(()); // 跳过这个邮件，不是错误
            }
        };

        debug!("{}", t!("monitor.email_content", content = content));

        if let Some(code) = parser::extract_verification_code(&content) {
            info!(
                "{}",
                t!("monitor.found_verification_code_email", code = code)
            );
            info!("{}", t!("monitor.mail_content", content = &content));

            let config = Config::load().unwrap_or_default();

            if config.floating_window {
                match ipc::spawn_floating_window(&code) {
                    Ok(_) => debug!("Floating window spawned successfully"),
                    Err(e) => error!("Failed to spawn floating window: {}", e),
                }
            } else {
                if config.direct_input {
                    if let Err(e) = clipboard::auto_paste(true, &code) {
                        error!("{}", t!("monitor.failed_to_direct_input", error = e));
                    } else {
                        info!(
                            "{}",
                            t!("monitor.direct_input_verification_code", code = code)
                        );

                        if config.auto_enter {
                            if let Err(e) = clipboard::press_enter() {
                                error!("{}", t!("monitor.failed_to_press_enter", error = e));
                            } else {
                                info!("{}", t!("monitor.auto_pressed_enter"));
                            }
                        }
                    }
                } else {
                    if let Err(e) = clipboard::copy_to_clipboard(&code) {
                        error!("{}", t!("monitor.failed_to_copy_to_clipboard", error = e));
                    } else {
                        info!("{}", t!("monitor.auto_copied_to_clipboard", code = code));

                        if config.auto_paste {
                            if let Err(e) = clipboard::auto_paste(false, &code) {
                                error!("{}", t!("monitor.failed_to_auto_paste", error = e));
                            } else {
                                info!(
                                    "{}",
                                    t!("monitor.auto_pasted_verification_code", code = code)
                                );

                                if config.auto_enter {
                                    if let Err(e) = clipboard::press_enter() {
                                        error!(
                                            "{}",
                                            t!("monitor.failed_to_press_enter", error = e)
                                        );
                                    } else {
                                        info!("{}", t!("monitor.auto_pressed_enter"));
                                    }
                                }
                            }
                        } else {
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
