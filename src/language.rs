use sys_locale::get_locale;

/// Available locales in the application
const AVAILABLE_LOCALES: &[&str] = &["en", "zh-CN"];

/// Detects the system locale and matches it to available locales
pub fn detect_system_locale() -> String {
    match get_locale() {
        Some(locale) => {
            // Print debug info
            eprintln!("System locale detected: {}", locale);

            // Try to match the exact locale first
            if AVAILABLE_LOCALES.contains(&locale.as_str()) {
                eprintln!("Exact match found: {}", locale);
                return locale;
            }

            // Try to match by language code (e.g., "zh" for "zh-CN" or "zh-TW")
            let lang_code = if let Some(pos) = locale.find('-') {
                &locale[..pos]
            } else {
                locale.as_str()
            };

            eprintln!("Language code extracted: {}", lang_code);

            // Check for direct language code matches
            for &available_locale in AVAILABLE_LOCALES {
                if available_locale.starts_with(lang_code) {
                    eprintln!(
                        "Language code match found: {} -> {}",
                        locale, available_locale
                    );
                    return available_locale.to_string();
                }
            }

            // Special case mappings
            match lang_code {
                "zh" => {
                    eprintln!("Chinese language detected, defaulting to zh-CN");
                    "zh-CN".to_string()
                } // Default to Simplified Chinese for any Chinese variant
                "en" => {
                    eprintln!("English language detected");
                    "en".to_string()
                } // Default to English for any English variant
                _ => {
                    eprintln!("Unsupported language, defaulting to en");
                    "en".to_string()
                } // Default to English for unsupported locales
            }
        }
        None => {
            eprintln!("Could not determine system locale, defaulting to en");
            "en".to_string()
        } // Default to English if locale cannot be determined
    }
}
