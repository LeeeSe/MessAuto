mod clipboard;
mod config;
mod floating_window;
mod ipc;
mod monitor;
mod parser;
mod tray;

use log::info;

use std::env;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use tokio::runtime::Runtime;

fn main() {
    println!("=== Starting Messauto ===");

    if let Err(e) = config::Config::init_logging() {
        eprintln!("Failed to initialize logging: {}", e);
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    let app_config = match config::Config::load() {
        Ok(config) => Arc::new(Mutex::new(config)),
        Err(e) => {
            log::error!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    if floating_window::maybe_start_floating_window() {
        return;
    }

    let test_mode = env::args().any(|arg| arg == "--test");

    if test_mode {
        sleep(Duration::from_secs(2));
        info!("启动测试验证码窗口...");
        if let Ok(child) = ipc::spawn_floating_window("123456") {
            info!("悬浮窗进程已启动: {:?}", child);

            thread::sleep(Duration::from_secs(5));

            if let Ok(child2) = ipc::spawn_floating_window("654321") {
                info!("第二个悬浮窗进程已启动: {:?}", child2);
            }

            thread::sleep(Duration::from_secs(600));
        }
    } else {
        info!("启动验证码提取器...");
        let rt = Runtime::new().unwrap();

        let quit_requested = Arc::new(Mutex::new(false));
        let quit_requested_clone = quit_requested.clone();

        let monitor_callback = Box::new(move || {
            info!("Tray application initialized. Monitoring actor is running.");
        });

        info!("初始化托盘图标...");
        info!("About to run tray application...");

        // --- 核心改动：启动Actor并获取Sender ---
        let _guard = rt.enter(); // 确保在正确的tokio上下文中
        let monitor_sender = monitor::start_monitoring_actor();

        tray::run_tray_application(
            quit_requested,
            app_config,
            Some(monitor_callback),
            monitor_sender,
        );

        {
            let quit = quit_requested_clone.lock().unwrap();
            if *quit {
                info!("Shutting down application...");
                rt.shutdown_timeout(Duration::from_secs(2));
            }
        }
    }

    info!("Application exited");
}
