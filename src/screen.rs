use std::sync::{Arc, RwLock};

use cpal::{
    FromSample, SizedSample, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use egui::{Color32, Key, TextureHandle, Vec2};

use crate::core::{Buttons, cpu::Cpu};

#[derive(Default)]
pub struct Debugger {
    pub show_vram: bool,
}

pub type ApuSamples = Arc<RwLock<Vec<f32>>>;

pub struct Screen {
    pub cpu: Cpu,
    pub screen_texture: TextureHandle,
    pub vram_texture: TextureHandle,
    pub last_frame: u128,
    pub handle: Option<Handle>,
    pub debugger: Debugger,
}

impl Screen {
    pub fn new(cpu: Cpu, ctx: &egui::Context) -> Self {
        let handle = Some(beep(cpu.mmu.apu.cur_sample.clone()));
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
            screen_texture,
            vram_texture,
            last_frame: 0,
            handle,
            debugger: Debugger::default(),
        }
    }

    pub fn frame(&mut self) -> anyhow::Result<Vec<Color32>> {
        for _ in 0..70224 {
            // is this right?
            self.cpu.cycle()?;
            self.cpu.mmu.apu.clock(self.cpu.mmu.sys);
            self.cpu.ppu.clock(&mut self.cpu.mmu)?;

            self.cpu.mmu.sys = self.cpu.mmu.sys.wrapping_add(1);
            // TODO: add a way to look at falling edges on sys/div
            // off the top of my head, APU needs it, timer needs it...
        }
        let f = self
            .cpu
            .ppu
            .frame(&mut self.cpu.mmu)?
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

    fn vram_debug_frame(&mut self) -> anyhow::Result<Vec<Color32>> {
        let v = self.cpu.ppu.dump_vram(&mut self.cpu.mmu)?;
        let f = v
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
        self.screen_texture.set(
            egui::ColorImage {
                size: [160, 144],
                source_size: Vec2::new(160.0, 144.0),
                pixels: frame,
            },
            egui::TextureOptions::NEAREST,
        );
        let sized = egui::load::SizedTexture::from_handle(&self.screen_texture);
        let max_size = 2.0 * Vec2::new(160.0, 144.0);
        let min_size = ui.available_size();
        let target_size = min_size.min(max_size);
        ui.add(egui::Image::new(sized).fit_to_exact_size(target_size));
        ui.checkbox(&mut self.cpu.logging, "logging enabled");
        ui.label(format!("frame time: {}ms", self.last_frame));
        ui.label(format!("lcdc: 0b{:08b}", self.cpu.mmu.io.lcdc));

        if self.debugger.show_vram {
            egui::Window::new("VRAM").show(ui.ctx(), |ui| {
                let debug_frame = self.vram_debug_frame().unwrap();
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

pub struct Handle(Stream);

pub fn beep(data: ApuSamples) -> Handle {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();

    Handle(match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), data),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), data),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), data),
        // not all supported sample formats are included in this example
        _ => panic!("Unsupported sample format!"),
    })
}

fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig, apu_data: ApuSamples) -> Stream
where
    T: SizedSample + FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _| write_data(data, channels, apu_data.clone()),
            |x| log::error!("stream error: {x:?}"),
            None,
        )
        .unwrap();
    stream.play().unwrap();
    stream
}

fn write_data<T>(output: &mut [T], channels: usize, samples: ApuSamples)
where
    T: SizedSample + FromSample<f32>,
{
    let samples = {
        let x = samples.read().expect("Screen: couldn't unlock samples");
        (*x).clone()
    };
    let chunks = output.chunks_mut(channels);
    let chunks_len = chunks.len();
    let samples_len = samples.len();
    let samples_time = samples_len as f32 / 1048576.0;
    log::info!(
        "Screen: write_data: chunks len: {}, samples len: {}, samples time: {}",
        chunks_len,
        samples_len,
        samples_time
    );

    for (i, frame) in chunks.enumerate() {
        let lerp = ((i as f32 / chunks_len as f32) * samples_len as f32).floor() as usize;
        let sample = if samples.is_empty() {
            0.0
        } else {
            samples[lerp]
        };
        let value: T = T::from_sample(sample);

        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
