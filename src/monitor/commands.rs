use tokio::sync::oneshot;

#[derive(Debug)]
pub enum MonitorCommand {
    StartMessageMonitoring,
    StopMessageMonitoring,
    StartEmailMonitoring,
    StopEmailMonitoring,
    StartWebhook,
    StopWebhook,
    RestartWebhook,
    StartNgrok,
    StopNgrok,
    RestartNgrok,
    #[allow(dead_code)]
    GetStatus(oneshot::Sender<String>),
}
