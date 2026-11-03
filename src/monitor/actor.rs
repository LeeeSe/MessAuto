use super::{
    commands::MonitorCommand,
    email::EmailProcessor,
    message::MessageProcessor,
    ngrok::{NgrokError, NgrokTunnel},
    watcher::FileWatcher,
    webhook::WebhookServer,
};
use crate::{
    config::Config,
    notification,
    status::{self, ServiceState, TunnelState},
};
use rust_i18n::t;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;

pub struct MonitorActor {
    receiver: Receiver<MonitorCommand>,
    config: Arc<Mutex<Config>>,
    webhook_server: Option<WebhookServer>,
    ngrok: Option<NgrokTunnel>,
    ngrok_connecting: Option<JoinHandle<Result<NgrokTunnel, NgrokError>>>,
    message_watcher: Option<FileWatcher<MessageProcessor>>,
    email_watcher: Option<FileWatcher<EmailProcessor>>,
}

impl MonitorActor {
    pub fn new(receiver: Receiver<MonitorCommand>, config: Arc<Mutex<Config>>) -> Self {
        Self {
            receiver,
            config,
            webhook_server: None,
            ngrok: None,
            ngrok_connecting: None,
            message_watcher: None,
            email_watcher: None,
        }
    }

    pub async fn run(&mut self) {
        log::info!("{}", t!("actor.actor_running"));
        loop {
            tokio::select! {
                command = self.receiver.recv() => match command {
                    Some(command) => self.handle_command(command).await,
                    None => break,
                },
                result = async { self.ngrok_connecting.as_mut().unwrap().await }, if self.ngrok_connecting.is_some() => {
                    self.ngrok_connecting = None;
                    match result {
                        Ok(Ok(tunnel)) => self.ngrok_online(tunnel),
                        Ok(Err(error)) => self.ngrok_failed(error),
                        Err(error) => self.ngrok_failed(NgrokError::Other(error.to_string())),
                    }
                }
            }
        }
        self.stop_webhook().await;
    }

    async fn handle_command(&mut self, command: MonitorCommand) {
        log::debug!(
            "{}",
            t!("actor.received_command", command = format!("{:?}", command))
        );
        match command {
            MonitorCommand::StartWebhook => self.start_webhook().await,
            MonitorCommand::StopWebhook => self.stop_webhook().await,
            MonitorCommand::RestartWebhook => {
                self.stop_webhook().await;
                if self.config.lock().unwrap().listen_webhook {
                    self.start_webhook().await;
                }
            }
            MonitorCommand::StartNgrok => self.start_ngrok(),
            MonitorCommand::StopNgrok => self.stop_ngrok().await,
            MonitorCommand::RestartNgrok => {
                self.stop_ngrok().await;
                self.start_ngrok();
            }
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
    async fn start_webhook(&mut self) {
        if self.webhook_server.is_some() {
            return;
        }
        let config = self.config.lock().unwrap().clone();
        match WebhookServer::start(
            &config.webhook_host,
            config.webhook_port,
            config.webhook_token,
        )
        .await
        {
            Ok(server) => {
                status::update(|s| {
                    s.webhook = ServiceState::Running {
                        addr: server.addr().to_string(),
                    }
                });
                self.webhook_server = Some(server);
                self.start_ngrok();
            }
            Err(error) => {
                status::update(|s| s.webhook = ServiceState::Failed(error.to_string()));
                notification::notify("MessAuto", &t!("webhook.start_failed", error = error));
            }
        }
    }

    async fn stop_webhook(&mut self) {
        self.stop_ngrok().await;
        if let Some(mut server) = self.webhook_server.take() {
            server.stop().await;
        }
        status::update(|s| s.webhook = ServiceState::Stopped);
    }

    fn start_ngrok(&mut self) {
        if self.webhook_server.is_none() || self.ngrok.is_some() || self.ngrok_connecting.is_some()
        {
            return;
        }
        let config = self.config.lock().unwrap().clone();
        if !config.ngrok_enabled {
            status::update(|s| s.ngrok = TunnelState::Disabled);
        } else if config.ngrok_authtoken.trim().is_empty() {
            status::update(|s| s.ngrok = TunnelState::NeedToken);
        } else if config.ngrok_domain.trim().is_empty() {
            status::update(|s| s.ngrok = TunnelState::NeedDomain);
            notification::notify("MessAuto", &t!("ngrok.need_domain"));
        } else {
            status::update(|s| s.ngrok = TunnelState::Connecting);
            let port = self.webhook_server.as_ref().unwrap().addr().port();
            self.ngrok_connecting = Some(tokio::spawn(async move {
                NgrokTunnel::start(&config.ngrok_authtoken, &config.ngrok_domain, port).await
            }));
        }
    }

    async fn stop_ngrok(&mut self) {
        if let Some(task) = self.ngrok_connecting.take() {
            task.abort();
            // 若连接恰好完成，仍需主动关闭它。
            if let Ok(Ok(tunnel)) = task.await {
                tunnel.stop().await;
            }
        }
        if let Some(tunnel) = self.ngrok.take() {
            tunnel.stop().await;
        }
        status::update(|s| s.ngrok = TunnelState::Disabled);
    }

    fn ngrok_online(&mut self, tunnel: NgrokTunnel) {
        let url = tunnel.url().to_string();
        status::update(|s| s.ngrok = TunnelState::Online { url });
        self.ngrok = Some(tunnel);
    }

    fn ngrok_failed(&self, error: NgrokError) {
        let message = match error {
            NgrokError::InvalidToken => {
                status::update(|s| s.ngrok = TunnelState::NeedToken);
                t!("ngrok.invalid_token").to_string()
            }
            NgrokError::DomainInUse => {
                let message = t!("ngrok.domain_in_use").to_string();
                status::update(|s| s.ngrok = TunnelState::Failed(message.clone()));
                message
            }
            NgrokError::Other(error) => {
                status::update(|s| s.ngrok = TunnelState::Failed(error.clone()));
                error
            }
        };
        log::warn!("ngrok: {}", message);
        notification::notify("MessAuto", &message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn request(port: u16, token: &str) -> String {
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let body = r#"{"value":"hello"}"#;
        let request = format!(
            "POST /webhook HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        response
    }

    #[tokio::test]
    async fn rotating_token_and_stopping_ngrok_preserve_local_service() {
        let mut settings = Config::default();
        settings.webhook_host = "127.0.0.1".into();
        settings.webhook_port = 0;
        settings.webhook_token = "old-token".into();
        settings.ngrok_enabled = false;
        let config = Arc::new(Mutex::new(settings));
        let (_sender, receiver) = tokio::sync::mpsc::channel(4);
        let mut actor = MonitorActor::new(receiver, config.clone());
        actor.handle_command(MonitorCommand::StartWebhook).await;
        let port = actor.webhook_server.as_ref().unwrap().addr().port();
        assert!(request(port, "old-token").await.starts_with("HTTP/1.1 200"));
        actor.handle_command(MonitorCommand::StopNgrok).await;
        assert!(request(port, "old-token").await.starts_with("HTTP/1.1 200"));
        config.lock().unwrap().webhook_token = "new-token".into();
        actor.handle_command(MonitorCommand::RestartWebhook).await;
        let port = actor.webhook_server.as_ref().unwrap().addr().port();
        assert!(request(port, "old-token").await.starts_with("HTTP/1.1 401"));
        assert!(request(port, "new-token").await.starts_with("HTTP/1.1 200"));
        actor.handle_command(MonitorCommand::StopWebhook).await;
        assert!(actor.webhook_server.is_none() && actor.ngrok.is_none());
    }
}
