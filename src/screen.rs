use cpal::{
    FromSample, SizedSample, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use egui::{Color32, Key, TextureHandle, Vec2, Widget};

use crate::core::{Buttons, cpu::Cpu};

pub struct Screen {
    pub cpu: Cpu,
    pub texture: TextureHandle,
    pub last_frame: u128,
    pub handle: Option<Handle>,
}

impl Screen {
    pub fn new(cpu: Cpu, texture: egui::TextureHandle) -> Self {
        Screen {
            cpu,
            texture,
            last_frame: 0,
            handle: None,
        }
    }
    pub fn frame(&mut self) -> anyhow::Result<Vec<Color32>> {
        let mut sys = 0u16;
        for i in 0..70224 {
            // is this right?
            self.cpu.ppu.clock(&mut self.cpu.mmu)?;
            self.cpu.cycle(sys)?;
            sys = sys.overflowing_add(1).0;
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
        let max_size = 2.0 * Vec2::new(160.0, 144.0);
        let min_size = ui.available_size();
        let target_size = min_size.min(max_size);
        ui.add(egui::Image::new(sized).fit_to_exact_size(target_size));
        ui.checkbox(&mut self.cpu.logging, "logging enabled");
        ui.label(format!("frame time: {}ms", self.last_frame));

        if ui.button("audio test").clicked() {
            self.handle = Some(beep());
        }
    }
}

pub struct Handle(Stream);

pub fn beep() -> Handle {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();

    Handle(match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into()),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into()),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into()),
        // not all supported sample formats are included in this example
        _ => panic!("Unsupported sample format!"),
    })
}

fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Stream
where
    T: SizedSample + FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    // Produce a sinusoid of maximum amplitude.
    let mut sample_clock = 0f32;
    let mut next_value = move || {
        sample_clock = (sample_clock + 1.0) % sample_rate;
        (sample_clock * 440.0 * 2.0 * 3.141592 / sample_rate).sin()
    };

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _| write_data(data, channels, &mut next_value),
            |x| log::error!("stream error: {x:?}"),
            None,
        )
        .unwrap();
    stream.play().unwrap();
    stream
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> f32)
where
    T: SizedSample + FromSample<f32>,
{
    for frame in output.chunks_mut(channels) {
        let value: T = T::from_sample(next_sample());
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
