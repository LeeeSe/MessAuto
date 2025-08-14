use log::debug;
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
}

impl TrayApplication {
    pub fn new(
        quit_requested: Arc<Mutex<bool>>,
        monitor_callback: Option<Box<dyn Fn() + Send>>,
    ) -> Self {
        Self {
            tray_icon: None,
            quit_requested,
            monitor_callback,
        }
    }

    fn new_tray_icon() -> Result<TrayIcon, Box<dyn std::error::Error>> {
        let menu = Self::new_tray_menu()?;

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

    fn new_tray_menu() -> Result<Menu, Box<dyn std::error::Error>> {
        let menu = Menu::new();

        // Add check menu items
        let check_items = [
            CheckMenuItem::new("auto copy", true, false, None),
            CheckMenuItem::new("auto paste", true, false, None),
            CheckMenuItem::new("restore clipboard", true, false, None),
            CheckMenuItem::new("launch at login", true, false, None),
            CheckMenuItem::new("listen email", true, false, None),
            CheckMenuItem::new("floating window", true, false, None),
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
            match Self::new_tray_icon() {
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
                    println!("3");
                } else if event.id == "4" {
                    println!("4")
                } else if event.id == "5" {
                    println!("5");
                } else if event.id == "6" {
                    println!("6");
                } else if event.id == "7" {
                    println!("7");
                } else if event.id == "8" {
                    println!("8");
                } else if event.id == "9" {
                    println!("9");
                } else if event.id == "10" {
                    println!("10");
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

    let mut app = TrayApplication::new(quit_requested, monitor_callback);

    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("Error in event loop: {:?}", err);
    }
}
