use super::{
    commands::MonitorCommand, email::EmailProcessor, message::MessageProcessor,
    watcher::FileWatcher,
};
use tokio::sync::mpsc::Receiver;

pub struct MonitorActor {
    receiver: Receiver<MonitorCommand>,
    message_watcher: Option<FileWatcher<MessageProcessor>>,
    email_watcher: Option<FileWatcher<EmailProcessor>>,
}

impl MonitorActor {
    pub fn new(receiver: Receiver<MonitorCommand>) -> Self {
        Self {
            receiver,
            message_watcher: None,
            email_watcher: None,
        }
    }

    pub async fn run(&mut self) {
        log::info!("Monitor Actor is running and waiting for commands.");
        while let Some(command) = self.receiver.recv().await {
            self.handle_command(command).await;
        }
    }

    async fn handle_command(&mut self, command: MonitorCommand) {
        log::debug!("MonitorActor received command: {:?}", command);
        match command {
            MonitorCommand::StartMessageMonitoring => {
                if self.message_watcher.is_some() {
                    log::warn!("Message monitoring is already running.");
                    return;
                }
                log::info!("Starting message monitoring...");
                let mut watcher = FileWatcher::new(MessageProcessor::new());
                if let Err(e) = watcher.start() {
                    log::error!("Failed to start message watcher: {}", e);
                } else {
                    self.message_watcher = Some(watcher);
                    log::info!("Message monitoring started successfully.");
                }
            }
            MonitorCommand::StopMessageMonitoring => {
                if let Some(mut watcher) = self.message_watcher.take() {
                    log::info!("Stopping message monitoring...");
                    watcher.stop().await;
                    log::info!("Message monitoring stopped successfully.");
                } else {
                    log::warn!("Message monitoring is not running, nothing to stop.");
                }
            }
            MonitorCommand::StartEmailMonitoring => {
                if self.email_watcher.is_some() {
                    log::warn!("Email monitoring is already running.");
                    return;
                }
                log::info!("Starting email monitoring...");
                let mut watcher = FileWatcher::new(EmailProcessor::new());
                if let Err(e) = watcher.start() {
                    log::error!("Failed to start email watcher: {}", e);
                } else {
                    self.email_watcher = Some(watcher);
                    log::info!("Email monitoring started successfully.");
                }
            }
            MonitorCommand::StopEmailMonitoring => {
                if let Some(mut watcher) = self.email_watcher.take() {
                    log::info!("Stopping email monitoring...");
                    watcher.stop().await; // 安全地停止
                    log::info!("Email monitoring stopped successfully.");
                } else {
                    log::warn!("Email monitoring is not running, nothing to stop.");
                }
            }
            MonitorCommand::GetStatus(responder) => {
                let mut status = "Monitoring Status:\n".to_string();
                status.push_str(&format!(
                    "- Message Monitoring: {}\n",
                    if self.message_watcher.is_some() {
                        "Running"
                    } else {
                        "Stopped"
                    }
                ));
                status.push_str(&format!(
                    "- Email Monitoring: {}",
                    if self.email_watcher.is_some() {
                        "Running"
                    } else {
                        "Stopped"
                    }
                ));
                let _ = responder.send(status);
            }
        }
    }
}
