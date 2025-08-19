use crate::notification;
use cargo_packager_updater::{Config, Update, check_update, semver::Version};
use log::{error, info};
use rust_i18n::t;
use std::env;
use std::process::{Command, exit};
use std::thread;

fn get_current_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else {
        "universal"
    }
}

fn get_endpoint() -> String {
    let arch = get_current_arch();
    let endpoint = format!(
        "https://github.com/LeeeSe/MessAuto/releases/latest/download/update-{}.json",
        arch
    );
    info!("{}", t!("updater.update_endpoint", endpoint = endpoint));
    endpoint
}

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

const PUB_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDY2NkEyQTU4M0Y3RTM3RkUKUldUK04zNC9XQ3BxWmhvQi84YkVYQUpOa2N5WWFDM2lhRXh5dDE0VE85SlRNejJ5VVJBR2JvYjEK";

pub fn check_for_updates() {
    thread::spawn(move || {
        info!("{}", t!("updater.checking_updates"));
        info!("{}", t!("updater.current_arch", arch = get_current_arch()));

        let current_version = match Version::parse(CURRENT_VERSION) {
            Ok(version) => version,
            Err(e) => {
                error!(
                    "{}",
                    t!("updater.failed_to_parse_version", error = e.to_string())
                );
                return;
            }
        };

        let endpoint = get_endpoint();

        let config = Config {
            pubkey: PUB_KEY.into(),
            endpoints: vec![endpoint.parse().unwrap()],
            ..Default::default()
        };

        match check_update(current_version, config) {
            Ok(Some(update)) => {
                info!(
                    "{}",
                    t!(
                        "updater.new_version_found",
                        version = update.version.to_string()
                    )
                );
                match download_update(update) {
                    Ok((update_obj, update_bytes)) => {
                        info!("{}", t!("updater.update_download_complete"));
                        if let Err(e) = install_update(update_obj.clone(), update_bytes) {
                            error!(
                                "{}",
                                t!("updater.update_check_failed", error = e.to_string())
                            );
                        }
                        if show_restart_notification(&update_obj.version) {
                            restart_app();
                        } else {
                            info!("{}", t!("updater.user_canceled_update"));
                        }
                    }
                    Err(e) => {
                        error!(
                            "{}",
                            t!("updater.update_download_failed", error = e.to_string())
                        );
                    }
                }
            }
            Ok(None) => {
                info!("{}", t!("updater.already_up_to_date"));
            }
            Err(e) => {
                error!(
                    "{}",
                    t!("updater.update_check_failed", error = e.to_string())
                );
            }
        }
    });
}

fn download_update(update: Update) -> Result<(Update, Vec<u8>), Box<dyn std::error::Error>> {
    info!("{}", t!("updater.downloading_update"));
    info!("{:?}", update.download_url);

    let update_bytes = update.download()?;
    info!("{}", t!("updater.update_downloaded"));

    Ok((update, update_bytes))
}

fn install_update(update: Update, update_bytes: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    info!("{}", t!("updater.installing_update"));

    // 安装更新
    update.install(update_bytes)?;
    info!("{}", t!("updater.update_installed"));

    Ok(())
}

fn show_restart_notification(version: &str) -> bool {
    info!(
        "{}",
        t!("updater.new_version_downloaded", version = version)
    );

    // 使用系统通知询问用户是否要安装更新
    let title = t!("updater.update_available");
    let content = format!(
        "{}\n\n{}",
        t!("updater.new_version_installed", version = version),
        t!("updater.choose_restart_manually")
    );

    let user_choice = notification::dialog(
        &title,
        &content,
        &t!("updater.restart_now"),
        &t!("updater.restart_later"),
    );

    if user_choice {
        info!("{}", t!("updater.user_chosen_restart"));
    } else {
        info!("{}", t!("updater.user_chosen_restart_later"));
    }

    user_choice
}

pub fn restart_app() {
    if let Ok(current_exe) = env::current_exe() {
        if let Some(app_path) = current_exe
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            if Command::new("open").arg("-n").arg(app_path).spawn().is_ok() {
                info!("{}", t!("updater.app_restarted"));
                exit(0);
            }
        }
    }
    error!("{}", t!("updater.failed_to_restart"));
}
