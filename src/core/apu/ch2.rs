use crate::core::{
    apu::{Channel, duty_cycle::DutyCycle, length::Length},
    util::extract,
};

#[derive(Debug, Default)]
pub struct Ch2 {
    // NR22
    initial_volume: u8,
    envelope_dir: u8,
    envelope_pace: u8,

    // NR23 and NR24
    period: u16,

    pub enabled: bool,
    envelope: u8,
    volume: u8,
    dac_enabled: bool,

    duty_cycle: DutyCycle,
    length: Length<u8>,
}

impl Channel for Ch2 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff16 => {
                let duty = self.duty_cycle.pattern << 6;
                0b0011_1111 | duty
            }
            0xff17 => {
                let volume = self.initial_volume << 4;
                let dir = self.envelope_dir << 3;
                let pace = self.envelope_pace;
                volume | dir | pace
            }
            0xff18 => 0xff, // write-only
            0xff19 => {
                let length = if self.length.enable { 1 << 6 } else { 0 };
                length | 0b1011_1111
            }
            _ => unreachable!(),
        }
    }
    fn write(&mut self, div_apu: u8, addr: u16, val: u8, enabled: bool) {
        // log::debug!("Ch2: write: {addr:04x?} = {val:02x?}");
        match addr {
            0xff16 => {
                // length registers are writable when apu is disabled, but not duty
                if enabled {
                    self.duty_cycle.pattern = extract(val, 0b1100_0000);
                }
                self.length.length = 64 - extract(val, 0b0011_1111);
                log::debug!("Ch2: write length: {}", self.length.length);
            }
            0xff17 => {
                self.initial_volume = extract(val, 0b1111_0000);
                self.envelope_dir = extract(val, 0b0000_1000);
                self.envelope_pace = extract(val, 0b0000_0111);
                self.dac_enabled = (val & 0b1111_1000) > 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xff18 => {
                self.period = (self.period & 0xff00) | val as u16;
            }
            0xff19 => {
                self.period = ((val as u16 & 0b0000_0111) << 8) | self.period & 0xff;
                let length_enable = (val & 0b0100_0000) > 0;

                if self.length.write_nrx4(length_enable, div_apu) {
                    self.enabled = false;
                }
                if (val & 0b1000_0000) > 0 {
                    self.enabled = true;
                    if !self.dac_enabled {
                        self.enabled = false;
                    }
                    self.envelope = 0;

                    self.length.trigger(64, div_apu);

                    self.volume = self.initial_volume;
                }
            }
            _ => unreachable!(),
        }
    }
    fn clock(&mut self, div_apu: u8) {
        if div_apu % 2 == 0 && self.length.clock() {
            self.enabled = false
        }
    }

    fn clear(&mut self) {
        *self = Ch2 {
            length: self.length,
            ..Default::default()
        };
        self.length.clear();
    }

    fn sample(&self) -> f32 {
        if self.dac_enabled {
            let volume = self.volume as f32 / 15.0;
            let duty = self.duty_cycle.sample() as f32 * 2.0 - 1.0;
            duty * volume
        } else {
            0.0
        }
    }

    fn clock_fast(&mut self) {
        if self.enabled {
            self.duty_cycle.clock(self.period);
        }
    }
}
