use egui::{Color32, TextureHandle, Vec2, Widget};

use crate::core::cpu::Cpu;

pub struct Screen {
    pub cpu: Cpu,
    pub texture: TextureHandle,
}

impl Screen {
    pub fn new(cpu: Cpu, texture: egui::TextureHandle) -> Self {
        Screen { cpu, texture }
    }
    pub fn frame(&mut self) -> anyhow::Result<Vec<Color32>> {
        for i in 0..70224 {
            // is this right?
            self.cpu.ppu.clock(&mut self.cpu.mmu)?;
            self.cpu.ppu.clock(&mut self.cpu.mmu)?;
            self.cpu.ppu.clock(&mut self.cpu.mmu)?;
            self.cpu.ppu.clock(&mut self.cpu.mmu)?;
            self.cpu.cycle()?;
        }
        let f = self
            .cpu
            .ppu
            .frame(&mut self.cpu.mmu)?
            .iter()
            .map(|x| match x {
                0 => Color32::from_gray(0),
                1 => Color32::from_gray(86),
                2 => Color32::from_gray(172),
                3 => Color32::from_gray(255),
                _ => unreachable!(),
            })
            .collect();
        Ok(f)
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.ctx().request_repaint();
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
        ui.add(egui::Image::new(sized));
    }
}
