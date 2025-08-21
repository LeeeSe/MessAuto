use crate::{clipboard::auto_paste, clipboard::press_enter, config::Config};
use eframe::{
    App, NativeOptions,
    egui::{self, vec2},
};
use rust_i18n::t;
use std::time::{Duration, Instant};
use winit::platform::macos::EventLoopBuilderExtMacOS;

const WINDOW_SIZE: egui::Vec2 = egui::Vec2::new(140.0, 110.0);
const CLOSE_BUTTON_SIZE: f32 = 12.0;
const CLOSE_BUTTON_OFFSET: egui::Vec2 = egui::Vec2::new(-4.0, -4.0);
const CONTENT_OFFSET: egui::Vec2 = egui::Vec2::new(2.0, 2.0);

pub struct VerificationCodeApp {
    code: String,
    source: String,
    created_at: Instant,
    lifetime: Duration,
    should_close: bool,
}

impl VerificationCodeApp {
    pub fn new(code: String, source: String) -> Self {
        Self {
            code,
            source,
            created_at: Instant::now(),
            lifetime: Duration::from_secs(600),
            should_close: false,
        }
    }

    pub fn run(code: String, source: String) {
        let options = NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(WINDOW_SIZE)
                .with_resizable(false)
                .with_titlebar_shown(false)
                .with_titlebar_buttons_shown(false)
                .with_fullsize_content_view(true)
                .with_title_shown(false)
                .with_always_on_top(),
            event_loop_builder: Some(Box::new(|builder| {
                builder
                    .with_activation_policy(winit::platform::macos::ActivationPolicy::Prohibited);
            })),
            ..Default::default()
        };

        eframe::run_native(
            "VerificationCode",
            options,
            Box::new(|cc| {
                let mut fonts = egui::FontDefinitions::default();

                fonts.font_data.insert(
                    "PingFang SC".to_owned(),
                    std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                        "../../resources/PingFang-SC-Regular.ttf"
                    ))),
                );

                fonts
                    .families
                    .get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "PingFang SC".to_owned());

                fonts
                    .families
                    .get_mut(&egui::FontFamily::Monospace)
                    .unwrap()
                    .insert(0, "PingFang SC".to_owned());

                cc.egui_ctx.set_fonts(fonts);
                Ok(Box::new(Self::new(code, source)))
            }),
        )
        .unwrap();
    }

    fn handle_window_drag(&self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let response = ui.interact(
            ui.max_rect(),
            ui.id().with("drag_window"),
            egui::Sense::drag(),
        );
        if response.dragged() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
    }

    fn draw_close_button(&self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let close_btn_pos = ui.max_rect().left_top() + CLOSE_BUTTON_OFFSET;
        let close_btn_rect = egui::Rect::from_center_size(
            close_btn_pos + vec2(CLOSE_BUTTON_SIZE / 2.0, CLOSE_BUTTON_SIZE / 2.0),
            vec2(CLOSE_BUTTON_SIZE, CLOSE_BUTTON_SIZE),
        );
        let close_btn_response = ui.allocate_rect(close_btn_rect, egui::Sense::click());

        let button_color = if close_btn_response.is_pointer_button_down_on() {
            egui::Color32::from_rgb(220, 90, 80)
        } else {
            egui::Color32::from_rgb(237, 106, 94)
        };

        ui.painter().circle_filled(
            close_btn_rect.center(),
            CLOSE_BUTTON_SIZE / 2.0,
            button_color,
        );

        if close_btn_response.hovered() {
            let text = "❌";
            let font = egui::FontId::proportional(CLOSE_BUTTON_SIZE * 0.9);
            let color = egui::Color32::from_rgb(152, 0, 0);

            let galley = ui
                .painter()
                .layout_no_wrap(text.to_owned(), font.clone(), color);

            let text_rect = egui::Rect::from_center_size(close_btn_rect.center(), galley.size());

            ui.painter().text(
                text_rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                font,
                color,
            );
        }

        if close_btn_response.clicked() {
            let ctx_clone = ctx.clone();
            std::thread::spawn(move || {
                ctx_clone.send_viewport_cmd(egui::ViewportCommand::Close);
            });
        }
    }

    fn draw_content(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let content_area = ui.max_rect().translate(CONTENT_OFFSET);
        let mut content_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(content_area)
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );

        content_ui.add_space(5.0);
        content_ui.add(egui::Label::new(t!("floating_window.click_input_box")).selectable(false));
        content_ui
            .add(egui::Label::new(t!("floating_window.click_button_below")).selectable(false));

        let btn_response = self.custom_button(
            &mut content_ui,
            &format!(
                "{}\n{}",
                t!("floating_window.code", code = self.code),
                t!("floating_window.from", source = self.source)
            ),
        );

        if btn_response.clicked() {
            let _ = auto_paste(true, &self.code);

            if let Ok(config) = Config::load() {
                if config.auto_enter {
                    if let Err(e) = press_enter() {
                        log::error!(
                            "{}",
                            t!("monitor.failed_to_press_enter_floating", error = e)
                        );
                    } else {
                        log::info!("{}", t!("monitor.auto_pressed_enter_floating"));
                    }
                }
            }

            self.should_close = true;
        }
    }

    fn custom_button(&self, ui: &mut egui::Ui, text: &str) -> egui::Response {
        let available_size = ui.available_size();
        let button_size = vec2(available_size.x - 5.0, available_size.y - 2.0);

        let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());

        let bg_color = if response.is_pointer_button_down_on() {
            if ui.visuals().dark_mode {
                egui::Color32::from_rgb(0x3C, 0x3C, 0x3C)
            } else {
                egui::Color32::from_rgb(0xE6, 0xE6, 0xE6)
            }
        } else if response.hovered() {
            // 悬停时的颜色
            if ui.visuals().dark_mode {
                egui::Color32::from_rgb(0x45, 0x45, 0x45)
            } else {
                egui::Color32::from_rgb(0xD8, 0xD8, 0xD8)
            }
        } else {
            // 正常状态的颜色
            if ui.visuals().dark_mode {
                egui::Color32::from_rgb(0x3C, 0x3C, 0x3C)
            } else {
                egui::Color32::from_rgb(0xE6, 0xE6, 0xE6)
            }
        };

        ui.painter().rect_filled(rect, 6.0, bg_color);

        let text_color = if ui.visuals().dark_mode {
            egui::Color32::WHITE
        } else {
            egui::Color32::BLACK
        };

        let lines: Vec<&str> = text.split('\n').collect();
        let font_id = egui::FontId::proportional(12.0);

        let line_height = 18.0;
        let total_height = line_height * lines.len() as f32;

        let first_line_y = rect.center().y - (total_height / 2.0) + (line_height / 2.0);

        for (i, line) in lines.iter().enumerate() {
            let y_pos = first_line_y + i as f32 * line_height;
            ui.painter().text(
                egui::pos2(rect.center().x, y_pos),
                egui::Align2::CENTER_CENTER,
                line,
                font_id.clone(),
                text_color,
            );
        }

        response
    }
}

impl App for VerificationCodeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.should_close || self.created_at.elapsed() > self.lifetime {
            let ctx_clone = ctx.clone();
            std::thread::spawn(move || {
                ctx_clone.send_viewport_cmd(egui::ViewportCommand::Close);
            });
            return;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.handle_window_drag(ui, ctx);
            self.draw_close_button(ui, ctx);
            self.draw_content(ui, ctx);
        });
    }
}
