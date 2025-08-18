use eframe::egui::Context;
use enigo::{
    Direction::{Press, Release},
    Enigo, Key, Keyboard, Settings,
};

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};

        // 使用 macOS 的 pbcopy 命令，通过标准输入传递内容
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn pbcopy: {}", e))?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(text.as_bytes())
                .map_err(|e| format!("Failed to write to pbcopy stdin: {}", e))?;

            // 关闭stdin以让pbcopy处理内容
            // stdin会在drop时自动关闭
        }

        child
            .wait()
            .map_err(|e| format!("Failed to wait for pbcopy: {}", e))?;

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        // 对于其他平台，这里可以实现相应的方法
        // 目前只支持 macOS
        Err("Auto copy is currently only supported on macOS".to_string())
    }
}

pub fn copy_text_with_egui(ctx: &Context, text: &str) {
    ctx.copy_text(text.to_string());
}

pub fn auto_paste(direct_input: bool, text: &str) -> Result<(), String> {
    if direct_input {
        // 方法2: 使用 enigo 直接输入字符串
        paste_with_enigo(text)
    } else {
        // 方法1: 模拟 Command + V 粘贴
        paste_with_keyboard_shortcut()
    }
}

fn paste_with_enigo(text: &str) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to create Enigo instance: {}", e))?;

    enigo
        .text(text)
        .map_err(|e| format!("Failed to input text: {}", e))?;

    Ok(())
}

fn paste_with_keyboard_shortcut() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to create Enigo instance: {}", e))?;

    // 模拟 Command + V (macOS)
    enigo
        .key(Key::Meta, Press)
        .map_err(|e| format!("Failed to press Command key: {}", e))?;

    enigo
        .key(Key::Unicode('v'), enigo::Direction::Click)
        .map_err(|e| format!("Failed to press V key: {}", e))?;

    enigo
        .key(Key::Meta, Release)
        .map_err(|e| format!("Failed to release Command key: {}", e))?;

    Ok(())
}

pub fn press_enter() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to create Enigo instance: {}", e))?;

    // 模拟 Enter 键
    enigo
        .key(Key::Return, enigo::Direction::Click)
        .map_err(|e| format!("Failed to press Enter key: {}", e))?;

    Ok(())
}
