use egui::{Color32, TextureHandle, Widget};
use poll_promise::Promise;

use crate::{core::cpu::Cpu, screen::Screen};

pub struct TemplateApp {
    promise: Option<Promise<Option<Vec<u8>>>>,
    screen: Option<Screen>,
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        TemplateApp {
            promise: None,
            screen: None,
        }
    }
}

impl eframe::App for TemplateApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(promise) = &self.promise {
            if let Some(Some(rom)) = promise.ready() {
                self.screen = Some(Screen {
                    cpu: Cpu::new(rom.clone()),
                    texture: ctx.load_texture(
                        "screen",
                        egui::ColorImage::filled([160, 144], Color32::BLACK),
                        egui::TextureOptions::NEAREST,
                    ),
                });
                self.promise = None;
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                if ui.button("Hello").clicked() {
                    self.promise = Some(poll_promise::Promise::spawn_local(async {
                        if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                            let f = file.read().await;
                            Some(f)
                        } else {
                            None
                        }
                    }));
                }
            })
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("hello vro </3");
            if let Some(screen) = &mut self.screen {
                screen.ui(ui);
            }
        });
    }
}
