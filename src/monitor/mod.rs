pub mod email;
pub mod message;
pub mod watcher;


use crate::config::Config;
use tokio::time::{self, Duration};
use watcher::FileWatcher;
use email::EmailProcessor;
use message::MessageProcessor;
use log::{debug, error};

use std::sync::atomic::{AtomicBool, Ordering};

static RESTART_MONITORING: AtomicBool = AtomicBool::new(false);

pub fn signal_restart() {
    RESTART_MONITORING.store(true, Ordering::SeqCst);
}

pub async fn start_monitoring() {
    let mut message_watcher: Option<FileWatcher<MessageProcessor>> = None;
    let mut email_watcher: Option<FileWatcher<EmailProcessor>> = None;
    
    loop {
        // 检查是否需要重启监控
        if RESTART_MONITORING.load(Ordering::SeqCst) {
            debug!("Restarting monitoring services due to config change...");
            RESTART_MONITORING.store(false, Ordering::SeqCst);
            
            // 停止现有的监控器（通过丢弃它们）
            message_watcher = None;
            email_watcher = None;
            
            // 等待一下让线程完全停止
            time::sleep(Duration::from_millis(100)).await;
        }
        
        let config = Config::load().unwrap_or_default();
        
        // 消息监控总是启用（短信/iMessage）
        if message_watcher.is_none() {
            let mut watcher = FileWatcher::new(MessageProcessor::new());
            if let Err(e) = watcher.start() {
                error!("Failed to start message watcher: {}", e);
            } else {
                message_watcher = Some(watcher);
                debug!("Message monitoring started");
            }
        }
        
        // 邮件监控根据配置决定
        if config.listen_email {
            if email_watcher.is_none() {
                let mut watcher = FileWatcher::new(EmailProcessor::new());
                if let Err(e) = watcher.start() {
                    error!("Failed to start email watcher: {}", e);
                } else {
                    email_watcher = Some(watcher);
                    debug!("Email monitoring started");
                }
            }
        } else {
            if email_watcher.is_some() {
                debug!("Stopping email monitoring due to config change");
                email_watcher = None;
            }
        }

        debug!("Monitoring services running...");
        time::sleep(Duration::from_secs(5)).await;
    }
}
