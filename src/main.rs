mod app;
mod models;
mod parser;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Discord Package Viewer",
        options,
        Box::new(|_cc| {
            Ok(Box::<app::App>::default())
        }
    ))
}
