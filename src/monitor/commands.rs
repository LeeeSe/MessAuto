use tokio::sync::oneshot;

#[derive(Debug)]
pub enum MonitorCommand {
    StartMessageMonitoring,
    StopMessageMonitoring,
    StartEmailMonitoring,
    StopEmailMonitoring,
    GetStatus(oneshot::Sender<String>),
}
