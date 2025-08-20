use crate::notification;
use cargo_packager_updater::{Config, Update, UpdaterBuilder, semver::Version};
use chrono::{DateTime, Utc};
use dirs;
use log::{error, info, warn};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, exit};
use std::thread;
use std::time::Duration;
use sysproxy::Sysproxy;

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

#[derive(Serialize, Deserialize)]
struct UpdateCheckState {
    last_check_time: String,
}

fn get_update_state_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_default()
        .join("messauto")
        .join("last_update_check.toml")
}

fn load_last_check_time() -> Option<DateTime<Utc>> {
    let path = get_update_state_path();
    if !path.exists() {
        return None;
    }

    match fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<UpdateCheckState>(&content) {
            Ok(state) => match DateTime::parse_from_rfc3339(&state.last_check_time) {
                Ok(dt) => Some(dt.with_timezone(&Utc)),
                Err(e) => {
                    warn!("Failed to parse last check time: {}", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to parse update state file: {}", e);
                None
            }
        },
        Err(e) => {
            warn!("Failed to read update state file: {}", e);
            None
        }
    }
}

fn save_last_check_time(time: DateTime<Utc>) {
    let path = get_update_state_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            warn!("Failed to create config directory: {}", e);
            return;
        }
    }

    let state = UpdateCheckState {
        last_check_time: time.to_rfc3339(),
    };

    match toml::to_string(&state) {
        Ok(content) => {
            if let Err(e) = fs::write(&path, content) {
                warn!("Failed to save update state: {}", e);
            }
        }
        Err(e) => {
            warn!("Failed to serialize update state: {}", e);
        }
    }
}

fn should_check_for_updates() -> bool {
    let now = Utc::now();
    let last_check = load_last_check_time();

    match last_check {
        Some(last) => {
            let duration_since_last = now - last;
            info!(
                "{}",
                t!(
                    "updater.duration_since_last_check",
                    hours = duration_since_last.num_hours()
                )
            );
            duration_since_last.num_hours() >= 24
        }
        None => true,
    }
}

fn get_system_proxy_url() -> Option<String> {
    match Sysproxy::get_system_proxy() {
        Ok(proxy) if proxy.enable => {
            let proxy_url = format!("http://{}:{}", proxy.host, proxy.port);
            info!(
                "{}",
                t!("updater.using_system_proxy", proxy_url = &proxy_url)
            );
            Some(proxy_url)
        }
        Ok(_) => {
            info!("{}", t!("updater.no_system_proxy_found"));
            None
        }
        Err(e) => {
            warn!(
                "{}",
                t!("updater.failed_to_get_proxy", error = e.to_string())
            );
            None
        }
    }
}

fn perform_update_check() {
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

    let proxy_url = get_system_proxy_url();

    let updater = match proxy_url {
        Some(url) => UpdaterBuilder::new(current_version, config)
            .proxy(&url)
            .build()
            .unwrap(),
        None => UpdaterBuilder::new(current_version, config)
            .build()
            .unwrap(),
    };

    match updater.check() {
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
}

pub fn check_for_updates() {
    thread::spawn(move || {
        perform_update_check();
    });
}

pub fn start_auto_update_checker() {
    thread::spawn(move || {
        loop {
            if should_check_for_updates() {
                info!("{}", t!("updater.auto_check_triggered"));
                perform_update_check();
                save_last_check_time(Utc::now());
            }
            thread::sleep(Duration::from_secs(300)); // 每5分钟检查一次
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
