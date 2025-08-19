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
                    eprintln!("Language code match found: {} -> {}", locale, available_locale);
                    return available_locale.to_string();
                }
            }
            
            // Special case mappings
            match lang_code {
                "zh" => {
                    eprintln!("Chinese language detected, defaulting to zh-CN");
                    "zh-CN".to_string()
                }, // Default to Simplified Chinese for any Chinese variant
                "en" => {
                    eprintln!("English language detected");
                    "en".to_string()
                },    // Default to English for any English variant
                _ => {
                    eprintln!("Unsupported language, defaulting to en");
                    "en".to_string()
                },       // Default to English for unsupported locales
            }
        }
        None => {
            eprintln!("Could not determine system locale, defaulting to en");
            "en".to_string()
        }, // Default to English if locale cannot be determined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_matching() {
        // We can't directly test the actual function because it depends on system locale,
        // but we can test the logic through helper functions
        
        // Test exact matches
        assert_eq!(match_locale_for_test("en"), "en");
        assert_eq!(match_locale_for_test("zh-CN"), "zh-CN");
        
        // Test language code matches
        assert_eq!(match_locale_for_test("zh-TW"), "zh-CN"); // Maps to Simplified Chinese
        assert_eq!(match_locale_for_test("zh-HK"), "zh-CN"); // Maps to Simplified Chinese
        assert_eq!(match_locale_for_test("zh-Hans-CN"), "zh-CN"); // Maps to Simplified Chinese
        assert_eq!(match_locale_for_test("en-US"), "en");    // Maps to English
        assert_eq!(match_locale_for_test("en-GB"), "en");    // Maps to English
        
        // Test fallback
        assert_eq!(match_locale_for_test("fr"), "en");       // French falls back to English
        assert_eq!(match_locale_for_test("de"), "en");       // German falls back to English
    }
    
    fn match_locale_for_test(locale: &str) -> String {
        // Simulate the matching logic
        if AVAILABLE_LOCALES.contains(&locale) {
            return locale.to_string();
        }
        
        let lang_code = if let Some(pos) = locale.find('-') {
            &locale[..pos]
        } else {
            locale
        };
        
        for &available_locale in AVAILABLE_LOCALES {
            if available_locale.starts_with(lang_code) {
                return available_locale.to_string();
            }
        }
        
        match lang_code {
            "zh" => "zh-CN".to_string(),
            "en" => "en".to_string(),
            _ => "en".to_string(),
        }
    }
}