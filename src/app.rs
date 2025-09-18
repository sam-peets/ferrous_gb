use std::collections::BTreeMap;

use egui::{Color32, FontData, FontDefinitions, FontFamily, TextureHandle, Widget};
use poll_promise::Promise;

use crate::{
    core::cpu::Cpu,
    screen::{self, Screen},
};

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
                self.screen = Some(Screen::new(Cpu::new(rom.clone()).unwrap(), ctx));
                self.promise = None;
            }
        }
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open ROM").clicked() {
                        self.promise = Some(poll_promise::Promise::spawn_local(async {
                            if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                                let f = file.read().await;
                                Some(f)
                            } else {
                                None
                            }
                        }));
                    }
                    ui.menu_button("Load Example", |ui| {})
                })
            })
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(screen) = &mut self.screen {
                screen.ui(ui);
            }
        });

        // request a repaint to avoid egui repaint behaviour
        // TODO: is there a better way around this? maybe...
        ctx.request_repaint();
    }
}
