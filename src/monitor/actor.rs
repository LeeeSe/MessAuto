use super::{
    commands::MonitorCommand, email::EmailProcessor, message::MessageProcessor,
    watcher::FileWatcher,
};
use rust_i18n::t;
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
        log::info!("{}", t!("actor.actor_running"));
        while let Some(command) = self.receiver.recv().await {
            self.handle_command(command).await;
        }
    }

    async fn handle_command(&mut self, command: MonitorCommand) {
        log::debug!("{}", t!("actor.received_command", command = format!("{:?}", command)));
        match command {
            MonitorCommand::StartMessageMonitoring => {
                if self.message_watcher.is_some() {
                    log::warn!("{}", t!("actor.message_monitoring_already_running"));
                    return;
                }
                log::info!("{}", t!("actor.starting_message_monitoring"));
                let mut watcher = FileWatcher::new(MessageProcessor::new());
                if let Err(e) = watcher.start() {
                    log::error!("{}", t!("actor.failed_to_start_message_watcher", error = e));
                } else {
                    self.message_watcher = Some(watcher);
                    log::info!("{}", t!("actor.message_monitoring_started"));
                }
            }
            MonitorCommand::StopMessageMonitoring => {
                if let Some(mut watcher) = self.message_watcher.take() {
                    log::info!("{}", t!("actor.stopping_message_monitoring"));
                    watcher.stop().await;
                    log::info!("{}", t!("actor.message_monitoring_stopped"));
                } else {
                    log::warn!("{}", t!("actor.message_monitoring_not_running"));
                }
            }
            MonitorCommand::StartEmailMonitoring => {
                if self.email_watcher.is_some() {
                    log::warn!("{}", t!("actor.email_monitoring_already_running"));
                    return;
                }
                log::info!("{}", t!("actor.starting_email_monitoring"));
                let mut watcher = FileWatcher::new(EmailProcessor::new());
                if let Err(e) = watcher.start() {
                    log::error!("{}", t!("actor.failed_to_start_email_watcher", error = e));
                } else {
                    self.email_watcher = Some(watcher);
                    log::info!("{}", t!("actor.email_monitoring_started"));
                }
            }
            MonitorCommand::StopEmailMonitoring => {
                if let Some(mut watcher) = self.email_watcher.take() {
                    log::info!("{}", t!("actor.stopping_email_monitoring"));
                    watcher.stop().await; // 安全地停止
                    log::info!("{}", t!("actor.email_monitoring_stopped"));
                } else {
                    log::warn!("{}", t!("actor.email_monitoring_not_running"));
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
