mod clipboard;
mod config;
mod floating_window;
mod ipc;
mod language;
mod launch;
mod monitor;
mod notification;
mod parser;
mod tray;
mod updater;

use log::info;
use rust_i18n::t;

use std::env;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use tokio::runtime::Runtime;

rust_i18n::i18n!("./locales");

fn main() {
    let system_locale = language::detect_system_locale();
    rust_i18n::set_locale(&system_locale);
    println!("=== {} ===", t!("app.name"));

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

    // Initialize launch manager and sync with config
    if let Ok(launch_manager) = launch::LaunchManager::new() {
        if let Err(e) = launch_manager.sync_with_config(&app_config.lock().unwrap()) {
            log::error!("Failed to sync launch at login status: {}", e);
        }
    }

    if floating_window::maybe_start_floating_window() {
        return;
    }

    info!("{}", t!("monitor.starting_auto_update_checker"));
    updater::start_auto_update_checker();

    let test_mode = env::args().any(|arg| arg == "--test");

    if test_mode {
        sleep(Duration::from_secs(2));
        info!("{}", t!("monitor.starting_test_verification_window"));
        if let Ok(child) = ipc::spawn_floating_window("123456") {
            info!(
                "{}",
                t!(
                    "monitor.floating_window_process_started",
                    child = format!("{:?}", child)
                )
            );

            thread::sleep(Duration::from_secs(5));

            if let Ok(child2) = ipc::spawn_floating_window("654321") {
                info!(
                    "{}",
                    t!(
                        "monitor.second_floating_window_process_started",
                        child2 = format!("{:?}", child2)
                    )
                );
            }

            thread::sleep(Duration::from_secs(600));
        }
    } else {
        info!("{}", t!("monitor.starting_verification_extractor"));
        let rt = Runtime::new().unwrap();

        let quit_requested = Arc::new(Mutex::new(false));
        let quit_requested_clone = quit_requested.clone();

        let monitor_callback = Box::new(move || {
            info!("{}", t!("actor.tray_application_initialized"));
        });

        info!("{}", t!("tray.initializing_tray_icon"));
        info!("{}", t!("tray.about_to_run_tray_application"));

        let _guard = rt.enter();
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
                info!("{}", t!("monitor.shutting_down_application"));
                rt.shutdown_timeout(Duration::from_secs(2));
            }
        }
    }

    info!("{}", t!("monitor.application_exited"));
}
