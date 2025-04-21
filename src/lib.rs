use std::io::Read;
use std::thread::sleep;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use arboard::Clipboard;
use auto_launch::AutoLaunch;
use emlx::parse_emlx;
use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, StreamExt,
};
use home::home_dir;
use log::{error, info, warn};
use macos_accessibility_client::accessibility::{
    application_is_trusted, application_is_trusted_with_prompt,
};
use mail_parser::MessageParser;
use native_dialog::{MessageDialog, MessageType};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use osakit::{Language, Script};
use regex_lite::Regex;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    TrayIconBuilder,
};
pub const ARGS_APP: &str = "app";
rust_i18n::i18n!("locales");
pub fn get_sys_locale() -> &'static str {
    let syslocal = sys_locale::get_locale().unwrap();
    // 只取前两个字符并转换为&str
    let lang_code = &syslocal[0..2];
    match lang_code {
        "zh" => "zh-CN",
        "en" => "en",
        _ => "en",
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MAConfig {
    #[serde(default)]
    pub auto_paste: bool,
    #[serde(default)]
    pub auto_return: bool,
    #[serde(default)]
    pub hide_icon_forever: bool,
    #[serde(default)]
    pub launch_at_login: bool,
    #[serde(default = "default_flags")]
    pub flags: Vec<String>,
    #[serde(default)]
    pub listening_to_mail: bool,
    #[serde(default)]
    pub max_email_length: usize,
    #[serde(default)]
    pub float_window: bool,
    #[serde(default)]
    pub recover_clipboard: bool,
    #[serde(default)]
    pub use_ai: bool,
    #[serde(default)]
    pub ai_model: String,
    #[serde(default)]
    pub ai_api_key: String,
    #[serde(default)]
    pub ai_api_url: String,
}

fn default_flags() -> Vec<String> {
    vec![
        "验证码".to_string(),
        "动态密码".to_string(),
        "verification".to_string(),
        "code".to_string(),
        "Code".to_string(),
        "인증".to_string(),
        "代码".to_string(),
    ]
}

impl Default for MAConfig {
    fn default() -> Self {
        MAConfig {
            auto_paste: false,
            auto_return: false,
            hide_icon_forever: false,
            launch_at_login: false,
            flags: default_flags(),
            listening_to_mail: false,
            max_email_length: 500,
            float_window: false,
            recover_clipboard: false,
            use_ai: false,
            ai_model: "".to_string(),
            ai_api_key: "".to_string(),
            ai_api_url: "".to_string(),
        }
    }
}

impl MAConfig {
    // update the local config "~/.config/messauto/messauto.json"
    pub fn update(&self) -> Result<(), Box<dyn Error>> {
        let updated_config_str = serde_json::to_string(&self)?;
        fs::write(config_path(), updated_config_str)?;
        Ok(())
    }
}

pub fn config_path() -> PathBuf {
    let mut config_path = home_dir().unwrap();
    config_path.push(".config");
    config_path.push("messauto");
    config_path.push("messauto.json");
    config_path
}

pub fn log_path() -> PathBuf {
    let mut log_path = home_dir().unwrap();
    log_path.push(".config");
    log_path.push("messauto");
    log_path.push("messauto.log");
    if !log_path.exists() {
        fs::create_dir_all(log_path.parent().unwrap()).unwrap();
    }
    log_path
}

pub fn read_config() -> MAConfig {
    if !config_path().exists() {
        let config = MAConfig::default();
        let config_str = serde_json::to_string(&config).unwrap();
        fs::create_dir_all(config_path().parent().unwrap()).unwrap();
        fs::write(config_path(), config_str).unwrap();
    }
    let config_str = fs::read_to_string(config_path()).unwrap();
    let config: MAConfig = serde_json::from_str(&config_str).unwrap();
    config.update().unwrap();
    config
}

pub struct TrayMenuItems {
    pub quit_i: MenuItem,
    pub check_auto_paste: CheckMenuItem,
    pub check_auto_return: CheckMenuItem,
    pub check_hide_icon_for_now: MenuItem,
    pub check_hide_icon_forever: MenuItem,
    pub check_launch_at_login: CheckMenuItem,
    pub add_flag: MenuItem,
    pub maconfig: MenuItem,
    pub logs: MenuItem,
    pub listening_to_mail: CheckMenuItem,
    pub float_window: CheckMenuItem,
    pub recover_clipboard: CheckMenuItem,
}

impl TrayMenuItems {
    pub fn build(config: &MAConfig) -> Self {
        let quit_i = MenuItem::new(t!("quit"), true, None);
        let check_auto_paste = CheckMenuItem::new(t!("auto-paste"), true, config.auto_paste, None);
        let check_auto_return = CheckMenuItem::new(
            t!("auto-return"),
            config.auto_paste,
            config.auto_return,
            None,
        );
        let check_hide_icon_for_now = MenuItem::new(t!("hide-icon-for-now"), true, None);

        let check_hide_icon_forever = MenuItem::new(t!("hide-icon-forever"), true, None);

        let check_launch_at_login =
            CheckMenuItem::new(t!("launch-at-login"), true, config.launch_at_login, None);

        let add_flag = MenuItem::new(t!("add-flag"), true, None);

        let maconfig = MenuItem::new(t!("config"), true, None);

        let logs = MenuItem::new(t!("logs"), true, None);

        let listening_to_mail = CheckMenuItem::new(
            t!("listening-to-mail"),
            true,
            config.listening_to_mail,
            None,
        );

        let float_window = CheckMenuItem::new(t!("float-window"), true, config.float_window, None);

        let recover_clipboard = CheckMenuItem::new(
            t!("recover-clipboard"),
            true,
            config.recover_clipboard,
            None,
        );

        TrayMenuItems {
            quit_i,
            check_auto_paste,
            check_auto_return,
            check_hide_icon_for_now,
            check_hide_icon_forever,
            check_launch_at_login,
            add_flag,
            listening_to_mail,
            float_window,
            maconfig,
            logs,
            recover_clipboard,
        }
    }
}

pub struct TrayMenu {}

impl TrayMenu {
    pub fn build(tray_menu_items: &TrayMenuItems) -> Menu {
        let tray_menu = Menu::new();
        let _ = tray_menu.append_items(&[
            &tray_menu_items.check_auto_paste,
            &tray_menu_items.check_auto_return,
            &tray_menu_items.recover_clipboard,
            &PredefinedMenuItem::separator(),
            &Submenu::with_items(
                t!("hide-icon"),
                true,
                &[
                    &tray_menu_items.check_hide_icon_for_now,
                    &tray_menu_items.check_hide_icon_forever,
                ],
            )
            .expect("create submenu failed"),
            &tray_menu_items.check_launch_at_login,
            &PredefinedMenuItem::separator(),
            // &tray_menu_items.add_flag,
            &tray_menu_items.listening_to_mail,
            &tray_menu_items.float_window,
            &PredefinedMenuItem::separator(),
            &tray_menu_items.maconfig,
            &tray_menu_items.logs,
            &PredefinedMenuItem::separator(),
            &tray_menu_items.quit_i,
        ]);
        tray_menu
    }
}

pub struct TrayIcon {}

impl TrayIcon {
    pub fn build(tray_menu: Menu) -> Option<tray_icon::TrayIcon> {
        let bin_path = get_current_exe_path();
        let mut icon_path = bin_path.join("Contents/Resources/assets/images/icon.png");
        if !icon_path.exists() {
            icon_path = "assets/images/icon.png".into();
        }
        let icon = load_icon(Path::new(&icon_path));
        Some(
            TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                // .with_title("📨")
                .with_icon(icon)
                .build()
                .unwrap(),
        )
    }
}

fn load_icon(path: &Path) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

pub fn auto_launch() -> AutoLaunch {
    let app_name = env!("CARGO_PKG_NAME");
    let app_path = get_current_exe_path();
    let args = &["--minimized", "--hidden"];
    AutoLaunch::new(app_name, app_path.to_str().unwrap(), false, args)
}

pub fn check_full_disk_access() {
    // 试图访问敏感文件来触发权限请求
    let check_db_path = home_dir()
        .expect("获取用户目录失败")
        .join("Library/Messages");
    let ct = fs::read_dir(check_db_path);
    if ct.is_err() {
        warn!("{}", t!("access-blocked-no-full-disk-access"));
        let yes = MessageDialog::new()
            .set_type(MessageType::Info)
            .set_title(t!("full-disk-access").to_string().as_str())
            .show_confirm()
            .unwrap();
        if yes {
            let output = Command::new("sh")
                .arg("-c")
                .arg(
                    "open x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles",
                )
                .output();
            if output.is_err() {
                error!("{}", t!("error-when-open-authorization-window"));
            } else {
                warn!("{}", t!("popup-authorization-window"));
            }
        }
        // panic!("exit without full disk access");
    } else {
        info!("{}", t!("successfully-obtained-disk-access-permissions"));
    }
}

pub fn check_script_permissions() -> bool {
    let mut script = Script::new_from_source(
        Language::AppleScript,
        "
        tell application \"System Events\"
	key code 58
        end tell
    ",
    );
    script.compile().unwrap();
    let result = script.execute();
    if result.is_ok() {
        return true;
    } else {
        return false;
    }
}

pub fn check_accessibility() -> bool {
    if application_is_trusted_with_prompt() && check_script_permissions() {
        return true;
    }
    false
}

pub fn check_accessibility_with_no_action() -> bool {
    application_is_trusted()
}

// 检查最新信息是否是验证码类型,并返回关键词来辅助定位验证码
pub fn check_captcha_or_other<'a>(stdout: &'a str, flags: &'a Vec<String>) -> bool {
    for flag in flags {
        if stdout.contains(flag) {
            return true;
        }
    }
    false
}
// 利用 AI 从信息中提取验证码
pub fn get_captchas_with_ai(stdout: &str) -> Vec<String> {
    let config = read_config();
    let mut captcha_vec = Vec::new();
    let url = format!("{}/v1/chat/completions", config.ai_api_url);
    let body = json!({
        "model": config.ai_model,
        "messages": [
            {
                "role": "user",
                "content": format!("从以下信息中提取验证码，不要输出任何额外信息: {}", stdout)
            }
        ]
    });
    let response = reqwest::blocking::Client::new().post(url).json(&body).send().unwrap();
    let response_body = response.json::<serde_json::Value>().unwrap();
    let content = response_body["choices"][0]["message"]["content"].as_str().unwrap();
    captcha_vec.push(content.to_string());
    captcha_vec
}
   

// 利用正则表达式从信息中提取验证码
pub fn get_captchas(stdout: &str) -> Vec<String> {
    // let re = Regex::new(r"\b[a-zA-Z0-9]{4,8}\b").unwrap(); // 只提取4-8位数字与字母组合
    let re = Regex::new(r"\b[a-zA-Z0-9][a-zA-Z0-9-]{2,6}[a-zA-Z0-9]\b").unwrap();
    let stdout_str = stdout;
    let mut captcha_vec = Vec::new();
    for m in re.find_iter(stdout_str) {
        for i in m.as_str().chars() {
            if i.is_ascii_digit() {
                captcha_vec.push(m.as_str().to_string());
                break;
            }
        }
    }
    captcha_vec
}

// 如果检测到 chat.db 有变动，则提取最近一分钟内最新的一条信息
pub fn get_message_in_one_minute() -> String {
    let output = Command::new("sqlite3")
        .arg(home_dir().expect("获取用户目录失败").join("Library/Messages/chat.db"))
        .arg("SELECT text FROM message WHERE datetime(date/1000000000 + 978307200,\"unixepoch\",\"localtime\") > datetime(\"now\",\"localtime\",\"-60 second\") ORDER BY date DESC LIMIT 1;")
        .output()
        .expect("sqlite命令运行失败");

    String::from_utf8(output.stdout).unwrap()
}

// 如果信息中包含多个4-8位数字与字母组合（比如公司名称和验证码都是4-8位英文数字组合，例如CSDN）
// 则选取数字字符个数最多的的那个字串作为验证码
pub fn get_real_captcha(stdout: &str) -> String {
    let captchas = get_captchas_with_ai(stdout);
    let mut real_captcha = String::new();
    let mut max_digit_count = 0;
    for captcha in captchas {
        let mut digit_count = 0;
        for i in captcha.chars() {
            if i.is_ascii_digit() {
                digit_count += 1;
            }
        }
        if digit_count > max_digit_count {
            max_digit_count = digit_count;
            real_captcha = captcha;
        }
    }
    real_captcha
}

pub fn paste_script() -> Result<(), Box<dyn Error>> {
    let mut script = Script::new_from_source(
        Language::AppleScript,
        "
        tell application \"System Events\"
	keystroke \"v\" using command down
        end tell
    ",
    );
    script.compile()?;
    script.execute()?;

    Ok(())
}

pub fn return_script() -> Result<(), Box<dyn Error>> {
    let mut script = Script::new_from_source(
        Language::AppleScript,
        "
        tell application \"System Events\"
	key code 36
        end tell
    ",
    );
    script.compile()?;
    script.execute()?;

    Ok(())
}

pub fn messages_thread() {
    thread::spawn(move || {
        let config = read_config();
        let flags = config.flags;
        let check_db_path = home_dir().unwrap().join("Library/Messages/chat.db-wal");
        let mut last_metadata_modified = fs::metadata(&check_db_path).unwrap().modified().unwrap();
        loop {
            let now_metadata = fs::metadata(&check_db_path).unwrap().modified().unwrap();
            if now_metadata != last_metadata_modified {
                last_metadata_modified = now_metadata;
                let stdout = get_message_in_one_minute();
                let captcha_or_other = check_captcha_or_other(&stdout, &flags);
                if captcha_or_other {
                    info!("{}", t!("new-verification-code-detected"));
                    let captchas = match config.use_ai {
                        true => get_captchas_with_ai(&stdout),
                        false => get_captchas(&stdout),
                    };
                    info!("{}:{:?}", t!("all-possible-codes"), captchas);
                    let real_captcha = get_real_captcha(&stdout);
                    info!("{}:{:?}", t!("real-verification-code"), real_captcha);
                    let mut ctx = Clipboard::new().unwrap();
                    let old_clipboard_contents = get_old_clipboard_contents();

                    let config = read_config();
                    if config.float_window {
                        let _child = open_app(real_captcha, t!("imessage").to_string());
                    } else if !config.float_window {
                        ctx.set_text(&real_captcha).unwrap();
                        if config.auto_paste {
                            match paste_script() {
                                Ok(_) => info!("{}", t!("paste-verification-code")),
                                Err(e) => {
                                    error!("{}: {:?}", t!("error-paste-verification-code"), e)
                                }
                            }
                        }
                        if config.auto_return {
                            match return_script() {
                                Ok(_) => info!("{}", t!("press-enter")),
                                Err(e) => error!("{}: {:?}", t!("error-press-enter"), e),
                            }
                        }
                        if config.recover_clipboard {
                            sleep(Duration::from_secs(2)); // wait applescript to finish
                            recover_clipboard_contents(old_clipboard_contents);
                        }
                    }
                }
                sleep(Duration::from_secs(5));
            }
            sleep(Duration::from_secs(1)); // check db change every second
        }
    });
}

pub fn get_current_exe_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    if path.to_str().unwrap().contains(".app") {
        path = path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
    }
    path
}

pub fn check_for_updates() -> Result<bool, Box<dyn Error>> {
    // 通过运行curl命令获取最新版本号
    let output = Command::new("curl")
        .arg("https://api.github.com/repos/LeeeSe/MessAuto/releases/latest")
        .arg("--max-time")
        .arg("10")
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    // 解析json
    let v: serde_json::Value = serde_json::from_str(&stdout)?;
    let latest_version = v["tag_name"].as_str();
    if latest_version.is_none() {
        return Err("Tag_name not found".into());
    }
    // 获取当前二进制文件的版本号
    let current_version = env!("CARGO_PKG_VERSION");
    // 格式化两个版本号,将字符串中的非数字字符去掉,并转换为数字
    let latest_version = latest_version
        .unwrap()
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    let current_version = current_version
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    // 转换为数字
    let latest_version = latest_version.parse::<i32>()?;
    let current_version = current_version.parse::<i32>()?;
    info!("{}: {}", t!("latest-version-number"), latest_version);
    info!("{}: {}", t!("current-version-number"), current_version);
    // 如果最新版本号大于当前版本号,则提示更新
    if latest_version > current_version {
        return Ok(true);
    }
    Ok(false)
}

pub fn download_latest_release() -> Result<(), Box<dyn Error>> {
    // 通过运行curl命令获取最新版本号
    let output = Command::new("curl")
        .arg("https://api.github.com/repos/LeeeSe/MessAuto/releases/latest")
        .arg("--max-time")
        .arg("10")
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    // 解析json
    let v: serde_json::Value = serde_json::from_str(&stdout)?;
    let latest_version = v["tag_name"].as_str();
    if latest_version.is_none() {
        return Err("Tag_name not found".into());
    }
    // 检查本机为arm还是x86
    let arch = std::env::consts::ARCH;
    // 根据本机架构选择下载链接
    match arch {
        "x86_64" => {
            let download_url = format!(
                "https://github.com/LeeeSe/MessAuto/releases/download/{}/MessAuto_x86_64.zip",
                latest_version.unwrap()
            );
            let _ = Command::new("curl")
                .arg(download_url)
                .arg("--max-time")
                .arg("10")
                .arg("-L")
                .arg("-f")
                .arg("-o")
                .arg("/tmp/MessAuto.zip")
                .output()?;
        }
        "aarch64" => {
            let download_url = format!(
                "https://github.com/LeeeSe/MessAuto/releases/download/{}/MessAuto_aarch64.zip",
                latest_version.unwrap()
            );
            let _ = Command::new("curl")
                .arg(download_url)
                .arg("--max-time")
                .arg("10")
                .arg("-L")
                .arg("-f")
                .arg("-o")
                .arg("/tmp/MessAuto.zip")
                .output()?;
        }
        _ => {
            error!("{}", t!("unsupported-platform"));
        }
    }
    if !Path::new("/tmp/MessAuto.zip").exists() {
        warn!("{}", t!("new-version-download-failed"));
        return Err("Download failed".into());
    } else {
        info!("{}", t!("new-version-download-success"));
    }
    Ok(())
}

pub fn update_thread(tx: std::sync::mpsc::Sender<bool>) {
    thread::spawn(move || {
        if check_for_updates().is_ok() {
            if check_for_updates().unwrap() {
                info!("{}", t!("detected-new-version"));
                if download_latest_release().is_ok() {
                    tx.send(true).unwrap();
                }
            } else {
                info!("{}", t!("version-up-to-date"));
            }
        } else {
            warn!("{}", t!("update-check-failed-ensure-network-access"));
        }
    });
}

// 将下载好的新版本替换老版本
pub fn replace_old_version() -> Result<(), Box<dyn Error>> {
    let unzip_output = Command::new("unzip")
        .arg("-o")
        .arg("/tmp/MessAuto.zip")
        .arg("-d")
        .arg("/tmp/")
        .output()?;
    info!("{}: {:?}", t!("unzip-operation"), unzip_output);

    let _ = Command::new("rm").arg("/tmp/MessAuto.zip").output()?;

    let mv_output = Command::new("cp")
        .arg("-R")
        .arg("/tmp/MessAuto.app")
        .arg(get_current_exe_path().parent().unwrap())
        .output()?;
    info!("{}: {:?}", t!("replace-binary-file"), mv_output);
    Ok(())
}

pub fn mail_thread() {
    thread::spawn(move || {
        let mail_path = home_dir().unwrap().join("Library/Mail");
        let path = String::from(mail_path.to_str().unwrap());

        futures::executor::block_on(async {
            if let Err(e) = async_watch(path).await {
                error!("error: {:?}", e)
            }
        });
    });
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

async fn async_watch<P: AsRef<Path>>(path: P) -> notify::Result<()> {
    let (mut watcher, mut rx) = async_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => {
                if let notify::event::EventKind::Create(_) = event.kind {
                    for path in event.paths {
                        let path = path.to_string_lossy();
                        if path.contains(".emlx") && path.contains("INBOX.mbox") {
                            async_std::task::sleep(Duration::from_secs(1)).await; // prevent repeated reading
                            info!("{}: {:?}", t!("new-email-received"), path);
                            let path = path.replace(".tmp", "");
                            let content = read_emlx(&path);
                            info!("len: {}", content.len());
                            let config = read_config();
                            let flags = config.flags;
                            // Protect user privacy
                            // info!("{}", t!("email-content"));

                            if content.len() < config.max_email_length {
                                let is_captcha = check_captcha_or_other(&content, &flags);
                                if is_captcha {
                                    info!("{}", t!("new-verification-email-detected"));
                                    let captchas = match config.use_ai {
                                        true => get_captchas_with_ai(&content),
                                        false => get_captchas(&content),
                                    };
                                    info!("{}:{:?}", t!("all-possible-codes"), captchas);
                                    let real_captcha = get_real_captcha(&content);
                                    info!("{}:{:?}", t!("real-verification-code"), real_captcha);
                                    let mut clpb = Clipboard::new().unwrap();

                                    let old_clpb_contents = get_old_clipboard_contents();

                                    let config = read_config();
                                    if config.float_window {
                                        let _child = open_app(real_captcha, t!("mail").to_string());
                                    } else if config.auto_paste {
                                        clpb.set_text(&real_captcha).unwrap();
                                        match paste_script() {
                                            Ok(_) => info!("{}", t!("paste-verification-code")),
                                            Err(e) => error!(
                                                "{}: {:?}",
                                                t!("error-paste-verification-code"),
                                                e
                                            ),
                                        }
                                        if config.auto_return {
                                            match return_script() {
                                                Ok(_) => info!("{}", t!("press-enter")),
                                                Err(e) => {
                                                    error!("{}: {:?}", t!("error-press-enter"), e)
                                                }
                                            }
                                        }
                                        if config.recover_clipboard {
                                            async_std::task::sleep(Duration::from_secs(2)).await; //wait for pasted
                                            recover_clipboard_contents(old_clpb_contents);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => error!("watch error: {:?}", e),
        }
    }
    Ok(())
}

fn read_emlx(path: &str) -> String {
    let mut file = fs::File::open(path).unwrap();
    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer).unwrap();

    let parsed = parse_emlx(&buffer).unwrap();

    let message = std::str::from_utf8(parsed.message).unwrap();
    let message = MessageParser::default().parse(message).unwrap();

    message.body_text(0).unwrap().clone().to_string()
}

pub fn open_app(code: String, from_app: String) -> std::process::Child {
    start_process(vec![ARGS_APP.to_string(), code, from_app])
}

fn start_process(command_args: Vec<String>) -> std::process::Child {
    let current_exe = std::env::current_exe().unwrap();

    let child = Command::new(current_exe)
        .args(&command_args)
        .spawn()
        .unwrap();
    child
}

pub fn sleep_key() {
    sleep(Duration::from_millis(60));
}

pub fn get_old_clipboard_contents() -> (
    Result<String, arboard::Error>,
    Result<arboard::ImageData<'static>, arboard::Error>,
) {
    let mut clpb = Clipboard::new().unwrap();
    let string = clpb.get_text();
    let image = clpb.get_image();

    (string, image)
}

pub fn recover_clipboard_contents(
    contents: (
        Result<String, arboard::Error>,
        Result<arboard::ImageData<'_>, arboard::Error>,
    ),
) {
    let mut clpb = Clipboard::new().unwrap();
    if contents.0.is_ok() {
        let _ = clpb.set_text(contents.0.as_ref().unwrap());
        info!(
            "{}:{:?}",
            t!("old-clpb-contents"),
            contents.0.as_ref().unwrap()
        );
    } else if contents.1.is_ok() {
        let _ = clpb.set_image(contents.1.unwrap());
        info!("{}:{:?}", t!("old-clpb-contents"), "is an image");
    } else {
        warn!("{}", t!("unable-to-recover-clipboard"));
    }
}
