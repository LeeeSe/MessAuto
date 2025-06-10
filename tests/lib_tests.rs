use home::home_dir;

use MessAuto::{
    check_captcha_or_other, check_for_updates, config_path, get_captchas, get_real_captcha,
    get_sys_locale,
};

#[test]
fn test_get_sys_locale() {
    let locale = get_sys_locale();
    assert!(locale == "zh-CN" || locale == "en" || locale == "de");
}

#[test]
fn test_config_path() {
    let expected_path = home_dir()
        .unwrap()
        .join(".config")
        .join("messauto")
        .join("messauto.json");
    assert_eq!(config_path(), expected_path);
}

#[test]
fn test_check_captcha_or_other() {
    // Test that the function returns false when the stdout doesn't contain any flags
    let stdout = "【自如网】自如验证码 356407，有效时间为一分钟，请勿将验证码告知任何人！如非您本人操作，请及时致电4001001111".to_string();
    let flags = vec![
        "验证码".to_string(),
        "动态密码".to_string(),
        "verification".to_string(),
        "code".to_string(),
        "인증".to_string(),
        "代码".to_string(),
    ];
    let result = check_captcha_or_other(&stdout, &flags);
    assert_eq!(result, true);

    // Test that the function returns true and the correct flag when the stdout contains a flag
    let stdout =
        "【腾讯云】尊敬的腾讯云用户，您的账号（账号 ID：100022305033，昵称：724818342@qq.com）下有 1 个域名即将到期：xjp.asia 将于北京时间 2023-11-01 到期。域名过期三天后仍未续费，将会停止正常解析，为避免影响您的业务正常使用，请及时登录腾讯云进行续费：https://mc.tencent.com/N1op7G3l，详情可查看邮件或站内信。。".to_string();
    let result = check_captcha_or_other(&stdout, &flags);
    assert_eq!(result, false);

    // Test that the function returns true and the correct flag when the stdout contains multiple flags
    let stdout = "【AIdea】您的验证码为：282443，请勿泄露于他人！".to_string();
    let result = check_captcha_or_other(&stdout, &flags);
    assert_eq!(result, true);

    let stdout = "【Microsoft】将 12345X 初始化Microsoft账户安全代码".to_string();
    let result = check_captcha_or_other(&stdout, &flags);
    assert_eq!(result, true);
}

#[test]
fn test_get_captchas() {
    let stdout = "【自如网】自如验证码 356407, 请及时致电4001001111".to_string();
    let captchas = get_captchas(&stdout);
    assert_eq!(captchas, vec!["356407".to_string()]);

    let stdout =
        "【百度账号】验证码：534571 。验证码提供他人可能导致百度账号被盗，请勿转发或泄漏。"
            .to_string();
    let captchas = get_captchas(&stdout);
    assert_eq!(captchas, vec!["534571".to_string()]);

    let stdout = "【AIdea】您的验证码为：282443，请勿泄露于他人！".to_string();
    let captchas = get_captchas(&stdout);
    assert_eq!(captchas, vec!["282443".to_string()]);

    let stdout = "【必胜客】116352（动态验证码），请在30分钟内填写".to_string();
    let captchas = get_captchas(&stdout);
    assert_eq!(captchas, vec!["116352".to_string()]);

    let stdout =
        "This output contains a captcha with non-alphanumeric characters: ABCD123".to_string();
    let captchas = get_captchas(&stdout);
    assert_eq!(captchas, vec!["ABCD123".to_string()]);

    let stdout = "[s1mple] your code is 123456".to_string();
    let captchas = get_captchas(&stdout);
    assert_eq!(captchas, vec!["s1mple".to_string(), "123456".to_string()]);

    let stdout = "您的验证码是12345，请勿泄露给他人。".to_string();
    let captchas = get_captchas(&stdout);
    assert_eq!(captchas, vec!["12345".to_string()]);

    let stdout = "您正在使用境外网上支付验证服务，动态密码为729729。动态密码连续输错3次，您的此次交易验证会失败。请勿向他人泄露！[中国工商银行]。【工商银行】".to_string();
    let captchas = get_captchas(&stdout);
    assert_eq!(captchas, vec!["729729".to_string()]);
}

#[test]
fn test_get_real_captcha() {
    let stdout = String::from("【Microsoft】将123456用作Microsoft账户安全代码");
    let result = get_real_captcha(&stdout);
    assert_eq!(result, "123456");

    let stdout = String::from("【APPLE】Apple ID代码为：724818。请勿与他人共享。");
    let result = get_real_captcha(&stdout);
    assert_eq!(result, "724818");

    let stdout = String::from("【自如网】自如验证码 356407，有效时间为一分钟，请勿将验证码告知任何人！如非您本人操作，请及时致电4001001111");
    let result = get_real_captcha(&stdout);
    assert_eq!(result, "356407");

    let stdout = String::from(
        "【腾讯云】验证码：134560，5分钟内有效，为了保障您的账户安全，请勿向他人泄漏验证码信息",
    );
    let result = get_real_captcha(&stdout);
    assert_eq!(result, "134560");

    let stdout = String::from("Your confirmation code is below 一 enter it in your open browser window and we'll help you get signed in RKJ-YP6 If you didn't request this email, there's nothing to worry about - you can safely ignore it.");
    let result = get_real_captcha(&stdout);
    assert_eq!(result, "RKJ-YP6");

    let stdout = String::from("If this was you, your verification code is: 047289 If you didn't request i： click here to deny.");
    let result = get_real_captcha(&stdout);
    assert_eq!(result, "047289");
}

// #[test]
// fn test_check_for_updates() {
//     // let need_update = check_for_updates();
//     // assert_eq!(need_update.is_ok(), true);
//
//     let need_update = check_for_updates();
//     println!("error:{:?}", need_update);
//     assert_eq!(need_update.is_ok(), true);
// }
