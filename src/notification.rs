use std::env;
use std::process::Command;

fn get_icon_script_part() -> String {
    if let Ok(exe_path) = env::current_exe() {
        if let Some(mut path) = exe_path.parent().map(|p| p.to_path_buf()) {
            if path.ends_with("Contents/MacOS") {
                path.pop();
                path.pop();
                path.push("Contents/Resources/Messauto.icns");

                if path.exists() {
                    return format!(
                        r#"with icon (POSIX file "{}")"#,
                        escape_applescript(&path.to_string_lossy())
                    );
                }
            }
        }
    }

    // Fallback: If not in a bundle or icon not found, return an empty string.
    // The system will then use the icon of the calling process (e.g., Terminal), which is a sensible default.
    "".to_string()
}

pub fn dialog(title: &str, content: &str, true_button: &str, false_button: &str) -> bool {
    let icon_part = get_icon_script_part();

    let script = format!(
        r#"display dialog "{content}" with title "{title}" {icon_part} buttons {{"{false_button}", "{true_button}"}} default button "{true_button}" cancel button "{false_button}""#,
        title = escape_applescript(title),
        content = escape_applescript(content),
        true_button = escape_applescript(true_button),
        false_button = escape_applescript(false_button),
        icon_part = icon_part
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        let button_pressed = String::from_utf8_lossy(&output.stdout);
        let button_text = button_pressed.trim();

        button_text.contains(true_button)
    } else {
        false
    }
}

fn escape_applescript(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// 显示系统通知横幅。
pub fn notify(title: &str, body: &str) {
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        escape_applescript(body),
        escape_applescript(title)
    );
    if let Err(error) = Command::new("osascript").args(["-e", &script]).output() {
        log::error!("Failed to display notification: {}", error);
    }
}

/// 显示输入框，用户取消时返回 None。
pub fn input_dialog(title: &str, prompt: &str, hidden: bool) -> Option<String> {
    let script = format!(
        "text returned of (display dialog \"{}\" with title \"{}\" default answer \"\"{})",
        escape_applescript(prompt),
        escape_applescript(title),
        if hidden { " with hidden answer" } else { "" }
    );
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}
