use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub auto_paste: bool,
    pub auto_enter: bool,
    pub direct_input: bool,
    pub launch_at_login: bool,
    pub listen_email: bool,
    pub listen_message: bool,
    pub floating_window: bool,
    pub verification_keywords: Vec<String>,
    pub verification_regex: String,

    #[serde(default)]
    version: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_paste: false,
            auto_enter: false,
            direct_input: false,
            launch_at_login: false,
            listen_email: true,
            listen_message: true,
            floating_window: true,
            verification_keywords: vec![
                "验证".to_string(),
                "动态密码".to_string(),
                "verification".to_string(),
                "code".to_string(),
                "captcha".to_string(),
                "인증".to_string(),
                "代码".to_string(),
            ],
            verification_regex: r"\b[a-zA-Z0-9][a-zA-Z0-9-]{2,6}[a-zA-Z0-9]\b".to_string(),
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

    fn migrate_config(mut config: Self) -> Self {
        if config.version < 1 {
            config.version = 1;
        }
        config
    }

    fn migrate_legacy_config(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        #[derive(Deserialize)]
        struct LegacyConfig {
            auto_paste: Option<bool>,
            auto_enter: Option<bool>,
            restore_clipboard: Option<bool>,
            launch_at_login: Option<bool>,
            listen_email: Option<bool>,
            listen_message: Option<bool>,
            floating_window: Option<bool>,
            verification_keywords: Option<Vec<String>>,
            verification_regex: Option<String>,
        }

        let legacy: LegacyConfig = toml::from_str(content)?;
        Ok(Self {
            auto_paste: legacy.auto_paste.unwrap_or_default(),
            auto_enter: legacy.auto_enter.unwrap_or_default(),
            direct_input: legacy.restore_clipboard.unwrap_or_default(),
            launch_at_login: legacy.launch_at_login.unwrap_or_default(),
            listen_email: legacy.listen_email.unwrap_or(true),
            listen_message: legacy.listen_message.unwrap_or(true),
            floating_window: legacy.floating_window.unwrap_or(true),
            verification_keywords: legacy.verification_keywords.unwrap_or_else(|| {
                vec![
                    "验证".to_string(),
                    "动态密码".to_string(),
                    "verification".to_string(),
                    "code".to_string(),
                    "인증".to_string(),
                    "代码".to_string(),
                    "captcha".to_string(),
                ]
            }),
            verification_regex: legacy
                .verification_regex
                .unwrap_or_else(|| r"\b[a-zA-Z0-9][a-zA-Z0-9-]{2,6}[a-zA-Z0-9]\b".to_string()),
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
            .target(env_logger::Target::Pipe(Box::new(std::io::stdout())))
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
