pub mod email;
pub mod message;
pub mod watcher;


use tokio::time::{self, Duration};
use watcher::FileWatcher;
use email::EmailProcessor;
use message::MessageProcessor;
use log::{debug, error};

pub async fn start_monitoring() {
    let mut message_watcher = FileWatcher::new(MessageProcessor::new());
    if let Err(e) = message_watcher.start() {
        error!("Failed to start message watcher: {}", e);
    }

    let mut email_watcher = FileWatcher::new(EmailProcessor::new());
    if let Err(e) = email_watcher.start() {
        error!("Failed to start email watcher: {}", e);
    }

    loop {
        debug!("Monitoring services running...");
        time::sleep(Duration::from_secs(60)).await;
    }
}
