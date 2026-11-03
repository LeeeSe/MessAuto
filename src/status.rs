use crate::tray::UserEvent;
use std::sync::{Mutex, OnceLock};
use winit::event_loop::EventLoopProxy;

#[derive(Clone, Debug, Default)]
pub enum ServiceState {
    #[default]
    Stopped,
    Running {
        addr: String,
    },
    Failed(String),
}

#[derive(Clone, Debug, Default)]
pub enum TunnelState {
    #[default]
    Disabled,
    NeedToken,
    NeedDomain,
    Connecting,
    Online {
        url: String,
    },
    Failed(String),
}

#[derive(Clone, Default)]
pub struct RuntimeStatus {
    pub webhook: ServiceState,
    pub ngrok: TunnelState,
}

static STATUS: Mutex<RuntimeStatus> = Mutex::new(RuntimeStatus {
    webhook: ServiceState::Stopped,
    ngrok: TunnelState::Disabled,
});
static NOTIFIER: OnceLock<EventLoopProxy<UserEvent>> = OnceLock::new();

pub fn set_notifier(proxy: EventLoopProxy<UserEvent>) {
    let _ = NOTIFIER.set(proxy);
}

pub fn update(f: impl FnOnce(&mut RuntimeStatus)) {
    f(&mut STATUS.lock().unwrap());
    if let Some(proxy) = NOTIFIER.get() {
        let _ = proxy.send_event(UserEvent::StatusChanged);
    }
}

pub fn snapshot() -> RuntimeStatus {
    STATUS.lock().unwrap().clone()
}
