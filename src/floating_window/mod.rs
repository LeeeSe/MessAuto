mod app;

use crate::ipc;

pub fn maybe_start_floating_window() -> bool {
    if let Some((code, source)) = ipc::parse_args() {
        app::VerificationCodeApp::run(code, source);
        return true;
    }
    false
}
