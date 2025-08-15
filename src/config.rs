use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub auto_copy: bool,
    pub auto_paste: bool,
    pub direct_input: bool,
    pub launch_at_login: bool,
    pub listen_email: bool,
    pub floating_window: bool,
    
    #[serde(default)]
    version: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_copy: false,
            auto_paste: false,
            direct_input: false,
            launch_at_login: false,
            listen_email: true,
            floating_window: true,
            version: 1,
        }
    }
}

impl Config {
    pub fn get_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_default()
            .join("messauto")
            .join("config.toml")
    }

    pub fn get_log_file_path() -> PathBuf {
        dirs::state_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
            .join("messauto")
            .join("logs")
            .join("app.log")
    }
    
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::get_config_path();
        
        if !path.exists() {
            let config = Self::default();
            config.save()?;
            log::info!("Created initial config file");
            return Ok(config);
        }

        let content = fs::read_to_string(&path)?;
        
        // 尝试解析当前版本，失败则尝试旧版本
        match toml::from_str(&content) {
            Ok(mut config) => {
                config = Self::migrate_config(config);
                config.save()?;
                Ok(config)
            }
            Err(_) => {
                log::warn!("Migrating legacy config");
                let config = Self::migrate_legacy_config(&content)?;
                config.save()?;
                Ok(config)
            }
        }
    }
    
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_config_path();
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    // 配置迁移
    fn migrate_config(mut config: Self) -> Self {
        if config.version < 1 {
            config.version = 1;
            // 未来版本迁移逻辑在这里
        }
        config
    }

    // 旧版本兼容
    fn migrate_legacy_config(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        #[derive(Deserialize)]
        struct LegacyConfig {
            auto_copy: Option<bool>,
            auto_paste: Option<bool>,
            restore_clipboard: Option<bool>,
            launch_at_login: Option<bool>,
            listen_email: Option<bool>,
            floating_window: Option<bool>,
        }

        let legacy: LegacyConfig = toml::from_str(content)?;
        Ok(Self {
            auto_copy: legacy.auto_copy.unwrap_or_default(),
            auto_paste: legacy.auto_paste.unwrap_or_default(),
            direct_input: legacy.restore_clipboard.unwrap_or_default(),
            launch_at_login: legacy.launch_at_login.unwrap_or_default(),
            listen_email: legacy.listen_email.unwrap_or(true),
            floating_window: legacy.floating_window.unwrap_or(true),
            version: 1,
        })
    }

    // 初始化日志系统
    pub fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
        let log_dir = dirs::state_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
            .join("messauto")
            .join("logs");
        
        fs::create_dir_all(&log_dir)?;
        
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .target(env_logger::Target::Pipe(Box::new(
                std::io::stdout(),
            )))
            .init();
        
        // 写入文件副本
        if let Ok(mut log_file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("app.log"))
        {
            use std::io::Write;
            let _ = writeln!(log_file, "Logging initialized at: {:?}", chrono::Utc::now());
        }
        
        log::info!("Logging initialized");
        Ok(())
    }
}