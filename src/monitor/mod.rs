pub mod actor;
pub mod commands;
pub mod email;
pub mod message;
pub mod ngrok;
pub mod watcher;
pub mod webhook;

use crate::config::Config;
use actor::MonitorActor;
use commands::MonitorCommand;
use rust_i18n::t;
use tokio::sync::mpsc;

pub fn start_monitoring_actor(
    config: std::sync::Arc<std::sync::Mutex<Config>>,
) -> mpsc::Sender<MonitorCommand> {
    let (sender, receiver) = mpsc::channel(32);
    let mut actor = MonitorActor::new(receiver, config.clone());
    let sender_clone = sender.clone();

    tokio::spawn(async move {
        let config = config.lock().unwrap().clone();
        if config.listen_message {
            if let Err(e) = sender_clone
                .send(MonitorCommand::StartMessageMonitoring)
                .await
            {
                log::error!(
                    "{}",
                    t!("errors.failed_to_send_initial_start_message", error = e)
                );
            }
        }
        if config.listen_email {
            if let Err(e) = sender_clone
                .send(MonitorCommand::StartEmailMonitoring)
                .await
            {
                log::error!(
                    "{}",
                    t!("errors.failed_to_send_initial_start_email", error = e)
                );
            }
        }

        if config.listen_webhook {
            let _ = sender_clone.send(MonitorCommand::StartWebhook).await;
        }
        drop(sender_clone);
        actor.run().await;
    });

    sender
}
