use crate::notification;
use cargo_packager_updater::{Config, Update, check_update, semver::Version};
use log::{error, info};
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
    format!(
        "https://github.com/LeeeSe/MessAuto/releases/latest/download/update-{}.json",
        arch
    )
}

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

const PUB_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDY2NkEyQTU4M0Y3RTM3RkUKUldUK04zNC9XQ3BxWmhvQi84YkVYQUpOa2N5WWFDM2lhRXh5dDE0VE85SlRNejJ5VVJBR2JvYjEK";

pub fn check_for_updates() {
    thread::spawn(move || {
        info!("开始检查更新...");
        info!("当前架构: {}", get_current_arch());

        let current_version = match Version::parse(CURRENT_VERSION) {
            Ok(version) => version,
            Err(e) => {
                error!("解析当前版本失败: {}", e);
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
                info!("发现新版本: {}", update.version);
                match download_update(update) {
                    Ok((update_obj, update_bytes)) => {
                        info!("更新下载完成，准备安装");
                        if show_install_notification(&update_obj.version) {
                            if let Err(e) = install_update(update_obj, update_bytes) {
                                error!("安装更新失败: {}", e);
                            }
                        } else {
                            info!("用户取消安装更新");
                        }
                    }
                    Err(e) => {
                        error!("下载更新失败: {}", e);
                    }
                }
            }
            Ok(None) => {
                info!("应用已是最新版本。");
            }
            Err(e) => {
                error!("检查更新失败: {}", e);
            }
        }
    });
}

fn download_update(update: Update) -> Result<(Update, Vec<u8>), Box<dyn std::error::Error>> {
    info!("正在后台下载更新...");

    let update_bytes = update.download()?;
    info!("更新下载完成");

    Ok((update, update_bytes))
}

fn install_update(update: Update, update_bytes: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    info!("正在安装更新...");

    // 安装更新
    update.install(update_bytes)?;
    info!("更新安装成功，应用将重启");

    Ok(())
}

fn show_install_notification(version: &str) -> bool {
    info!("新版本 {} 已下载完成，询问用户是否安装", version);

    // 使用系统通知询问用户是否要安装更新
    let title = "MessAuto 更新可用";
    let content = format!(
        "新版本 {} 已下载完成。是否现在安装更新？\n\n安装后应用将自动重启。",
        version
    );

    let user_choice = notification::dialog(title, &content, "安装", "稍后");

    if user_choice {
        info!("用户选择安装更新");
    } else {
        info!("用户选择稍后安装更新");
    }

    user_choice
}
