use crate::config::Config;
use crate::monitor::commands::MonitorCommand;
use log::{info, trace};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use tray_icon::menu::IconMenuItem;
use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
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
    hide_tray: MenuItem,
    exit: MenuItem,
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
        info!("Step 1: Creating tray menu...");
        let menu = self.new_tray_menu()?;
        info!("Step 1: Tray menu created successfully");

        info!("Step 2: Finding icon path...");
        let icon_path =
            Self::find_icon_path().unwrap_or_else(|| PathBuf::from("resources").join("icon.png"));
        info!("Step 2: Using icon path: {:?}", icon_path);

        info!("Step 3: Loading icon...");
        let icon = Self::load_icon(&icon_path)?;
        info!("Step 3: Icon loaded successfully");

        info!("Step 4: Building tray icon...");
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_icon_as_template(true)
            .with_tooltip("Messauto")
            .build()?;
        info!("Step 4: Tray icon built successfully");

        Ok(tray_icon)
    }

    fn find_icon_path() -> Option<PathBuf> {
        info!("Finding icon path...");
        let exe_path = env::current_exe().ok()?;
        let exe_dir = exe_path.parent()?;
        info!("Executable directory: {:?}", exe_dir);

        let possible_paths = [
            exe_dir.join("resources").join("icon.png"),
            PathBuf::from("resources").join("icon.png"),
            PathBuf::from("../resources").join("icon.png"),
            PathBuf::from("assets").join("images").join("icon.png"),
            exe_dir.join("assets").join("images").join("icon.png"),
        ];

        for path in &possible_paths {
            info!("尝试加载图标: {:?}", path);
            if path.exists() {
                info!("找到图标文件: {:?}", path);
                return Some(path.clone());
            }
        }

        info!("未找到图标文件，将使用默认路径");
        None
    }

    fn load_icon(path: &Path) -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
        info!("Loading icon from: {:?}", path);
        if !path.exists() {
            return Err(format!("Icon file does not exist: {:?}", path).into());
        }

        let (icon_rgba, icon_width, icon_height) = {
            info!("Opening image file...");
            let image = image::open(path)?.into_rgba8();
            let (width, height) = image.dimensions();
            info!("Image dimensions: {}x{}", width, height);
            let rgba = image.into_raw();
            info!("Image data length: {}", rgba.len());
            (rgba, width, height)
        };

        info!("Creating tray icon from RGBA data...");
        let icon = tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)?;
        info!("Icon created successfully");

        Ok(icon)
    }

    // 2. 修改 new_tray_menu 函数的实现和签名
    fn new_tray_menu(&mut self) -> Result<Menu, Box<dyn std::error::Error>> {
        let menu = Menu::new();
        let config_guard = self.config.lock().unwrap();

        // 3. 直接创建 MenuItems 实例，让它拥有所有菜单项对象
        let menu_items = MenuItems {
            auto_paste: CheckMenuItem::new("auto paste", true, config_guard.auto_paste, None),
            auto_enter: CheckMenuItem::new("auto enter", true, config_guard.auto_enter, None),
            direct_input: CheckMenuItem::new("direct input", true, config_guard.direct_input, None),
            launch_at_login: CheckMenuItem::new(
                "launch at login",
                true,
                config_guard.launch_at_login,
                None,
            ),
            listen_email: CheckMenuItem::new("listen email", true, config_guard.listen_email, None),
            listen_message: CheckMenuItem::new(
                "listen message",
                true,
                config_guard.listen_message,
                None,
            ),
            floating_window: CheckMenuItem::new(
                "floating window",
                true,
                config_guard.floating_window,
                None,
            ),
            config: MenuItem::new("config", true, None),
            log: MenuItem::new("log", true, None),
            hide_tray: MenuItem::new("hide tray", true, None),
            exit: MenuItem::new("exit", true, None),
        };

        // 4. 将所有权转移给 self，这样它们的生命周期就和 TrayApplication 实例绑定了
        self.menu_items = Some(menu_items);

        // 5. 从 self.menu_items 中获取不可变引用来构建菜单
        //    使用 .as_ref().unwrap() 是安全的，因为我们刚刚才存入了 Some(menu_items)
        let items_ref = self.menu_items.as_ref().unwrap();

        // 应用互斥逻辑
        self.apply_menu_logic(items_ref, &config_guard);

        // 使用 items_ref 中的引用来构建菜单
        menu.append(&items_ref.auto_paste)?;
        menu.append(&items_ref.auto_enter)?;
        menu.append(&items_ref.direct_input)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&items_ref.launch_at_login)?;
        menu.append(&items_ref.listen_email)?;
        menu.append(&items_ref.listen_message)?;
        menu.append(&items_ref.floating_window)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&items_ref.config)?;
        menu.append(&items_ref.log)?;
        menu.append(&items_ref.hide_tray)?;
        menu.append(&items_ref.exit)?;

        // 6. 只返回 Menu 对象
        Ok(menu)
    }

    fn apply_menu_logic(&self, menu_items: &MenuItems, config: &Config) {
        if config.floating_window {
            // 悬浮窗开启时强制启用直接输入，禁用剪贴板相关选项
            menu_items.direct_input.set_enabled(false);
            menu_items.direct_input.set_checked(true);
            menu_items.auto_paste.set_enabled(false);
            menu_items.auto_paste.set_checked(false);
        } else if config.direct_input {
            // 直接输入开启时禁用剪贴板相关选项
            menu_items.auto_paste.set_enabled(false);
            menu_items.auto_paste.set_checked(false);
            menu_items.direct_input.set_enabled(true);
        } else {
            // 普通模式：auto_paste 和 direct_input 选项可用
            menu_items.auto_paste.set_enabled(true);
            menu_items.direct_input.set_enabled(true);
        }
        // auto_enter 不受其他配置影响，始终保持可用状态
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
            info!("Creating tray icon...");
            match self.new_tray_icon() {
                Ok(icon) => {
                    info!("Tray icon created successfully");
                    self.tray_icon = Some(icon);
                }
                Err(err) => {
                    info!("Failed to create tray icon: {:?}", err);
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
            UserEvent::TrayIconEvent(_event) => {
                // debug!("Tray event: {:?}", event); // 注释掉，太吵闹了
            }
            UserEvent::MenuEvent(event) => {
                if let Some(menu_items) = &self.menu_items {
                    let mut config = self.config.lock().unwrap();

                    if event.id == menu_items.auto_paste.id() {
                        config.auto_paste = !config.auto_paste;
                        menu_items.auto_paste.set_checked(config.auto_paste);
                        if let Err(e) = config.save() {
                            log::error!("Failed to save config: {}", e);
                        }
                        info!(
                            "Auto paste {}",
                            if config.auto_paste {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );
                    } else if event.id == menu_items.auto_enter.id() {
                        config.auto_enter = !config.auto_enter;
                        menu_items.auto_enter.set_checked(config.auto_enter);
                        if let Err(e) = config.save() {
                            log::error!("Failed to save config: {}", e);
                        }
                        info!(
                            "Auto enter {}",
                            if config.auto_enter {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );
                    } else if event.id == menu_items.direct_input.id() {
                        config.direct_input = !config.direct_input;
                        if config.direct_input {
                            config.auto_paste = false;
                        }
                        if let Err(e) = config.save() {
                            log::error!("Failed to save config: {}", e);
                        }
                        info!(
                            "Direct input {}",
                            if config.direct_input {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );

                        // Re-apply menu logic after config change
                        self.apply_menu_logic(menu_items, &config);
                    } else if event.id == menu_items.launch_at_login.id() {
                        config.launch_at_login = !config.launch_at_login;
                        if let Err(e) = config.save() {
                            log::error!("Failed to save config: {}", e);
                        }
                        info!(
                            "Launch at login {}",
                            if config.launch_at_login {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );
                    } else if event.id == menu_items.listen_email.id() {
                        config.listen_email = !config.listen_email;
                        menu_items.listen_email.set_checked(config.listen_email);
                        if let Err(e) = config.save() {
                            log::error!("Failed to save config: {}", e);
                        }
                        info!(
                            "Listen email {}",
                            if config.listen_email {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );

                        // --- 发送命令给Actor ---
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
                            log::error!("Failed to save config: {}", e);
                        }
                        info!(
                            "Listen message {}",
                            if config.listen_message {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );

                        // --- 发送命令给Actor ---
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

                        // 悬浮窗开启时强制启用直接输入，禁用剪贴板相关选项
                        if config.floating_window {
                            config.direct_input = true;
                            config.auto_paste = false;
                            menu_items.direct_input.set_checked(true);
                            menu_items.auto_paste.set_checked(false);
                        }

                        if let Err(e) = config.save() {
                            log::error!("Failed to save config: {}", e);
                        }
                        info!(
                            "Floating window {}",
                            if config.floating_window {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );

                        // 重新应用菜单逻辑
                        self.apply_menu_logic(menu_items, &config);
                    } else if event.id == menu_items.config.id() {
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
                    } else if event.id == menu_items.log.id() {
                        // log menu item - open log file
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
                    } else if event.id == menu_items.hide_tray.id() {
                        println!("11");
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
