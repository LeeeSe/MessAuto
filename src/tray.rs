use crate::config::Config;
use log::{debug, info};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use winit::{application::ApplicationHandler, event_loop::EventLoop};

#[derive(Debug)]
pub enum UserEvent {
    TrayIconEvent(tray_icon::TrayIconEvent),
    MenuEvent(tray_icon::menu::MenuEvent),
}

pub struct TrayApplication {
    tray_icon: Option<TrayIcon>,
    quit_requested: Arc<Mutex<bool>>,
    monitor_callback: Option<Box<dyn Fn() + Send>>,
    config: Arc<Mutex<Config>>,
}

impl TrayApplication {
    pub fn new(
        quit_requested: Arc<Mutex<bool>>,
        config: Arc<Mutex<Config>>,
        monitor_callback: Option<Box<dyn Fn() + Send>>,
    ) -> Self {
        Self {
            tray_icon: None,
            quit_requested,
            config,
            monitor_callback,
        }
    }

    fn new_tray_icon(&self) -> Result<TrayIcon, Box<dyn std::error::Error>> {
        let menu = self.new_tray_menu()?;

        let icon_path = Self::find_icon_path()
            .unwrap_or_else(|| PathBuf::from("resources").join("icon_256.png"));

        let icon = Self::load_icon(&icon_path)?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_icon_as_template(true)
            .build()?;

        Ok(tray_icon)
    }

    fn find_icon_path() -> Option<PathBuf> {
        let exe_path = env::current_exe().ok()?;
        let exe_dir = exe_path.parent()?;

        let possible_paths = [
            exe_dir.join("resources").join("icon.png"),
            PathBuf::from("resources").join("icon.png"),
            PathBuf::from("../resources").join("icon.png"),
            PathBuf::from("assets").join("images").join("icon.png"),
            exe_dir.join("assets").join("images").join("icon.png"),
        ];

        for path in &possible_paths {
            println!("尝试加载图标: {:?}", path);
            if path.exists() {
                return Some(path.clone());
            }
        }

        None
    }

    fn load_icon(path: &Path) -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
        let (icon_rgba, icon_width, icon_height) = {
            let image = image::open(path)?.into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            (rgba, width, height)
        };

        Ok(tray_icon::Icon::from_rgba(
            icon_rgba,
            icon_width,
            icon_height,
        )?)
    }

    fn new_tray_menu(&self) -> Result<Menu, Box<dyn std::error::Error>> {
        let menu = Menu::new();

        let config_guard = self.config.lock().unwrap();

        // Add check menu items with current config state
        let check_items = [
            CheckMenuItem::new("auto copy", true, config_guard.auto_copy, None),
            CheckMenuItem::new("auto paste", true, config_guard.auto_paste, None),
            CheckMenuItem::new("restore clipboard", true, config_guard.restore_clipboard, None),
            CheckMenuItem::new("launch at login", true, config_guard.launch_at_login, None),
            CheckMenuItem::new("listen email", true, config_guard.listen_email, None),
            CheckMenuItem::new("floating window", true, config_guard.floating_window, None),
        ];

        // Add regular menu items
        let regular_items = [
            MenuItem::new("config", true, None),
            MenuItem::new("log", true, None),
            MenuItem::new("hide tray", true, None),
            MenuItem::new("exit", true, None),
        ];

        // Append all check menu items
        for item in &check_items {
            menu.append(item)?;
        }

        // Append all regular menu items
        for item in &regular_items {
            menu.append(item)?;
        }

        menu.insert_items(&[&PredefinedMenuItem::separator()], 3)?;
        menu.insert_items(&[&PredefinedMenuItem::separator()], 7)?;
        menu.insert_items(&[&PredefinedMenuItem::separator()], 10)?;

        Ok(menu)
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
        if winit::event::StartCause::Init == cause {
            match self.new_tray_icon() {
                Ok(icon) => self.tray_icon = Some(icon),
                Err(err) => eprintln!("Failed to create tray icon: {:?}", err),
            }

            if let Some(callback) = &self.monitor_callback {
                callback();
            }
        }
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::TrayIconEvent(event) => {
                debug!("Tray event: {:?}", event);
            }
            UserEvent::MenuEvent(event) => {
                println!("Menu event received");
                println!("{:?}", event);
                if event.id == "3" {
                    let mut config = self.config.lock().unwrap();
                    config.auto_copy = !config.auto_copy;
                    if let Err(e) = config.save() {
                        log::error!("Failed to save config: {}", e);
                    }
                    info!("Auto copy {}", if config.auto_copy { "enabled" } else { "disabled" });
                } else if event.id == "4" {
                    // auto paste
                    let mut config = self.config.lock().unwrap();
                    config.auto_paste = !config.auto_paste;
                    if let Err(e) = config.save() {
                        log::error!("Failed to save config: {}", e);
                    }
                    info!("Auto paste {}", if config.auto_paste { "enabled" } else { "disabled" });
                } else if event.id == "5" {
                    // restore clipboard
                    let mut config = self.config.lock().unwrap();
                    config.restore_clipboard = !config.restore_clipboard;
                    if let Err(e) = config.save() {
                        log::error!("Failed to save config: {}", e);
                    }
                    info!("Restore clipboard {}", if config.restore_clipboard { "enabled" } else { "disabled" });
                } else if event.id == "6" {
                    // launch at login
                    let mut config = self.config.lock().unwrap();
                    config.launch_at_login = !config.launch_at_login;
                    if let Err(e) = config.save() {
                        log::error!("Failed to save config: {}", e);
                    }
                    info!("Launch at login {}", if config.launch_at_login { "enabled" } else { "disabled" });
                } else if event.id == "7" {
                    // listen email
                    let mut config = self.config.lock().unwrap();
                    config.listen_email = !config.listen_email;
                    if let Err(e) = config.save() {
                        log::error!("Failed to save config: {}", e);
                    }
                    info!("Listen email {}", if config.listen_email { "enabled" } else { "disabled" });
                } else if event.id == "8" {
                    // floating window
                    let mut config = self.config.lock().unwrap();
                    config.floating_window = !config.floating_window;
                    if let Err(e) = config.save() {
                        log::error!("Failed to save config: {}", e);
                    }
                    info!("Floating window {}", if config.floating_window { "enabled" } else { "disabled" });
                } else if event.id == "9" {
                    // config menu item - open config file
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
                } else if event.id == "10" {
                    // log menu item - open log file
                    #[cfg(target_os = "macos")]
                    {
                        use std::process::Command;
                        if let Err(e) = Command::new("open").arg(Config::get_log_file_path()).output() {
                            log::error!("Failed to open log file: {}", e);
                        } else {
                            info!("Opened log file");
                        }
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        log::warn!("Log file opening is only supported on macOS");
                    }
                } else if event.id == "11" {
                    println!("11");
                } else if event.id == "12" {
                    let mut quit = self.quit_requested.lock().unwrap();
                    *quit = true;
                    event_loop.exit();
                }
            }
        }
    }
}

pub fn run_tray_application(
    quit_requested: Arc<Mutex<bool>>,
    config: Arc<Mutex<Config>>,
    monitor_callback: Option<Box<dyn Fn() + Send>>,
) {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();

    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::TrayIconEvent(event));
    }));

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
    }));

    let mut app = TrayApplication::new(quit_requested, config, monitor_callback);

    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("Error in event loop: {:?}", err);
    }
}
