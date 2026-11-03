use crate::config::Config;
use crate::launch::LaunchManager;
use crate::monitor::commands::MonitorCommand;
use crate::{
    clipboard, notification,
    status::{self, ServiceState, TunnelState},
    updater,
};
use log::{info, trace};
use rust_i18n::t;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
};
use winit::{application::ApplicationHandler, event_loop::EventLoop};

#[derive(Debug)]
pub enum UserEvent {
    TrayIconEvent(tray_icon::TrayIconEvent),
    MenuEvent(tray_icon::menu::MenuEvent),
    StatusChanged,
}

pub struct TrayApplication {
    tray_icon: Option<TrayIcon>,
    quit_requested: Arc<Mutex<bool>>,
    monitor_callback: Option<Box<dyn Fn() + Send>>,
    config: Arc<Mutex<Config>>,
    menu_items: Option<MenuItems>,
    monitor_sender: Sender<MonitorCommand>,
}

// 保存菜单项引用
#[derive(Clone)]
struct MenuItems {
    auto_paste: CheckMenuItem,
    auto_enter: CheckMenuItem,
    direct_input: CheckMenuItem,
    launch_at_login: CheckMenuItem,
    listen_email: CheckMenuItem,
    listen_message: CheckMenuItem,
    floating_window: CheckMenuItem,
    config: MenuItem,
    log: MenuItem,
    check_update: MenuItem,
    hide_tray: MenuItem,
    exit: MenuItem,
    webhook: Submenu,
    listen_webhook: CheckMenuItem,
    ngrok_enabled: CheckMenuItem,
    local_status: MenuItem,
    public_status: MenuItem,
    copy_public: MenuItem,
    copy_local: MenuItem,
    copy_token: MenuItem,
    regenerate_token: MenuItem,
    set_ngrok_token: MenuItem,
}

impl TrayApplication {
    pub fn new(
        quit_requested: Arc<Mutex<bool>>,
        config: Arc<Mutex<Config>>,
        monitor_callback: Option<Box<dyn Fn() + Send>>,
        monitor_sender: Sender<MonitorCommand>,
    ) -> Self {
        Self {
            tray_icon: None,
            quit_requested,
            config,
            monitor_callback,
            menu_items: None,
            monitor_sender,
        }
    }

    fn new_tray_icon(&mut self) -> Result<TrayIcon, Box<dyn std::error::Error>> {
        info!("{}", t!("tray.creating_tray_menu"));
        let menu = self.new_tray_menu()?;
        info!("{}", t!("tray.tray_menu_created"));

        info!("{}", t!("tray.finding_icon_path"));
        let icon_path =
            Self::find_icon_path().unwrap_or_else(|| PathBuf::from("resources").join("icon.png"));
        info!(
            "{}",
            t!(
                "tray.using_icon_path",
                icon_path = format!("{:?}", icon_path)
            )
        );

        info!("{}", t!("tray.loading_icon"));
        let icon = Self::load_icon(&icon_path)?;
        info!("{}", t!("tray.icon_loaded"));

        info!("{}", t!("tray.building_tray_icon"));
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_icon_as_template(true)
            .with_tooltip("Messauto")
            .build()?;
        info!("{}", t!("tray.tray_icon_built"));

        Ok(tray_icon)
    }

    fn find_icon_path() -> Option<PathBuf> {
        info!("{}", t!("tray.finding_icon_path"));
        let exe_path = env::current_exe().ok()?;
        let exe_dir = exe_path.parent()?;
        info!(
            "{}",
            t!(
                "tray.executable_directory",
                exe_dir = format!("{:?}", exe_dir)
            )
        );

        let possible_paths = [
            exe_dir.join("resources").join("icon.png"),
            PathBuf::from("resources").join("icon.png"),
            PathBuf::from("../resources").join("icon.png"),
            PathBuf::from("assets").join("images").join("icon.png"),
            exe_dir.join("assets").join("images").join("icon.png"),
        ];

        for path in &possible_paths {
            info!("{}", t!("tray.try_load_icon", path = format!("{:?}", path)));
            if path.exists() {
                info!(
                    "{}",
                    t!("tray.found_icon_file", path = format!("{:?}", path))
                );
                return Some(path.clone());
            }
        }

        info!("{}", t!("tray.icon_file_not_found"));
        None
    }

    fn load_icon(path: &Path) -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
        info!(
            "{}",
            t!("tray.loading_icon_from", path = format!("{:?}", path))
        );

        // Try to load from embedded resource first, fallback to file system
        let (icon_rgba, icon_width, icon_height) = {
            info!("{}", t!("tray.opening_image_file"));

            // Try embedded icon first
            let image = if path.ends_with("icon.png") {
                // Use embedded icon data
                let icon_data = include_bytes!("../resources/icon.png");
                image::load_from_memory(icon_data)?.into_rgba8()
            } else {
                // Fallback to file system for other icons
                if !path.exists() {
                    return Err(format!("Icon file does not exist: {:?}", path).into());
                }
                image::open(path)?.into_rgba8()
            };

            let (width, height) = image.dimensions();
            info!(
                "{}",
                t!("tray.image_dimensions", width = width, height = height)
            );
            let rgba = image.into_raw();
            info!("{}", t!("tray.image_data_length", length = rgba.len()));
            (rgba, width, height)
        };

        info!("{}", t!("tray.creating_tray_icon_from_rgba"));
        let icon = tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)?;
        info!("{}", t!("tray.icon_created"));

        Ok(icon)
    }

    fn new_tray_menu(&mut self) -> Result<Menu, Box<dyn std::error::Error>> {
        let menu = Menu::new();
        let config_guard = self.config.lock().unwrap();

        let menu_items = MenuItems {
            auto_paste: CheckMenuItem::new(
                &t!("menu.auto_paste"),
                true,
                config_guard.auto_paste,
                None,
            ),
            auto_enter: CheckMenuItem::new(
                &t!("menu.auto_enter"),
                true,
                config_guard.auto_enter,
                None,
            ),
            direct_input: CheckMenuItem::new(
                &t!("menu.direct_input"),
                true,
                config_guard.direct_input,
                None,
            ),
            launch_at_login: CheckMenuItem::new(
                &t!("menu.launch_at_login"),
                true,
                config_guard.launch_at_login,
                None,
            ),
            listen_email: CheckMenuItem::new(
                &t!("menu.listen_email"),
                true,
                config_guard.listen_email,
                None,
            ),
            listen_message: CheckMenuItem::new(
                &t!("menu.listen_message"),
                true,
                config_guard.listen_message,
                None,
            ),
            floating_window: CheckMenuItem::new(
                &t!("menu.floating_window"),
                true,
                config_guard.floating_window,
                None,
            ),
            config: MenuItem::new(&t!("menu.config"), true, None),
            log: MenuItem::new(&t!("menu.log"), true, None),
            check_update: MenuItem::new(&t!("menu.check_update"), true, None),
            hide_tray: MenuItem::new(&t!("menu.hide_tray"), true, None),
            exit: MenuItem::new(&t!("menu.exit"), true, None),
            webhook: Submenu::new(&t!("menu.webhook.title"), true),
            listen_webhook: CheckMenuItem::new(
                &t!("menu.webhook.enable"),
                true,
                config_guard.listen_webhook,
                None,
            ),
            ngrok_enabled: CheckMenuItem::new(
                &t!("menu.webhook.ngrok_enable"),
                true,
                config_guard.ngrok_enabled,
                None,
            ),
            local_status: MenuItem::new("", false, None),
            public_status: MenuItem::new("", false, None),
            copy_public: MenuItem::new(&t!("menu.webhook.copy_public"), false, None),
            copy_local: MenuItem::new(&t!("menu.webhook.copy_local"), false, None),
            copy_token: MenuItem::new(&t!("menu.webhook.copy_token"), true, None),
            regenerate_token: MenuItem::new(&t!("menu.webhook.regenerate_token"), true, None),
            set_ngrok_token: MenuItem::new(&t!("menu.webhook.set_ngrok_token"), true, None),
        };

        self.menu_items = Some(menu_items);

        let items_ref = self.menu_items.as_ref().unwrap();

        self.apply_menu_logic(items_ref, &config_guard);

        menu.append(&items_ref.auto_paste)?;
        menu.append(&items_ref.auto_enter)?;
        menu.append(&items_ref.direct_input)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&items_ref.listen_message)?;
        menu.append(&items_ref.listen_email)?;
        items_ref.webhook.append_items(&[
            &items_ref.listen_webhook,
            &items_ref.ngrok_enabled,
            &PredefinedMenuItem::separator(),
            &items_ref.local_status,
            &items_ref.public_status,
            &PredefinedMenuItem::separator(),
            &items_ref.copy_public,
            &items_ref.copy_local,
            &items_ref.copy_token,
            &items_ref.regenerate_token,
            &items_ref.set_ngrok_token,
        ])?;
        menu.append(&items_ref.webhook)?;
        self.refresh_webhook_menu();
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&items_ref.launch_at_login)?;
        menu.append(&items_ref.floating_window)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&items_ref.config)?;
        menu.append(&items_ref.log)?;
        menu.append(&items_ref.check_update)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&items_ref.hide_tray)?;
        menu.append(&items_ref.exit)?;

        Ok(menu)
    }

    fn refresh_webhook_menu(&self) {
        let Some(items) = &self.menu_items else {
            return;
        };
        let runtime = status::snapshot();
        let local = match &runtime.webhook {
            ServiceState::Stopped => t!("webhook.stopped").to_string(),
            ServiceState::Running { addr } => format!("{} · {}", addr, t!("webhook.running")),
            ServiceState::Failed(error) => t!("webhook.failed", error = error).to_string(),
        };
        let public = match &runtime.ngrok {
            TunnelState::Disabled => t!("ngrok.disabled").to_string(),
            TunnelState::NeedToken => t!("ngrok.need_token").to_string(),
            TunnelState::NeedDomain => t!("ngrok.need_domain").to_string(),
            TunnelState::Connecting => t!("ngrok.connecting").to_string(),
            TunnelState::Online { url } => url.clone(),
            TunnelState::Failed(error) => t!("webhook.failed", error = error).to_string(),
        };
        items
            .local_status
            .set_text(t!("menu.webhook.local_status", status = local));
        items
            .public_status
            .set_text(t!("menu.webhook.public_status", status = public));
        items
            .copy_local
            .set_enabled(matches!(runtime.webhook, ServiceState::Running { .. }));
        items
            .copy_public
            .set_enabled(matches!(runtime.ngrok, TunnelState::Online { .. }));
    }

    fn send_monitor_command(&self, command: MonitorCommand) {
        let sender = self.monitor_sender.clone();
        tokio::spawn(async move {
            if let Err(error) = sender.send(command).await {
                log::error!("Failed to send monitor command: {}", error);
            }
        });
    }

    fn copy_webhook_text(&self, text: &str) {
        match clipboard::copy_to_clipboard(text) {
            Ok(()) => notification::notify("MessAuto", &t!("webhook.copied")),
            Err(error) => notification::notify("MessAuto", &error),
        }
    }

    fn handle_webhook_menu(&self, id: &tray_icon::menu::MenuId) -> bool {
        let Some(items) = &self.menu_items else {
            return false;
        };
        if ![
            items.listen_webhook.id(),
            items.ngrok_enabled.id(),
            items.regenerate_token.id(),
            items.set_ngrok_token.id(),
            items.copy_token.id(),
            items.copy_public.id(),
            items.copy_local.id(),
        ]
        .contains(&id)
        {
            return false;
        }
        let mut config = self.config.lock().unwrap();
        // 保留用户在 config.toml 中修改的域名，再保存菜单开关。
        if let Ok(latest) = Config::load() {
            *config = latest;
        }
        let command = if id == items.listen_webhook.id() {
            config.listen_webhook = !config.listen_webhook;
            items.listen_webhook.set_checked(config.listen_webhook);
            Some(if config.listen_webhook {
                MonitorCommand::StartWebhook
            } else {
                MonitorCommand::StopWebhook
            })
        } else if id == items.ngrok_enabled.id() {
            config.ngrok_enabled = !config.ngrok_enabled;
            items.ngrok_enabled.set_checked(config.ngrok_enabled);
            Some(if config.ngrok_enabled {
                MonitorCommand::StartNgrok
            } else {
                MonitorCommand::StopNgrok
            })
        } else if id == items.regenerate_token.id() {
            if !notification::dialog(
                "MessAuto",
                &t!("webhook.regenerate_confirm"),
                &t!("webhook.confirm"),
                &t!("webhook.cancel"),
            ) {
                return true;
            }
            match crate::config::generate_token() {
                Ok(token) => config.webhook_token = token,
                Err(error) => {
                    notification::notify("MessAuto", &error.to_string());
                    return true;
                }
            }
            Some(MonitorCommand::RestartWebhook)
        } else if id == items.set_ngrok_token.id() {
            let Some(token) = notification::input_dialog("ngrok", &t!("ngrok.token_prompt"), true)
            else {
                return true;
            };
            config.ngrok_authtoken = token;
            Some(MonitorCommand::RestartNgrok)
        } else if id == items.copy_token.id() {
            self.copy_webhook_text(&config.webhook_token);
            return true;
        } else if id == items.copy_public.id() {
            if let TunnelState::Online { url } = status::snapshot().ngrok {
                self.copy_webhook_text(&webhook_url(&url, &config.webhook_token));
            }
            return true;
        } else if id == items.copy_local.id() {
            if let ServiceState::Running { addr } = status::snapshot().webhook {
                if let Some(ip) = lan_ip() {
                    let port = addr.parse::<std::net::SocketAddr>().unwrap().port();
                    self.copy_webhook_text(&webhook_url(
                        &format!("http://{ip}:{port}"),
                        &config.webhook_token,
                    ));
                } else {
                    notification::notify("MessAuto", &t!("webhook.no_lan_ip"));
                }
            }
            return true;
        } else {
            return false;
        };
        match config.save() {
            Ok(()) => {
                if let Some(command) = command {
                    self.send_monitor_command(command);
                }
            }
            Err(error) => notification::notify("MessAuto", &error.to_string()),
        }
        true
    }

    fn apply_menu_logic(&self, menu_items: &MenuItems, config: &Config) {
        if config.floating_window {
            menu_items.direct_input.set_enabled(false);
            menu_items.direct_input.set_checked(true);
            menu_items.auto_paste.set_enabled(false);
            menu_items.auto_paste.set_checked(false);
        } else if config.direct_input {
            menu_items.auto_paste.set_enabled(false);
            menu_items.auto_paste.set_checked(false);
            menu_items.direct_input.set_enabled(true);
        } else {
            menu_items.auto_paste.set_enabled(true);
            menu_items.direct_input.set_enabled(true);
        }
    }
}

impl ApplicationHandler<UserEvent> for TrayApplication {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }

    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        trace!("new_events called with cause: {:?}", cause);
        if winit::event::StartCause::Init == cause {
            info!("{}", t!("tray.creating_tray_icon"));
            match self.new_tray_icon() {
                Ok(icon) => {
                    info!("{}", t!("tray.tray_icon_created"));
                    self.tray_icon = Some(icon);
                }
                Err(err) => {
                    info!(
                        "{}",
                        t!(
                            "tray.failed_to_create_tray_icon",
                            err = format!("{:?}", err)
                        )
                    );
                    eprintln!("Failed to create tray icon: {:?}", err);
                }
            }

            if let Some(callback) = &self.monitor_callback {
                callback();
            }
        }
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::StatusChanged => self.refresh_webhook_menu(),
            UserEvent::TrayIconEvent(_event) => {
                // debug!("Tray event: {:?}", event);
            }
            UserEvent::MenuEvent(event) => {
                if self.handle_webhook_menu(&event.id) {
                    return;
                }
                if let Some(menu_items) = &self.menu_items {
                    let mut config = self.config.lock().unwrap();

                    if event.id == menu_items.auto_paste.id() {
                        config.auto_paste = !config.auto_paste;
                        menu_items.auto_paste.set_checked(config.auto_paste);
                        if let Err(e) = config.save() {
                            log::error!("{}", t!("config.failed_to_save_config", error = e));
                        }
                        info!(
                            "{}",
                            if config.auto_paste {
                                t!("config.auto_paste_enabled")
                            } else {
                                t!("config.auto_paste_disabled")
                            }
                        );
                    } else if event.id == menu_items.auto_enter.id() {
                        config.auto_enter = !config.auto_enter;
                        menu_items.auto_enter.set_checked(config.auto_enter);
                        if let Err(e) = config.save() {
                            log::error!("{}", t!("config.failed_to_save_config", error = e));
                        }
                        info!(
                            "{}",
                            if config.auto_enter {
                                t!("config.auto_enter_enabled")
                            } else {
                                t!("config.auto_enter_disabled")
                            }
                        );
                    } else if event.id == menu_items.direct_input.id() {
                        config.direct_input = !config.direct_input;
                        if config.direct_input {
                            config.auto_paste = false;
                        }
                        if let Err(e) = config.save() {
                            log::error!("{}", t!("config.failed_to_save_config", error = e));
                        }
                        info!(
                            "{}",
                            if config.direct_input {
                                t!("config.direct_input_enabled")
                            } else {
                                t!("config.direct_input_disabled")
                            }
                        );

                        self.apply_menu_logic(menu_items, &config);
                    } else if event.id == menu_items.launch_at_login.id() {
                        config.launch_at_login = !config.launch_at_login;
                        if let Err(e) = config.save() {
                            log::error!("{}", t!("config.failed_to_save_config", error = e));
                        }
                        info!(
                            "{}",
                            if config.launch_at_login {
                                t!("config.launch_at_login_enabled")
                            } else {
                                t!("config.launch_at_login_disabled")
                            }
                        );

                        if let Ok(launch_manager) = LaunchManager::new() {
                            if let Err(e) = launch_manager.sync_with_config(&config) {
                                log::error!("Failed to sync launch at login status: {}", e);
                            }
                        }
                    } else if event.id == menu_items.listen_email.id() {
                        config.listen_email = !config.listen_email;
                        menu_items.listen_email.set_checked(config.listen_email);
                        if let Err(e) = config.save() {
                            log::error!("{}", t!("config.failed_to_save_config", error = e));
                        }
                        info!(
                            "Listen email {}",
                            if config.listen_email {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );

                        let sender = self.monitor_sender.clone();
                        let enabled = config.listen_email;
                        tokio::spawn(async move {
                            let command = if enabled {
                                MonitorCommand::StartEmailMonitoring
                            } else {
                                MonitorCommand::StopEmailMonitoring
                            };
                            if let Err(e) = sender.send(command).await {
                                log::error!("Failed to send command to monitor actor: {}", e);
                            }
                        });
                    } else if event.id == menu_items.listen_message.id() {
                        config.listen_message = !config.listen_message;
                        menu_items.listen_message.set_checked(config.listen_message);
                        if let Err(e) = config.save() {
                            log::error!("{}", t!("config.failed_to_save_config", error = e));
                        }
                        info!(
                            "Listen message {}",
                            if config.listen_message {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );

                        let sender = self.monitor_sender.clone();
                        let enabled = config.listen_message;
                        tokio::spawn(async move {
                            let command = if enabled {
                                MonitorCommand::StartMessageMonitoring
                            } else {
                                MonitorCommand::StopMessageMonitoring
                            };
                            if let Err(e) = sender.send(command).await {
                                log::error!("Failed to send command to monitor actor: {}", e);
                            }
                        });
                    } else if event.id == menu_items.floating_window.id() {
                        config.floating_window = !config.floating_window;

                        if config.floating_window {
                            config.direct_input = true;
                            config.auto_paste = false;
                            menu_items.direct_input.set_checked(true);
                            menu_items.auto_paste.set_checked(false);
                        }

                        if let Err(e) = config.save() {
                            log::error!("{}", t!("config.failed_to_save_config", error = e));
                        }
                        info!(
                            "Floating window {}",
                            if config.floating_window {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );

                        self.apply_menu_logic(menu_items, &config);
                    } else if event.id == menu_items.config.id() {
                        let config_path = Config::get_config_path();
                        #[cfg(target_os = "macos")]
                        {
                            use std::process::Command;
                            if let Err(e) = Command::new("open").arg(&config_path).output() {
                                log::error!("Failed to open config file: {}", e);
                            } else {
                                info!("Opened config file: {:?}", config_path);
                            }
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            log::warn!("Config file opening is only supported on macOS");
                        }
                    } else if event.id == menu_items.log.id() {
                        #[cfg(target_os = "macos")]
                        {
                            use std::process::Command;
                            if let Err(e) = Command::new("open")
                                .arg(Config::get_log_file_path())
                                .output()
                            {
                                log::error!("Failed to open log file: {}", e);
                            } else {
                                info!("Opened log file");
                            }
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            log::warn!("Log file opening is only supported on macOS");
                        }
                    } else if event.id == menu_items.check_update.id() {
                        info!("用户触发检查更新");
                        updater::check_for_updates();
                    } else if event.id == menu_items.hide_tray.id() {
                        if let Some(ref mut tray_icon) = self.tray_icon {
                            if let Err(e) = tray_icon.set_visible(false) {
                                log::error!("{}", t!("tray.failed_to_hide_tray", error = e));
                            } else {
                                info!("{}", t!("tray.tray_hidden"));
                            }
                        }
                    } else if event.id == menu_items.exit.id() {
                        let mut quit = self.quit_requested.lock().unwrap();
                        *quit = true;
                        event_loop.exit();
                    }
                }
            }
        }
    }
}

pub fn run_tray_application(
    quit_requested: Arc<Mutex<bool>>,
    config: Arc<Mutex<Config>>,
    monitor_callback: Option<Box<dyn Fn() + Send>>,
    monitor_sender: Sender<MonitorCommand>,
) {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();

    status::set_notifier(event_loop.create_proxy());
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::TrayIconEvent(event));
    }));

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
    }));

    let mut app = TrayApplication::new(quit_requested, config, monitor_callback, monitor_sender);

    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("Error in event loop: {:?}", err);
    }
}

fn webhook_url(base: &str, token: &str) -> String {
    let mut url = url::Url::parse(base).unwrap();
    url.set_path("/webhook");
    url.query_pairs_mut().append_pair("token", token);
    url.to_string()
}

fn lan_ip() -> Option<String> {
    for interface in ["en0", "en1"] {
        let output = std::process::Command::new("ipconfig")
            .args(["getifaddr", interface])
            .output()
            .ok()?;
        if output.status.success() {
            let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !ip.is_empty() {
                return Some(ip);
            }
        }
    }
    None
}
