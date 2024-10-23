use eframe::egui;
use chrono::Local;

struct MyApp {
    show_time: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            show_time: false,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Центрируем содержимое окна.
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Настройка размера кнопки.
                let size = egui::Vec2::splat(60.0); // Размер кнопки.
                // Создаем кнопку.
                if ui.add_sized(size, egui::Button::new("Time")).clicked() {
                    self.show_time = true;
                }
                // Отображаем текущее время после нажатия на кнопку.
                if self.show_time {
                    let now = Local::now();
                    ui.label(now.format("%H:%M:%S").to_string());
                }
            });
        });
        ctx.request_repaint();
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Окно с кнопкой",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    ).expect("TODO: panic message");
}