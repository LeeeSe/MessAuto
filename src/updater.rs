use crate::notification;
use cargo_packager_updater::{Config, Update, check_update, semver::Version};
use log::{error, info};
use rust_i18n::t;
use std::env;
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
    info!("{}", t!("updater.endpoint_for_update", endpoint));
    endpoint
}

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

const PUB_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDY2NkEyQTU4M0Y3RTM3RkUKUldUK04zNC9XQ3BxWmhvQi84YkVYQUpOa2N5WWFDM2lhRXh5dDE0VE85SlRNejJ5VVJBR2JvYjEK";

pub fn check_for_updates() {
    thread::spawn(move || {
        info!("{}", t!("updater.checking_for_updates"));
        info!("{}", t!("updater.current_architecture", get_current_arch()));

        let current_version = match Version::parse(CURRENT_VERSION) {
            Ok(version) => version,
            Err(e) => {
                error!("{}", t!("updater.failed_to_parse_current_version", e));
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
                info!("{}", t!("updater.new_version_found", update.version));
                match download_update(update) {
                    Ok((update_obj, update_bytes)) => {
                        info!("{}", t!("updater.update_download_complete_ready_to_install"));
                        if show_install_notification(&update_obj.version) {
                            if let Err(e) = install_update(update_obj, update_bytes) {
                                error!("{}", t!("updater.failed_to_download_update", e));
                            }
                        } else {
                            info!("{}", t!("updater.user_cancelled_update_installation"));
                        }
                    }
                    Err(e) => {
                        error!("{}", t!("updater.failed_to_download_update", e));
                    }
                }
            }
            Ok(None) => {
                info!("{}", t!("updater.already_up_to_date"));
            }
            Err(e) => {
                error!("{}", t!("updater.failed_to_check_for_updates", e));
            }
        }
    });
}

fn download_update(update: Update) -> Result<(Update, Vec<u8>), Box<dyn std::error::Error>> {
    info!("{}", t!("updater.downloading_update_in_background"));

    let update_bytes = update.download()?;
    info!("{}", t!("updater.update_download_complete"));

    Ok((update, update_bytes))
}

fn install_update(update: Update, update_bytes: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    info!("{}", t!("updater.installing_update"));

    // 安装更新
    update.install(update_bytes)?;
    info!("{}", t!("updater.update_installed_successfully_app_will_restart"));

    Ok(())
}

fn show_install_notification(version: &str) -> bool {
    info!("{}", t!("updater.new_version_downloaded_ask_user_to_install", version));

    // 使用系统通知询问用户是否要安装更新
    let title = t!("updater.update_available_dialog_title");
    let content = t!("updater.update_available_dialog_content", version);

    let user_choice = notification::dialog(&title, &content, "安装", "稍后");

    if user_choice {
        info!("{}", t!("updater.user_chosen_to_install_update"));
    } else {
        info!("{}", t!("updater.user_chosen_to_install_update_later"));
    }

    user_choice
}
