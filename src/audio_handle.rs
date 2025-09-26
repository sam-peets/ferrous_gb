use cpal::{
    FromSample, SizedSample, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::{
    client_config::{self, ClientConfigShared},
    screen::ApuSamples,
};

pub struct Handle {
    stream: Stream,
}

impl Handle {
    pub fn new(data: ApuSamples, client_config: ClientConfigShared) -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("failed to find a default output device");
        let config = device.default_output_config().unwrap();
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), data, client_config),
            cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), data, client_config),
            cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), data, client_config),
            // not all supported sample formats are included in this example
            _ => panic!("Unsupported sample format!"),
        };

        Self { stream }
    }
}

fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    apu_data: ApuSamples,
    client_config: ClientConfigShared,
) -> Stream
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _| write_data(data, channels, &apu_data, &client_config),
            |x| log::error!("stream error: {x:?}"),
            None,
        )
        .unwrap();
    stream.play().unwrap();
    stream
}

fn write_data<T>(
    output: &mut [T],
    channels: usize,
    samples: &ApuSamples,
    client_config: &ClientConfigShared,
) where
    T: SizedSample + FromSample<f32>,
{
    let mut samples = samples.write().expect("Screen: couldn't lock samples");
    let volume = {
        client_config
            .read()
            .expect("Screen: couldn't lock client_config")
            .volume
    };

    let chunks = output.chunks_mut(channels);

    for frame in chunks {
        let (sample_left, sample_right) = samples.pop_front().unwrap_or_default();
        let sample_left = sample_left * volume;
        let sample_right = sample_right * volume;

        if channels == 2 {
            let value_left: T = T::from_sample(sample_left);
            let value_right: T = T::from_sample(sample_right);
            frame[0] = value_left;
            frame[1] = value_right;
        } else {
            let value: T = T::from_sample(f32::midpoint(sample_left, sample_right));
            for sample in frame.iter_mut() {
                *sample = value;
            }
        }
    }
}
