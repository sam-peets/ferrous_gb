use crate::core::{
    apu::{Channel, length::Length},
    util::extract,
};

#[derive(Debug, Default)]
pub struct Ch3 {
    // NR30
    pub dac_enabled: bool,

    // NR32
    initial_volume: u8,

    // NR32 & NR33
    period: u16,

    pub enabled: bool,
    volume: u8,
    length: Length<u16>,
    sample_index: usize,

    wave: [u8; 16], // wave pattern RAM (0xff30-ff3f)
    timer: u16,
}

impl Ch3 {}

impl Channel for Ch3 {
    fn clock(&mut self, div_apu: u8) {
        if div_apu % 2 == 0 && self.length.clock() {
            self.enabled = false;
        }
    }

    fn clear(&mut self) {
        *self = Ch3 {
            length: self.length,
            wave: self.wave,
            ..Default::default()
        };
        self.length.clear();
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff1a => {
                let dac_enabled = if self.dac_enabled { 1 << 7 } else { 0 };
                dac_enabled | 0b0111_1111
            }
            0xff1c => {
                let initial_volume = self.initial_volume << 5;
                initial_volume | 0b1001_1111
            }
            0xff1b | 0xff1d => 0xff, // write-only
            0xff1e => {
                let length_enable = if self.length.enable { 1 << 6 } else { 0 };
                length_enable | 0b1011_1111
            }
            0xff30..=0xff3f => {
                self.wave[(addr - 0xff30) as usize]
                // if self.enabled {
                //     self.wave[self.sample_index / 2]
                // } else {
                //     self.wave[(addr - 0xff30) as usize]
                // }
            }

            _ => unreachable!(),
        }
    }

    fn write(&mut self, div_apu: u8, addr: u16, val: u8, _: bool) {
        // log::debug!("Ch3: write: {addr:04x?} = {val:02x?}");
        match addr {
            0xff1a => {
                self.dac_enabled = (val & 0b1000_0000) > 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xff1b => {
                self.length.length = 256 - u16::from(val);
                log::debug!("Ch3: write length: {}", self.length.length);
            }
            0xff1c => {
                self.initial_volume = extract(val, 0b0110_0000);
            }
            0xff1d => {
                self.period = (self.period & 0xff00) | u16::from(val);
            }
            0xff1e => {
                self.period = (self.period & 0x00ff) | (u16::from(val & 0b0000_0111) << 8);
                let length_enable = (val & 0b0100_0000) > 0;

                if self.length.write_nrx4(length_enable, div_apu) {
                    self.enabled = false;
                }

                if (val & 0b1000_0000) > 0 {
                    self.length.trigger(256, div_apu);
                    self.volume = self.initial_volume;
                    self.enabled = true;
                    self.timer = 2048 - self.period;
                    if !self.dac_enabled {
                        self.enabled = false;
                    }
                }
            }
            0xff30..=0xff3f => self.wave[(addr - 0xff30) as usize] = val,
            _ => unreachable!(),
        }
    }

    fn sample(&self) -> f32 {
        if self.dac_enabled && self.enabled {
            let idx = self.sample_index / 2;
            let sample = match self.sample_index % 2 {
                0 => (self.wave[idx] & 0xf0) >> 4,
                1 => self.wave[idx] & 0x0f,
                _ => unreachable!(),
            };
            let shift = match self.volume {
                0 => 4,
                1 => 0,
                2 => 1,
                3 => 2,
                _ => unreachable!(),
            };
            (f32::from(sample >> shift) / 15.0) * 2.0 - 1.0
        } else {
            0.0
        }
    }

    fn clock_fast(&mut self) {
        if self.enabled {
            self.timer += 1;
            if self.timer > 0x7ff {
                self.sample_index = (self.sample_index + 1) % 32;
                self.timer = self.period;
            }
        }
    }
}
