use cpal::traits::{DeviceTrait, HostTrait};
use poll_promise::Promise;

use crate::{core::cpu::Cpu, screen::Screen};

pub struct TemplateApp {
    promise: Option<Promise<Option<Vec<u8>>>>,
    screen: Option<Screen>,
}

impl TemplateApp {
    pub fn new(_: &eframe::CreationContext<'_>) -> Self {
        TemplateApp {
            promise: None,
            screen: None,
        }
    }
}

const TOBU: &[u8] = include_bytes!("../assets/roms/tobu.gb");

impl eframe::App for TemplateApp {
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
                    Cpu::new_fastboot(rom.clone(), sample_rate).unwrap(),
                    ctx,
                ));
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
                    ui.menu_button("Load Example", |ui| {
                        if ui.button("Tobu Tobu Girl").clicked() {
                            self.promise =
                                Some(poll_promise::Promise::from_ready(Some(TOBU.to_vec())))
                        }
                    })
                });
                if let Some(screen) = &mut self.screen {
                    ui.menu_button("Debug", |ui| {
                        ui.checkbox(&mut screen.debugger.show_vram, "Show VRAM");
                    });
                }
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
