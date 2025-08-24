use eframe::egui;
use egui::{FontId, Sense};
use rfd::FileDialog;
use std::path::PathBuf;
use std::fs::File;
use zip::ZipArchive;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

use crate::parser::Parser;
use crate::models::extracted_data::ExtractedData;

#[derive(Debug)]
enum ExtractionMessage {
    Progress(String),
    Complete(ExtractedData),
    Error(String),
}

#[derive(Default)]
pub struct App {
    zip_path: Option<PathBuf>,
    extracted_data: Option<ExtractedData>,
    is_loading: bool,
    error_message: Option<String>,
    progress_message: String,
    extraction_receiver: Option<Receiver<ExtractionMessage>>,
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
                    self.zip_path = Some(path.clone());
                    self.is_loading = true;
                    self.error_message = None;
                    self.progress_message = "Opening ZIP file...".to_string();
                    println!("Selected file: {:?}", path);

                    // Start extraction in background thread
                    self.start_extraction(path);
                }
            }
        });
    }

    fn start_extraction(&mut self, path: PathBuf) {
        let (sender, receiver) = channel::<ExtractionMessage>();
        self.extraction_receiver = Some(receiver);

        thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = sender.send(ExtractionMessage::Error(format!("Failed to create async runtime: {}", e)));
                    return;
                }
            };

            match File::open(&path) {
                Ok(file) => {
                    let _ = sender.send(ExtractionMessage::Progress("Reading ZIP archive...".to_string()));

                    match ZipArchive::new(file) {
                        Ok(archive) => {
                            let _ = sender.send(ExtractionMessage::Progress("Analyzing package structure...".to_string()));

                            let mut parser = Parser::new();

                            let progress_callback = |msg: String| {
                                let _ = sender.send(ExtractionMessage::Progress(msg));
                            };

                            match rt.block_on(parser.extract_data(archive, progress_callback)) {
                                Ok(data) => {
                                    let _ = sender.send(ExtractionMessage::Complete(data));
                                }
                                Err(e) => {
                                    let _ = sender.send(ExtractionMessage::Error(format!("Extraction error: {}", e)));
                                }
                            }
                        }
                        Err(e) => {
                            let _ = sender.send(ExtractionMessage::Error(format!("Failed to open ZIP archive: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = sender.send(ExtractionMessage::Error(format!("Failed to open file: {}", e)));
                }
            }
        });
    }

    fn check_extraction_progress(&mut self, ctx: &egui::Context) {
        let mut should_remove_receiver = false;
        let mut messages_received = false;

        if let Some(receiver) = &self.extraction_receiver {
            while let Ok(message) = receiver.try_recv() {
                messages_received = true;
                match message {
                    ExtractionMessage::Progress(msg) => {
                        self.progress_message = msg;
                    }
                    ExtractionMessage::Complete(data) => {
                        self.extracted_data = Some(data);
                        self.is_loading = false;
                        should_remove_receiver = true;
                        println!("Extraction completed successfully!");
                    }
                    ExtractionMessage::Error(error) => {
                        self.error_message = Some(error);
                        self.is_loading = false;
                        should_remove_receiver = true;
                    }
                }
            }
        }

        if should_remove_receiver {
            self.extraction_receiver = None;
        }

        if messages_received {
            ctx.request_repaint();
        }
    }

    fn ui_loading(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(200.0);

                ui.spinner();
                ui.add_space(20.0);

                ui.label(egui::RichText::new("Loading Discord Package...")
                    .size(20.0));

                ui.add_space(10.0);

                ui.label(egui::RichText::new(&self.progress_message)
                    .size(14.0)
                    .color(egui::Color32::GRAY));

                ui.add_space(20.0);
            });
        });
    }

    fn ui_error(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            let (rect, _response) = ui.allocate_exact_size(available, Sense::hover());

            if let Some(error) = &self.error_message {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &format!("Error: {}\n\nClick anywhere to try again", error),
                    FontId::proportional(16.0),
                    egui::Color32::RED,
                );

                if ui.input(|i| i.pointer.any_click()) {
                    self.zip_path = None;
                    self.error_message = None;
                }
            }
        });
    }

    fn ui_main_view(&mut self, ctx: &egui::Context) {
        if self.extracted_data.is_some() {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let data = match self.extracted_data {
                        Some(ref data) => data,
                        None => return,
                    };
                    ui.heading("Discord Package Analysis");
                    ui.separator();

                    // User Information
                    if let Some(user) = &data.user {
                        ui.heading("User Information");
                        let tex = user.avatar.as_ref().map(|avatar_data| {
                            let dyn_img = image::load_from_memory(avatar_data).expect("Failed to decode avatar image");
                            let size = [dyn_img.width() as usize, dyn_img.height() as usize];
                            let rgba = dyn_img.to_rgba8().into_vec();
                            let image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                            ui.ctx().load_texture("user_avatar", image, Default::default())
                        });

                        if let Some(texture) = tex {
                            ui.add(egui::Image::new(&texture));
                        } else if let Some(default_url) = &user.default_avatar_url {
                            ui.image(default_url);
                        }
                        ui.label(format!("Username: {}#{}", user.username, user.discriminator));
                        ui.label(format!("User ID: {}", user.id));
                        ui.separator();
                    }

                    // Statistics
                    ui.heading("Statistics");
                    ui.label(format!("Total Messages: {}", data.message_count));
                    ui.label(format!("Total Characters: {}", data.character_count));
                    ui.label(format!("Guild Count: {}", data.guild_count));
                    ui.label(format!("DM Channels: {}", data.dm_channel_count));
                    ui.label(format!("Server Channels: {}", data.channel_count));
                    ui.separator();

                    // Top Channels
                    ui.heading("Top Channels");
                    for (i, channel) in data.top_channels.iter().enumerate() {
                        ui.label(format!("{}. {} - {} messages",
                            i + 1,
                            channel.name,
                            channel.message_count
                        ));
                        if let Some(guild_name) = &channel.guild_name {
                            ui.label(format!("   Server: {}", guild_name));
                        }
                    }
                    ui.separator();

                    // Top DMs
                    ui.heading("Top Direct Messages");
                    for (i, dm) in data.top_dms.iter().enumerate() {
                        ui.label(format!("{}. User ID {} - {} messages",
                            i + 1,
                            dm.dm_user_id,
                            dm.message_count
                        ));
                    }
                    ui.separator();

                    // Favorite Words
                    ui.heading("Favorite Words");
                    for (i, word) in data.favorite_words.iter().enumerate() {
                        ui.label(format!("{}. {} (used {} times)",
                            i + 1,
                            word.word,
                            word.count
                        ));
                    }
                    ui.separator();

                    // Hourly Activity
                    ui.heading("Activity by Hour");
                    for (hour, count) in data.hours_values.iter().enumerate() {
                        if *count > 0 {
                            ui.label(format!("{}:00 - {} messages", hour, count));
                        }
                    }

                    ui.separator();
                    if ui.button("Load Another Package").clicked() {
                        self.zip_path = None;
                        self.extracted_data = None;
                        self.error_message = None;
                    }
                });
            });
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        self.check_extraction_progress(ctx);

        if self.is_loading {
            self.ui_loading(ctx);
        } else if self.error_message.is_some() {
            self.ui_error(ctx);
        } else if self.extracted_data.is_some() {
            self.ui_main_view(ctx);
        } else {
            self.ui_file_prompt(ctx);
        }
    }
}
