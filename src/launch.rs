use auto_launch::AutoLaunch;
use log::info;
use std::env;
use std::path::PathBuf;

use crate::config::Config;
use rust_i18n::t;

rust_i18n::i18n!("../locales");

pub struct LaunchManager {
    auto_launch: AutoLaunch,
}

impl LaunchManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let app_name = "Messauto";
        let app_path = Self::get_app_path()?;

        let auto_launch =
            AutoLaunch::new(app_name, &app_path.to_string_lossy(), true, &[] as &[&str]);

        Ok(Self { auto_launch })
    }

    fn get_app_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let exe_path = env::current_exe()?;
        Ok(exe_path)
    }

    pub fn enable_launch_at_login(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.auto_launch
            .enable()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub fn disable_launch_at_login(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.auto_launch
            .disable()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub fn is_launch_at_login_enabled(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.auto_launch
            .is_enabled()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub fn sync_with_config(&self, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
        let current_status = self.is_launch_at_login_enabled().unwrap_or(false);

        if config.launch_at_login && !current_status {
            self.enable_launch_at_login()?;
            info!("{}", t!("launch_manager.enabled_launch_at_login"));
        } else if !config.launch_at_login && current_status {
            self.disable_launch_at_login()?;
            info!("{}", t!("launch_manager.disabled_launch_at_login"));
        }

        Ok(())
    }
}
