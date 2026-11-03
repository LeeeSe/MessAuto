use chrono::Local;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct LogTarget {
    file: Mutex<std::fs::File>,
}

impl LogTarget {
    pub fn new(file: std::fs::File) -> Self {
        Self {
            file: Mutex::new(file),
        }
    }
}

impl Write for LogTarget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let stdout_result = io::stdout().write(buf);

        let file_result = {
            let mut file = self.file.lock().unwrap();
            file.write(buf)
        };

        stdout_result.or(file_result)
    }

    fn flush(&mut self) -> io::Result<()> {
        let stdout_result = io::stdout().flush();

        let file_result = {
            let mut file = self.file.lock().unwrap();
            file.flush()
        };

        stdout_result.or(file_result)
    }
}

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

    /// 是否启用 HTTP Webhook 监听。
    #[serde(default = "default_listen_webhook")]
    pub listen_webhook: bool,
    /// Webhook 监听地址，默认 0.0.0.0（局域网可访问）。
    #[serde(default = "default_webhook_host")]
    pub webhook_host: String,
    /// Webhook 监听端口。
    #[serde(default = "default_webhook_port")]
    pub webhook_port: u16,
    /// Webhook 鉴权 token；为空时应用启动会自动生成。
    #[serde(default)]
    pub webhook_token: String,

    #[serde(default = "default_listen_webhook")]
    pub ngrok_enabled: bool,
    #[serde(default)]
    pub ngrok_authtoken: String,
    #[serde(default)]
    pub ngrok_domain: String,

    #[serde(default)]
    version: u32,
}

const CURRENT_CONFIG_VERSION: u32 = 2;

fn default_listen_webhook() -> bool {
    true
}

fn default_webhook_host() -> String {
    "0.0.0.0".to_string()
}

fn default_webhook_port() -> u16 {
    8978
}

/// 从系统随机源生成 32 位十六进制 token。
pub fn generate_token() -> io::Result<String> {
    let mut buf = [0u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut buf)?;
    Ok(buf.iter().map(|b| format!("{:02x}", b)).collect())
}

fn read_ngrok_authtoken(path: &std::path::Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    content.lines().find_map(|line| {
        let value = line.trim().strip_prefix("authtoken:")?.trim();
        let value = value.split(" #").next()?.trim().trim_matches(['\'', '"']);
        (!value.is_empty()).then(|| value.to_string())
    })
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
                "验证码".to_string(),
                "动态密码".to_string(),
                "验证".to_string(),
                "代码".to_string(),
                "verification code".to_string(),
                "captcha code".to_string(),
                "verification".to_string(),
                "captcha".to_string(),
                "code".to_string(),
                "인증".to_string(),
            ],
            verification_regex: r"(?=[a-zA-Z0-9-]*[0-9])[a-zA-Z0-9-]{4,8}".to_string(),
            listen_webhook: default_listen_webhook(),
            webhook_host: default_webhook_host(),
            webhook_port: default_webhook_port(),
            webhook_token: String::new(),
            ngrok_enabled: true,
            ngrok_authtoken: String::new(),
            ngrok_domain: String::new(),
            version: CURRENT_CONFIG_VERSION,
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
        dirs::config_dir()
            .unwrap_or_default()
            .join("messauto")
            .join("logs")
            .join("app.log")
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::get_config_path();

        if !path.exists() {
            let config = Self::default();
            config.save()?;
            log::info!("{}", t!("logging.created_initial_config"));
            return Ok(config);
        }

        let content = fs::read_to_string(&path)?;

        match toml::from_str::<Self>(&content) {
            Ok(config) => {
                let needs_migration = config.version < CURRENT_CONFIG_VERSION;
                let config = Self::migrate_config(config);
                if needs_migration {
                    config.save()?;
                }
                Ok(config)
            }
            Err(_) => {
                log::warn!("{}", t!("logging.migrating_legacy_config"));
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
        if config.version < CURRENT_CONFIG_VERSION {
            // v2 新增 webhook 字段，已通过 serde 默认值补齐。
            config.version = CURRENT_CONFIG_VERSION;
        }
        config
    }

    /// 确保 webhook token 非空；生成了新 token 时返回 true（调用方负责保存）。
    pub fn ensure_webhook_token(&mut self) -> io::Result<bool> {
        if self.webhook_token.trim().is_empty() {
            self.webhook_token = generate_token()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 仅在尚未设置时，从已有的 ngrok 配置导入 authtoken。
    pub fn import_ngrok_authtoken(&mut self) -> bool {
        if !self.ngrok_authtoken.is_empty() {
            return false;
        }
        let Some(home) = dirs::home_dir() else {
            return false;
        };
        for path in [
            home.join("Library/Application Support/ngrok/ngrok.yml"),
            home.join(".config/ngrok/ngrok.yml"),
        ] {
            if let Some(token) = read_ngrok_authtoken(&path) {
                self.ngrok_authtoken = token;
                return true;
            }
        }
        false
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
                    "验证码".to_string(),
                    "动态密码".to_string(),
                    "验证".to_string(),
                    "代码".to_string(),
                    "verification code".to_string(),
                    "captcha code".to_string(),
                    "verification".to_string(),
                    "captcha".to_string(),
                    "code".to_string(),
                    "인증".to_string(),
                ]
            }),
            verification_regex: legacy
                .verification_regex
                .unwrap_or_else(|| r"(?=[a-zA-Z0-9-]*[0-9])[a-zA-Z0-9-]{4,8}".to_string()),
            listen_webhook: default_listen_webhook(),
            webhook_host: default_webhook_host(),
            webhook_port: default_webhook_port(),
            webhook_token: String::new(),
            ngrok_enabled: true,
            ngrok_authtoken: String::new(),
            ngrok_domain: String::new(),
            version: CURRENT_CONFIG_VERSION,
        })
    }

    pub fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
        let log_dir = dirs::config_dir()
            .unwrap_or_default()
            .join("messauto")
            .join("logs");

        fs::create_dir_all(&log_dir)?;

        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("app.log"))?;

        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .target(env_logger::Target::Pipe(Box::new(LogTarget::new(log_file))))
            .format(|buf, record| {
                let local_time = Local::now();
                writeln!(
                    buf,
                    "[{} {} {}] {}",
                    local_time.format("%Y-%m-%d %H:%M:%S%.3f"),
                    record.level(),
                    record.target(),
                    record.args()
                )
            })
            .init();

        log::info!("{}", t!("logging.initialized"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_preserves_settings_and_defaults_new_fields() {
        let original = Config {
            auto_paste: true,
            auto_enter: true,
            direct_input: true,
            listen_email: false,
            listen_message: false,
            floating_window: false,
            verification_keywords: vec!["custom".into()],
            verification_regex: "[0-9]{6}".into(),
            ..Config::default()
        };
        let mut old: toml::Table = toml::from_str(&toml::to_string(&original).unwrap()).unwrap();
        for key in [
            "listen_webhook",
            "webhook_host",
            "webhook_port",
            "webhook_token",
            "ngrok_enabled",
            "ngrok_authtoken",
            "ngrok_domain",
        ] {
            old.remove(key);
        }
        old.insert("version".into(), 1.into());
        let loaded =
            Config::migrate_config(toml::from_str(&toml::to_string(&old).unwrap()).unwrap());
        assert_eq!(loaded.version, 2);
        assert!(loaded.auto_paste && loaded.auto_enter && loaded.direct_input);
        assert!(!loaded.listen_email && !loaded.listen_message && !loaded.floating_window);
        assert_eq!(loaded.verification_keywords, original.verification_keywords);
        assert_eq!(loaded.verification_regex, original.verification_regex);
        assert!(loaded.listen_webhook && loaded.ngrok_enabled);
        assert_eq!(loaded.webhook_host, "0.0.0.0");
        assert_eq!(loaded.webhook_port, 8978);
        assert!(
            loaded.webhook_token.is_empty()
                && loaded.ngrok_authtoken.is_empty()
                && loaded.ngrok_domain.is_empty()
        );
    }

    #[test]
    fn token_is_random_hex_and_only_generated_once() {
        let mut config = Config::default();
        assert!(config.ensure_webhook_token().unwrap());
        let token = config.webhook_token.clone();
        assert_eq!(token.len(), 32);
        assert!(token.bytes().all(|c| c.is_ascii_hexdigit()));
        assert!(!config.ensure_webhook_token().unwrap());
        assert_eq!(config.webhook_token, token);
        assert_ne!(generate_token().unwrap(), token);
    }

    #[test]
    fn imports_indented_and_quoted_ngrok_tokens() {
        let path =
            std::env::temp_dir().join(format!("messauto-ngrok-import-{}.yml", std::process::id()));
        for (text, expected) in [
            (
                "version: 3\nagent:\n  authtoken: example-token\n",
                Some("example-token"),
            ),
            (
                "agent:\n  authtoken: \"double-quoted\"\n",
                Some("double-quoted"),
            ),
            (
                "authtoken: 'single-quoted' # comment\n",
                Some("single-quoted"),
            ),
            ("version: 3\n", None),
            ("agent:\n  authtoken:\n", None),
        ] {
            fs::write(&path, text).unwrap();
            assert_eq!(read_ngrok_authtoken(&path).as_deref(), expected);
        }
        fs::remove_file(path).unwrap();
    }
}
