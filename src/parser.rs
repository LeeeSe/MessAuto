use crate::config::Config;
use regex_lite::Regex;

pub fn extract_verification_code(content: &str) -> Option<String> {
    let config = Config::load().unwrap_or_default();

    let has_verification_keyword = config
        .verification_keywords
        .iter()
        .any(|keyword| content.to_lowercase().contains(&keyword.to_lowercase()));

    if !has_verification_keyword {
        return None;
    }

    let code_patterns = [&config.verification_regex];

    let mut candidates = Vec::new();
    for pattern in code_patterns {
        let regex = Regex::new(pattern).unwrap();
        for m in regex.find_iter(content) {
            let candidate = m.as_str();
            // 确保提取的字符串中至少包含一个数字
            if candidate.chars().any(|c| c.is_ascii_digit()) {
                candidates.push(candidate.to_string());
            }
        }
    }

    if !candidates.is_empty() {
        return candidates
            .into_iter()
            .max_by_key(|c| c.chars().filter(|ch| ch.is_ascii_digit()).count());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_verification_code_no_keywords() {
        assert_eq!(
            extract_verification_code("【腾讯云】尊敬的腾讯云用户，您的账号即将到期。"),
            None
        );
        assert_eq!(
            extract_verification_code("Hello world, no codes here"),
            None
        );
    }

    #[test]
    fn test_extract_verification_code_mixed_candidates() {
        let sms_content = "【测试】您的验证码是ABC123和XYZ4567，请使用ABC123。";
        assert_eq!(
            extract_verification_code(sms_content),
            Some("XYZ4567".to_string())
        );
    }

    #[test]
    fn test_extract_verification_code_comprehensive_accuracy() {
        let test_cases = vec![
            (
                "【自如网】自如验证码 356407，有效时间为一分钟，请勿将验证码告知任何人！如非您本人操作，请及时致电4001001111",
                Some("356407".to_string()),
            ),
            (
                "【腾讯云】尊敬的腾讯云用户，您的账号（账号 ID：100022305033，昵称：724818342@qq.com）下有 1 个域名即将到期：xjp.asia 将于北京时间 2023-11-01 到期。域名过期三天后仍未续费，将会停止正常解析，为避免影响您的业务正常使用，请及时登录腾讯云进行续费：https://mc.tencent.com/N1op7G3l，详情可查看邮件或站内信。",
                None,
            ),
            (
                "【AIdea】您的验证码为：282443，请勿泄露于他人！",
                Some("282443".to_string()),
            ),
            (
                "【Microsoft】将 12345X 初始化Microsoft账户安全代码",
                Some("12345X".to_string()),
            ),
            (
                "【百度账号】验证码：534571 。验证码提供他人可能导致百度账号被盗，请勿转发或泄漏。",
                Some("534571".to_string()),
            ),
            (
                "【必胜客】116352（动态验证码），请在30分钟内填写",
                Some("116352".to_string()),
            ),
            (
                "This output contains a captcha with non-alphanumeric characters: ABCD123",
                Some("ABCD123".to_string()),
            ),
            (
                "您正在使用境外网上支付验证服务，动态密码为729729。动态密码连续输错3次，您的此次交易验证会失败。请勿向他人泄露！[中国工商银行]。【工商银行】",
                Some("729729".to_string()),
            ),
            (
                "【Microsoft】将123456用作Microsoft账户安全代码",
                Some("123456".to_string()),
            ),
            (
                "【APPLE】Apple ID代码为：724818。请勿与他人共享。",
                Some("724818".to_string()),
            ),
            (
                "【腾讯云】验证码：134560，5分钟内有效，为了保障您的账户安全，请勿向他人泄漏验证码信息",
                Some("134560".to_string()),
            ),
            (
                "If this was you, your verification code is: 047289 If you didn't request i： click here to deny.",
                Some("047289".to_string()),
            ),
            ("your code is 432141", Some("432141".to_string())),
        ];

        let mut total_tests = 0;
        let mut passed_tests = 0;
        let mut failed_cases = Vec::new();

        for (input, expected) in test_cases {
            total_tests += 1;
            let result = extract_verification_code(input);

            if result == expected {
                passed_tests += 1;
            } else {
                failed_cases.push((input, expected, result));
            }
        }

        // 输出测试结果统计
        println!("=== 验证码提取正确率测试结果 ===");
        println!("总测试数: {}", total_tests);
        println!("通过测试数: {}", passed_tests);
        println!("失败测试数: {}", total_tests - passed_tests);
        println!(
            "正确率: {:.2}%",
            (passed_tests as f64 / total_tests as f64) * 100.0
        );

        if !failed_cases.is_empty() {
            println!("\n=== 失败案例 ===");
            for (input, expected, result) in failed_cases {
                println!("输入: \"{}\"", input);
                println!("期望: {:?}", expected);
                println!("实际: {:?}", result);
                println!("---");
            }
        }

        // 验证正确率应该达到 100%
        assert_eq!(passed_tests, total_tests, "验证码提取正确率未达到100%");
    }
}
