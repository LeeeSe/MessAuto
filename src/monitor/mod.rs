pub mod actor;
pub mod commands;
pub mod email;
pub mod message;
pub mod watcher;

use crate::config::Config;
use actor::MonitorActor;
use commands::MonitorCommand;
use tokio::sync::mpsc;

pub fn start_monitoring_actor() -> mpsc::Sender<MonitorCommand> {
    let (sender, receiver) = mpsc::channel(32);
    let mut actor = MonitorActor::new(receiver);
    let sender_clone = sender.clone();

    tokio::spawn(async move {
        let config = Config::load().unwrap_or_default();
        if config.listen_message {
            if let Err(e) = sender_clone
                .send(MonitorCommand::StartMessageMonitoring)
                .await
            {
                log::error!("Failed to send initial start message command: {}", e);
            }
        }
        if config.listen_email {
            if let Err(e) = sender_clone
                .send(MonitorCommand::StartEmailMonitoring)
                .await
            {
                log::error!("Failed to send initial start email command: {}", e);
            }
        }

        actor.run().await;
    });

    sender
}
