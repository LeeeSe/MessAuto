use eframe::egui::Context;

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    // 创建一个临时的 egui 上下文来复制文本
    // 注意：这个方法需要在有 egui 上下文的环境中调用
    // 对于 auto_copy 功能，我们可能需要在主线程或适当的上下文中调用
    
    // 由于我们需要在没有 egui 上下文的情况下复制文本，
    // 我们可以使用其他方法，比如直接调用系统剪贴板 API
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // 使用 macOS 的 pbcopy 命令
        let output = Command::new("pbcopy")
            .arg(text)
            .output();
            
        match output {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to copy to clipboard: {}", e)),
        }
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