use regex::Regex;

pub fn extract_verification_code(content: &str) -> Option<String> {
    // 使用正则表达式匹配常见验证码模式
    let patterns = [
        r"验证码[是为:：\s]*([0-9]{4,6})", // 中文验证码格式，允许没有分隔符
        r"验证码[^0-9]*([0-9]{4,6})",     // 中文验证码格式，允许任意非数字字符作为分隔符
        r"code[is:：\s]*([0-9]{4,6})",    // 英文验证码格式，允许没有分隔符
        r"code[^0-9]*([0-9]{4,6})",       // 英文验证码格式，允许任意非数字字符作为分隔符
    ];
    
    for pattern in patterns {
        let regex = Regex::new(pattern).unwrap();
        if let Some(captures) = regex.captures(content) {
            if captures.len() > 1 {
                return Some(captures[1].to_string());
            } else {
                return Some(captures[0].to_string());
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_verification_code() {
        assert_eq!(extract_verification_code("Your verification code is 123456"), Some("123456".to_string()));
        assert_eq!(extract_verification_code("验证码是 654321"), Some("654321".to_string()));
        assert_eq!(extract_verification_code("Code: 1234"), Some("1234".to_string()));
        assert_eq!(extract_verification_code("No code here"), None);
        assert_eq!(extract_verification_code("【CSDN】验证码934820，您正在使用短信验证码登录"), Some("934820".to_string()));
        assert_eq!(extract_verification_code("验证码123456，请勿泄露"), Some("123456".to_string()));
    }
} 