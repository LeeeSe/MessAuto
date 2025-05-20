use std::fs::File;
use std::process::Command;

use log::{info, trace, warn};
use native_dialog::MessageDialog;
use rust_i18n::t;
use simplelog::{
    ColorChoice, CombinedLogger, ConfigBuilder, LevelFilter, TermLogger, TerminalMode, WriteLogger,
};
use tao::platform::macos::ActivationPolicy;
use tao::{
    event_loop::{ControlFlow, EventLoopBuilder},
    platform::macos::EventLoopExtMacOS,
};
use tray_icon::{menu::MenuEvent, TrayIconEvent};

use MessAuto::{
    auto_launch, check_accessibility, check_accessibility_with_no_action, check_full_disk_access,
    config_path, get_sys_locale, log_path, mail_thread, messages_thread, read_config, TrayIcon,
    TrayMenu, TrayMenuItems,
};

rust_i18n::i18n!("locales");
pub fn main() {
    let logger_config = ConfigBuilder::new()
        .set_time_offset_to_local()
        .unwrap()
        .build();
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            logger_config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            logger_config.clone(),
            File::create(log_path()).unwrap(),
        ),
    ])
    .unwrap();
    info!("{}", t!("log-initialization-completed"));

    let locale = get_sys_locale();
    info!("{}: {}", t!("detect-and-set-app-language-to"), locale);

    rust_i18n::set_locale(locale);

    check_full_disk_access();

    let mut event_loop = EventLoopBuilder::new().build();

    event_loop.set_activation_policy(ActivationPolicy::Accessory);
    let auto = auto_launch();

    let mut config = read_config();

    messages_thread();
    if config.listening_to_mail {
        mail_thread();
    }

    // 禁用自动更新
    // let (tx, rx) = mpsc::channel();
    // update_thread(tx);

    let tray_menu_items = TrayMenuItems::build(&config);
    let tray_menu = TrayMenu::build(&tray_menu_items);
    let mut tray_icon = TrayIcon::build(tray_menu);
    tray_icon.as_mut().unwrap().set_icon_as_template(true);

    if config.auto_paste {
        if check_accessibility_with_no_action() {
            info!("{}", t!("accessibility-permission-granted"));
        } else {
            warn!("{}", t!("accessibility-permission-denied"));
            tray_menu_items.check_auto_paste.set_checked(false);
            tray_menu_items.check_auto_return.set_checked(false);
            tray_menu_items.recover_clipboard.set_checked(false);
        }
    }

    if config.hide_icon_forever {
        tray_icon
            .as_mut()
            .unwrap()
            .set_visible(false)
            .expect("set_visible failed");
    } else {
        tray_icon
            .as_mut()
            .unwrap()
            .set_visible(true)
            .expect("set_visible failed");
    }

    let menu_channel = MenuEvent::receiver();
    let tray_channel = TrayIconEvent::receiver();

    event_loop.run(move |_event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        // set upgrade thread in main loop to ensure recv rx in anytime
        // if let Ok(msg) = rx.try_recv() {
        //     if msg {
        //         let yes = MessageDialog::new()
        //             .set_title(&t!("new-version"))
        //             .set_text(&t!("new-version-text"))
        //             .show_confirm()
        //             .unwrap();
        //         if yes {
        //             match replace_old_version() {
        //                 Ok(_) => {
        //                     info!("{}", t!("binary-file-replace-success"));
        //                     let reboot = MessageDialog::new()
        //                         .set_title(&t!("update-success"))
        //                         .set_text(&t!("update-success-text"))
        //                         .show_confirm()
        //                         .unwrap();
        //                     if reboot {
        //                         tray_icon.take();
        //                         *control_flow = ControlFlow::Exit;
        //                         let _ = Command::new("open")
        //                             .arg(get_current_exe_path())
        //                             .output()
        //                             .expect("Failed to open MessAuto");
        //                     }
        //                 }
        //                 Err(e) => {
        //                     warn!("{}: {}", t!("binary-file-replace-failed"), e);
        //                     MessageDialog::new()
        //                         .set_title(&t!("update-failed"))
        //                         .set_text(&e.to_string())
        //                         .show_alert()
        //                         .unwrap();
        //                 }
        //             }
        //         }
        //     }
        // }
        if let Ok(event) = menu_channel.try_recv() {
            if event.id == tray_menu_items.quit_i.id() {
                tray_icon.take();
                *control_flow = ControlFlow::Exit;
            } else if event.id == tray_menu_items.check_hide_icon_for_now.id() {
                tray_icon
                    .as_mut()
                    .unwrap()
                    .set_visible(false)
                    .expect("set_visible failed");
            } else if event.id == tray_menu_items.check_hide_icon_forever.id() {
                config.hide_icon_forever = true;
                tray_icon
                    .as_mut()
                    .unwrap()
                    .set_visible(false)
                    .expect("set_visible failed");
                config.update().expect("failed to update config");
            } else if event.id == tray_menu_items.check_auto_paste.id() {
                if tray_menu_items.check_auto_paste.is_checked() {
                    if check_accessibility() {
                        config.auto_paste = true;
                        info!("{}", t!("enable-auto-paste"));
                        tray_menu_items
                            .check_auto_paste
                            .set_checked(config.auto_paste);
                        tray_menu_items
                            .check_auto_return
                            .set_enabled(config.auto_paste);
                    } else {
                        config.auto_paste = false;
                        tray_menu_items
                            .check_auto_paste
                            .set_checked(config.auto_paste);
                    }
                } else {
                    config.auto_paste = false;
                    config.auto_return = false;
                    info!("{}", t!("disable-auto-paste"));
                    info!("{}", t!("disable-auto-return"));
                    tray_menu_items.check_auto_return.set_enabled(false);
                    tray_menu_items.check_auto_return.set_checked(false);
                }
                config.update().expect("failed to update config");
            } else if event.id == tray_menu_items.check_auto_return.id() {
                config.auto_return = tray_menu_items.check_auto_return.is_checked();
                if config.auto_return {
                    info!("{}", t!("enable-auto-return"));
                } else {
                    info!("{}", t!("disable-auto-return"));
                }
                config.update().expect("failed to update config");
            } else if event.id == tray_menu_items.check_launch_at_login.id() {
                if tray_menu_items.check_launch_at_login.is_checked() {
                    auto.enable().expect("failed to enable auto launch");
                    info!("{}", t!("set-launch-at-login"));
                    config.launch_at_login = true;
                } else {
                    auto.disable().expect("failed to disable auto launch");
                    info!("{}", t!("disable-launch-at-login"));
                    config.launch_at_login = false;
                }
                config.update().expect("failed to update config");
                // } else if event.id == tray_menu_items.add_flag.id() {
                //     println!("add flag");
            } else if event.id == tray_menu_items.maconfig.id() {
                let _ = Command::new("open")
                    .arg(config_path())
                    .output()
                    .expect("Failed to open config");
            } else if event.id == tray_menu_items.logs.id() {
                let _ = Command::new("open")
                    .arg(log_path())
                    .output()
                    .expect("Failed to open logs");
            } else if event.id == tray_menu_items.listening_to_mail.id() {
                if tray_menu_items.listening_to_mail.is_checked() {
                    config.listening_to_mail = true;
                    mail_thread();
                    info!("{}", t!("mail-listening-enabled"));
                } else {
                    config.listening_to_mail = false;
                    info!("{}", t!("mail-listening-disabled"));
                }
                config.update().expect("failed to update config");
            } else if event.id == tray_menu_items.float_window.id() {
                if tray_menu_items.float_window.is_checked() {
                    config.float_window = true;
                    config.auto_paste = true;
                    tray_menu_items.check_auto_paste.set_checked(true);
                    tray_menu_items.check_auto_return.set_enabled(true);
                    tray_menu_items.check_auto_paste.set_enabled(false);
                    info!("{}", t!("float-window-enabled"));
                } else {
                    config.float_window = false;
                    if !tray_menu_items.recover_clipboard.is_checked() {
                        tray_menu_items.check_auto_paste.set_enabled(true);
                    }
                    info!("{}", t!("float-window-disabled"));
                }
                config.update().expect("failed to update config");
            } else if event.id == tray_menu_items.recover_clipboard.id() {
                if tray_menu_items.recover_clipboard.is_checked() {
                    config.recover_clipboard = true;
                    config.auto_paste = true;
                    tray_menu_items.check_auto_paste.set_checked(true);
                    tray_menu_items.check_auto_return.set_enabled(true);
                    tray_menu_items.check_auto_paste.set_enabled(false);
                    info!("{}", t!("recover-clipboard-enabled"));
                    info!("{}", t!("enable-auto-paste"));
                } else {
                    config.recover_clipboard = false;
                    if !tray_menu_items.float_window.is_checked() {
                        tray_menu_items.check_auto_paste.set_enabled(true);
                    }
                    info!("{}", t!("recover-clipboard-disabled"));
                }
                config.update().expect("failed to update config");
            } else {
                warn!("{}", t!("unknown-operation"));
            }
        }
        if let Ok(event) = tray_channel.try_recv() {
            trace!("{event:?}");
        }
    });
}
