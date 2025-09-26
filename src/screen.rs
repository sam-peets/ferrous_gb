use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    sync::{Arc, RwLock},
};

use cpal::{
    FromSample, SizedSample, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use egui::{Color32, Key, TextureHandle, Vec2};

use crate::{
    audio_handle::Handle,
    client_config::ClientConfigShared,
    core::{Buttons, cpu::Cpu, ppu::Ppu},
};

#[derive(Default)]
pub struct Debugger {
    pub show_vram: bool,
}

pub const MAX_AUDIO_BUFFER: usize = 2048;
pub type ApuSamples = Arc<RwLock<VecDeque<(f32, f32)>>>;

pub struct Screen {
    pub cpu: Cpu,
    pub texture: TextureHandle,
    pub vram_texture: TextureHandle,
    pub handle: Option<Handle>,
    pub debugger: Debugger,
    pub config: ClientConfigShared,
}

impl Screen {
    pub fn new(cpu: Cpu, config: ClientConfigShared, ctx: &egui::Context) -> Self {
        let handle = Some(Handle::new(
            cpu.mmu.mmio.apu.cur_sample.clone(),
            config.clone(),
        ));
        let screen_texture = ctx.load_texture(
            "screen",
            egui::ColorImage::filled([160, 144], Color32::BLACK),
            egui::TextureOptions::NEAREST,
        );
        let vram_texture = ctx.load_texture(
            "vram_debug",
            egui::ColorImage::filled([128, 64 * 4], Color32::BLACK),
            egui::TextureOptions::NEAREST,
        );
        Screen {
            cpu,
            texture: screen_texture,
            vram_texture,
            handle,
            debugger: Debugger::default(),
            config,
        }
    }

    pub fn frame(&mut self) -> anyhow::Result<Vec<Color32>> {
        // sync the cpu to the audio
        if self.cpu.mmu.mmio.apu.cur_sample.read().unwrap().len() <= MAX_AUDIO_BUFFER {
            for _ in 0..70224 {
                // is this right?
                self.cpu.cycle()?;
                self.cpu.mmu.mmio.apu.clock(self.cpu.mmu.mmio.sys);
                self.cpu.mmu.mmio.ppu.clock();

                self.cpu.mmu.mmio.sys = self.cpu.mmu.mmio.sys.wrapping_add(1);
                // TODO: add a way to look at falling edges on sys/div
                // off the top of my head, APU needs it, timer needs it...
            }
        }

        let f = self
            .cpu
            .mmu
            .mmio
            .ppu
            .frame()
            .iter()
            .map(|x| match x {
                0 => Color32::from_hex("#e0f8d0").unwrap(),
                1 => Color32::from_hex("#88c070").unwrap(),
                2 => Color32::from_hex("#346856").unwrap(),
                3 => Color32::from_hex("#081820").unwrap(),
                _ => unreachable!(),
            })
            .collect();
        Ok(f)
    }

    fn vram_debug_frame(&mut self) -> Vec<Color32> {
        let v = Ppu::dump_vram(&mut self.cpu.mmu);

        v.iter()
            .map(|x| match x {
                0 => Color32::from_hex("#e0f8d0").unwrap(),
                1 => Color32::from_hex("#88c070").unwrap(),
                2 => Color32::from_hex("#346856").unwrap(),
                3 => Color32::from_hex("#081820").unwrap(),
                _ => unreachable!(),
            })
            .collect()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.ctx().request_repaint();
        let buttons = ui.input(|i| Buttons {
            up: i.key_down(Key::ArrowUp).into(),
            down: i.key_down(Key::ArrowDown).into(),
            left: i.key_down(Key::ArrowLeft).into(),
            right: i.key_down(Key::ArrowRight).into(),
            start: i.key_down(Key::Enter).into(),
            select: i.key_down(Key::Space).into(),
            a: i.key_down(Key::A).into(),
            b: i.key_down(Key::B).into(),
        });
        self.cpu.mmu.mmio.buttons = buttons;
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
        let max_size = 2.0 * Vec2::new(160.0, 144.0);
        let min_size = ui.available_size();
        let target_size = min_size.min(max_size);
        ui.add(egui::Image::new(sized).fit_to_exact_size(target_size));

        if self.debugger.show_vram {
            egui::Window::new("VRAM").show(ui.ctx(), |ui| {
                let debug_frame = self.vram_debug_frame();
                self.vram_texture.set(
                    egui::ColorImage {
                        size: [128, 64 * 4],
                        source_size: Vec2::new(128.0, 64.0 * 4.0),
                        pixels: debug_frame,
                    },
                    egui::TextureOptions::NEAREST,
                );
                let sized = egui::load::SizedTexture::from_handle(&self.vram_texture);
                ui.add(egui::Image::new(sized));
            });
        }
    }
}
