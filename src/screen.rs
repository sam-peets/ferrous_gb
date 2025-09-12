use egui::{Color32, Key, TextureHandle, Vec2, Widget};

use crate::core::{Buttons, cpu::Cpu};

pub struct Screen {
    pub cpu: Cpu,
    pub texture: TextureHandle,
    pub last_frame: u128,
}

impl Screen {
    pub fn new(cpu: Cpu, texture: egui::TextureHandle) -> Self {
        Screen {
            cpu,
            texture,
            last_frame: 0,
        }
    }
    pub fn frame(&mut self) -> anyhow::Result<Vec<Color32>> {
        for i in 0..(70224 / 4) {
            // is this right?
            self.cpu.ppu.clock(&mut self.cpu.mmu)?;
            self.cpu.ppu.clock(&mut self.cpu.mmu)?;
            self.cpu.ppu.clock(&mut self.cpu.mmu)?;
            self.cpu.ppu.clock(&mut self.cpu.mmu)?;
            self.cpu.cycle(i)?;
        }
        let f = self
            .cpu
            .ppu
            .frame(&mut self.cpu.mmu)?
            .iter()
            .map(|x| match x {
                3 => Color32::from_gray(0),
                2 => Color32::from_gray(86),
                1 => Color32::from_gray(172),
                0 => Color32::from_gray(255),
                _ => unreachable!(),
            })
            .collect();
        Ok(f)
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.ctx().request_repaint();
        let buttons = ui.input(|i| Buttons {
            up: i.key_down(Key::ArrowUp),
            down: i.key_down(Key::ArrowDown),
            left: i.key_down(Key::ArrowLeft),
            right: i.key_down(Key::ArrowRight),
            start: i.key_down(Key::Enter),
            select: i.key_down(Key::Space),
            a: i.key_down(Key::A),
            b: i.key_down(Key::B),
        });
        self.cpu.mmu.buttons = buttons;
        // TODO: Joypad interrupt

        let frame = match self.frame() {
            Ok(x) => x,
            Err(e) => {
                log::info!("screen: crashed after {} cycles", self.cpu.cycles);
                panic!("error: {e}");
            }
        };
        self.texture.set(
            egui::ColorImage {
                size: [160, 144],
                source_size: Vec2::new(160.0, 144.0),
                pixels: frame,
            },
            egui::TextureOptions::NEAREST,
        );
        let sized = egui::load::SizedTexture::from_handle(&self.texture);
        ui.add(egui::Image::new(sized).fit_to_exact_size(4.0 * Vec2::new(160.0, 144.0)));
        ui.checkbox(&mut self.cpu.logging, "logging enabled");
        ui.label(format!("frame time: {}ms", self.last_frame));
    }
}
