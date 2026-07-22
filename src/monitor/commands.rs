use tokio::sync::oneshot;

#[derive(Debug)]
pub enum MonitorCommand {
    StartMessageMonitoring,
    StopMessageMonitoring,
    StartEmailMonitoring,
    StopEmailMonitoring,
    StartDingTalkMonitoring,
    StopDingTalkMonitoring,
    #[allow(dead_code)]
    GetStatus(oneshot::Sender<String>),
}
