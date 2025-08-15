use eframe::egui;
use egui::{FontId, Sense};
use rfd::FileDialog;

#[derive(Default)]
pub struct App {
    zip_path: Option<String>,
}

impl App {
    fn ui_file_prompt(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let (rect, response) = ui.allocate_exact_size(available, Sense::click());

            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Click to select Discord Package ZIP file",
                FontId::proportional(20.0),
                ui.visuals().text_color(),
            );

            if response.clicked() {
                if let Some(path) = FileDialog::new()
                    .add_filter("ZIP files", &["zip"])
                    .pick_file()
                {
                    self.zip_path = Some(path.display().to_string());
                    println!("Selected file: {:?}", self.zip_path);
                }
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        if !self.zip_path.is_some() {
            self.ui_file_prompt(ctx);
        } else {
            // TODO: Main application UI after file is loaded
        }
    }
}
