use cpal::traits::{DeviceTrait, HostTrait};
use egui::{Context, Ui};
use poll_promise::Promise;

use crate::{core::cpu::Cpu, screen::Screen};

pub struct GbApp {
    promise: Option<Promise<Option<Vec<u8>>>>,
    screen: Option<Screen>,
    about_window: AboutWindow,
}

impl GbApp {
    pub fn new(_: &eframe::CreationContext<'_>) -> Self {
        GbApp {
            promise: None,
            screen: None,
            about_window: Default::default(),
        }
    }
}

const TOBU: &[u8] = include_bytes!("../assets/roms/tobu.gb");

impl GbApp {
    fn draw_menubar(&mut self, ui: &mut Ui) {
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
                ui.menu_button("Load Example", |ui| {
                    if ui.button("Tobu Tobu Girl").clicked() {
                        self.promise = Some(poll_promise::Promise::from_ready(Some(TOBU.to_vec())))
                    }
                });
                ui.separator();
                if ui.button("About").clicked() {
                    self.about_window.open = true;
                }
            });
            if let Some(screen) = &mut self.screen {
                ui.menu_button("Debug", |ui| {
                    ui.checkbox(&mut screen.debugger.show_vram, "Show VRAM");
                    ui.checkbox(&mut screen.cpu.logging, "CPU Tracing");
                });
            }
        });
    }
}

impl eframe::App for GbApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(promise) = &self.promise {
            if let Some(Some(rom)) = promise.ready() {
                let host = cpal::default_host();
                let device = host
                    .default_output_device()
                    .expect("failed to find a default output device");
                let config = device.default_output_config().unwrap();
                let sample_rate = config.sample_rate().0;
                self.screen = Some(Screen::new(
                    Cpu::new(rom.clone(), sample_rate).unwrap(),
                    ctx,
                ));
                self.promise = None;
            }
        }
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.draw_menubar(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(screen) = &mut self.screen {
                ui.vertical_centered(|ui| {
                    screen.ui(ui);
                });
            }
        });
        self.about_window.show(ctx);
        // request a repaint to avoid egui repaint behaviour
        // TODO: is there a better way around this? maybe...
        ctx.request_repaint();
    }
}

#[derive(Default)]
struct AboutWindow {
    open: bool,
}

impl AboutWindow {
    fn show(&mut self, ctx: &Context) {
        egui::Window::new("About")
            .open(&mut self.open)
            .show(ctx, |ui| {
                ui.label("Ferrous GB is a WIP Gameboy emulator built in Rust targeting the web (through WASM) and native platforms.");
                ui.label("This software open source and licensed under the MIT license");
                ui.hyperlink_to("Source Code (github.com)", "https://github.com/sam-peets/ferrous_gb/");
                ui.separator();
                ui.label("Bootix (CC0-1.0) is included as an open-source bootrom.");
                ui.hyperlink("https://github.com/Ashiepaws/Bootix");
                ui.label("Tobu Tobu Girl (MIT/CC-BY 4.0, © 2017 Tangram Games) is included as an open-source example game.");
                ui.hyperlink("https://github.com/SimonLarsen/tobutobugirl")
            });
    }
}
