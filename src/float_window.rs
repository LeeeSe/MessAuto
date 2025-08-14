use std::{fs::File, thread::sleep, time::Duration};

use arboard::Clipboard;
use i_slint_backend_winit::winit::platform::macos::WindowBuilderExtMacOS;
use log::{error, info};
use mouse_position::mouse_position::Mouse;
use rust_i18n::t;
use simplelog::{
    ColorChoice, CombinedLogger, ConfigBuilder, LevelFilter, TermLogger, TerminalMode, WriteLogger,
};
use MessAuto::{
    get_old_clipboard_contents, get_sys_locale, log_path, paste_script, read_config,
    recover_clipboard_contents, return_script,
};

slint::include_modules!();

pub fn main(code: &str, from_app: &str) -> Result<(), slint::PlatformError> {
    let logger_config = ConfigBuilder::new().build();

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            logger_config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            logger_config.clone(),
            File::open(log_path()).unwrap(),
        ),
    ])
    .unwrap();

    let locale = get_sys_locale();
    rust_i18n::set_locale(locale);

    let paste_code_instruction = t!("paste_code_instruction");
    let verification_code_label = format!(
        "{}: {}\n{} {}",
        t!("verification-code"),
        &code,
        t!("from-label"),
        from_app
    );

    let mut backend = i_slint_backend_winit::Backend::new().unwrap();
    backend.window_builder_hook = Some(Box::new(|builder| {
        builder
            .with_titlebar_buttons_hidden(true)
            .with_titlebar_transparent(true)
            .with_title_hidden(true)
    }));
    slint::platform::set_platform(Box::new(backend)).unwrap();

    let ui = AppWindow::new()?;

    let ui_weak = ui.as_weak();

    ui.on_mouse_move(move |delta_x, delta_y| {
        let ui_weak = ui_weak.unwrap();
        let logical_pos = ui_weak.window().position();
        ui_weak.window().set_position(slint::PhysicalPosition::new(
            logical_pos.x + delta_x as i32,
            logical_pos.y + delta_y as i32,
        ));
    });

    // 添加关闭窗口回调的处理
    let ui_weak_close = ui.as_weak();
    ui.on_close_window(move || {
        let ui = ui_weak_close.unwrap();
        ui.hide().unwrap();
    });

    ui.set_paste_code_instruction(paste_code_instruction.to_string().into());
    ui.set_verification_code_label(verification_code_label.to_string().into());

    let position = Mouse::get_mouse_position();
    let mut mouse_pos = (0, 0);
    match position {
        Mouse::Position { x, y } => mouse_pos = (x, y),
        Mouse::Error => error!("error-getting-mouse-position"),
    }

    ui.window()
        .set_position(slint::PhysicalPosition::new(mouse_pos.0, mouse_pos.1));

    let ui_handle = ui.as_weak();
    let config = read_config();
    let mut clpb = Clipboard::new().unwrap();

    let captcha = String::from(code);

    ui.on_paste_code(move || {
        let ui = ui_handle.unwrap();
        let old_clpb_contents = get_old_clipboard_contents();

        clpb.set_text(captcha.as_str()).unwrap();
        match paste_script() {
            Ok(_) => info!("{}", t!("paste-verification-code")),
            Err(e) => error!("{}: {:?}", t!("error-paste-verification-code"), e),
        }
        if config.auto_return {
            match return_script() {
                Ok(_) => info!("{}", t!("press-enter")),
                Err(e) => error!("{}: {:?}", t!("error-press-enter"), e),
            }
        }
        if config.recover_clipboard {
            sleep(Duration::from_secs(2));
            recover_clipboard_contents(old_clpb_contents);
        }
        ui.hide().unwrap();
    });

    ui.run()
}
