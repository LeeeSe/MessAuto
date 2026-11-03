use crate::{clipboard, config::Config, ipc};
use log::{debug, error, info};
use rust_i18n::t;

/// 根据当前配置显示验证码浮窗，或执行直接输入、剪贴板粘贴及回车。
pub fn handle_verification_code(code: &str, source: &str) {
    let config = Config::load().unwrap_or_default();

    if config.floating_window {
        match ipc::spawn_floating_window(code, source) {
            Ok(_) => debug!("Floating window spawned successfully"),
            Err(e) => error!("Failed to spawn floating window: {}", e),
        }
    } else {
        if config.direct_input {
            if let Err(e) = clipboard::auto_paste(true, code) {
                error!("{}", t!("monitor.failed_to_direct_input", error = e));
            } else {
                info!(
                    "{}",
                    t!("monitor.direct_input_verification_code", code = code)
                );

                if config.auto_enter {
                    if let Err(e) = clipboard::press_enter() {
                        error!("{}", t!("monitor.failed_to_press_enter", error = e));
                    } else {
                        info!("{}", t!("monitor.auto_pressed_enter"));
                    }
                }
            }
        } else {
            if let Err(e) = clipboard::copy_to_clipboard(code) {
                error!("{}", t!("monitor.failed_to_copy_to_clipboard", error = e));
            } else {
                info!("{}", t!("monitor.auto_copied_to_clipboard", code = code));

                if config.auto_paste {
                    if let Err(e) = clipboard::auto_paste(false, code) {
                        error!("{}", t!("monitor.failed_to_auto_paste", error = e));
                    } else {
                        info!(
                            "{}",
                            t!("monitor.auto_pasted_verification_code", code = code)
                        );

                        if config.auto_enter {
                            if let Err(e) = clipboard::press_enter() {
                                error!("{}", t!("monitor.failed_to_press_enter", error = e));
                            } else {
                                info!("{}", t!("monitor.auto_pressed_enter"));
                            }
                        }
                    }
                } else {
                    if config.auto_enter {
                        if let Err(e) = clipboard::press_enter() {
                            error!("{}", t!("monitor.failed_to_press_enter", error = e));
                        } else {
                            info!("{}", t!("monitor.auto_pressed_enter"));
                        }
                    }
                }
            }
        }
    }
}
