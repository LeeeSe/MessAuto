use crate::config::Config;
use fancy_regex::Regex;

pub fn extract_verification_code(content: &str) -> Option<String> {
    let config = Config::load().unwrap_or_default();

    let keyword_pos = find_first_keyword_position(content, &config.verification_keywords);
    if keyword_pos.is_none() {
        return None;
    }
    let keyword_pos = keyword_pos.unwrap();

    let candidates = extract_candidate_codes(content, &config.verification_regex);
    if candidates.is_empty() {
        return None;
    }

    let filtered_candidates = filter_candidates_step1(candidates, content);
    if filtered_candidates.is_empty() {
        return None;
    }

    let result = find_closest_candidate(filtered_candidates, keyword_pos);

    result
}

fn find_first_keyword_position(text: &str, keywords: &[String]) -> Option<usize> {
    // 找到第一个关键词的位置（保留原有的小写匹配逻辑）
    let text_lower = text.to_lowercase();
    for keyword in keywords {
        let keyword_lower = keyword.to_lowercase();
        if let Some(pos) = text_lower.find(&keyword_lower) {
            return Some(pos);
        }
    }
    None
}

fn extract_candidate_codes(text: &str, pattern: &str) -> Vec<(String, usize)> {
    // 使用正则提取候选验证码及其位置
    let re = Regex::new(pattern).unwrap();
    let mut candidates = Vec::new();

    for result in re.find_iter(text) {
        if let Ok(mat) = result {
            let code = mat.as_str();
            // 确保提取的字符串中至少包含一个数字
            if code.chars().any(|c| c.is_ascii_digit()) {
                let pos = mat.start();
                candidates.push((code.to_string(), pos));
            }
        }
    }

    candidates
}

fn filter_candidates_step1(candidates: Vec<(String, usize)>, _text: &str) -> Vec<(String, usize)> {
    // 过滤1: 去除包含多个'-'或在首尾的候选码
    let mut filtered = Vec::new();

    for (code, pos) in candidates {
        // 检查'-'的数量
        if code.matches('-').count() > 1 {
            continue;
        }

        filtered.push((code, pos));
    }

    filtered
}

fn find_closest_candidate(candidates: Vec<(String, usize)>, keyword_pos: usize) -> Option<String> {
    // 找到距离关键词最近的候选码，距离不超过100
    let mut closest_code: Option<String> = None;
    let mut min_distance = usize::MAX;

    for (code, code_pos) in candidates {
        // 计算距离 (候选码中心到关键词位置的距离)
        let code_center = code_pos + code.len() / 2;
        let distance = if code_center > keyword_pos {
            code_center - keyword_pos
        } else {
            keyword_pos - code_center
        };

        if distance < min_distance && distance <= 100 {
            min_distance = distance;
            closest_code = Some(code);
        }
    }

    closest_code
}

#[cfg(test)]
mod tests {
    use super::*;

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
                "【智谱AI】您的验证码为210465，请于3分钟内使用，若非本人操作，请忽略本短信。",
                Some("210465".to_string()),
            ),
            (
                "【倒三角】易支撑（登录）——您的账号W8406772本次登录验证码为666684，请勿泄露，有效时间5分钟，如非本人操作请忽略本短信。",
                Some("666684".to_string()),
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
            (
                "由XXXX发送，验证码：678571，验证码有效期2分钟，切勿将验证码泄露于他人。发送时间：2025-08-19 XX:XX:XX",
                Some("678571".to_string()),
            ),
            (
                "【CSDN】678571是你的验证码，有效期2分钟，切勿将验证码泄露于他人。发送时间：2025-08-19 XX:XX:XX",
                Some("678571".to_string()),
            ),
            (
                "Citi ID Code: 12345678 We'll NEVER call or text for this code.",
                Some("12345678".to_string()),
            ),
            (
                "Code is: RKJ-YP6 We'll NEVER call or text for this code.",
                Some("RKJ-YP6".to_string()),
            ),
            (
                "【google】your code is G-23414",
                Some("G-23414".to_string()),
            ),
            (
                "As a token of our appreciation, upon completing the survey, you will get a 10% discount promo code on your first payment. Your feedback is invaluable to us, and we are committed to making your experience as rewarding and effective as possible.",
                None,
            ),
            (
                "Hey LeeeSe2!A sign in attempt requires further verification because we did not recognize your device. To complete the sign in, enter the verification code on the unrecognized device. Device: Safari on macOS Verification code: 731464 If you did not attempt to sign in to your account, your password may be compromised. Visit https://github.com/settings/security to create a new, strong password for your GitHub account.If you'd like to automatically verify devices in the future, consider enabling two-factor authentication on your account. Visit https://docs.github.com/articles/configuring-two-factor-authentication to learn about two-factor authentication.If you decide to enable two-factor authentication, ensure you retain access to one or more account recovery methods. See https://docs.github.com/articles/configuring-two-factor-authentication-recovery-methods in the GitHub Help.Thanks,The GitHub Team",
                Some("731464".to_string()),
            ),
            (
                "[2025-08-21 00:47:22.775 INFO messauto::monitor::email] 邮件内容:------=_Part_153214_622935313.1755708437680
                Content-Type: text/plain; charset=GBK
                Content-Transfer-Encoding: 7bit

                code 723333
                ------=_Part_153214_622935313.1755708437680
                Content-Type: text/html; charset=GBK
                Content-Transfer-Encoding: 7bit

                ------=_Part_153214_622935313.1755708437680
                Content-Type: text/html; charset=UTF-8
                Content-Transfer-Encoding: quoted-printable

                <html>...</html>",
                Some("723333".to_string())
            ),
            (
                "【XXX】您在2024-04-02 17:23:35登录系统的动态密码为：524678",
                Some("524678".to_string())
            ),
            (
                "【倒三角】易支撑（登录）——您的账号W8406772本次登录验证码为666684，请勿泄露，有效时间5分钟，如非本人操作请忽略本短信。",
                Some("666684".to_string())
            )
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
