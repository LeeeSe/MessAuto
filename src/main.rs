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

    // 初始化日志系统
    if let Err(e) = config::Config::init_logging() {
        eprintln!("Failed to initialize logging: {}", e);
        // 继续运行，但使用标准输出日志
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    // 初始化配置系统
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

        let monitor_handle = rt.spawn(async {
            monitor::start_monitoring().await;
        });

        let monitor_callback = Box::new(move || {
            info!("开始监控短信和邮件...");
            // 监控服务已经在主运行时中启动
        });

        info!("初始化托盘图标...");
        info!("About to run tray application...");
        tray::run_tray_application(quit_requested, app_config, Some(monitor_callback));

        {
            let quit = quit_requested_clone.lock().unwrap();
            if *quit {
                info!("关闭 tokio 运行时...");
                drop(quit);

                monitor_handle.abort();

                rt.shutdown_timeout(Duration::from_secs(2));
                info!("应用程序正在退出...");
                std::process::exit(0);
            }
        }
    }

    info!("Application exited");
}
