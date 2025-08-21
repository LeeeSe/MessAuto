use tokio::sync::oneshot;

#[derive(Debug)]
pub enum MonitorCommand {
    StartMessageMonitoring,
    StopMessageMonitoring,
    StartEmailMonitoring,
    StopEmailMonitoring,
    #[allow(dead_code)]
    GetStatus(oneshot::Sender<String>),
}
