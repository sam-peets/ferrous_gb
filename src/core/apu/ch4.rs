use crate::core::{
    apu::{Channel, length::Length},
    util::extract,
};

#[derive(Debug, Default)]
pub struct Ch4 {
    // NR42
    initial_volume: u8,
    envelope_dir: u8,
    envelope_pace: u8,

    // NR43
    clock_shift: u8,
    lfsr_width: u8,
    clock_divider: u8,

    pub enabled: bool,
    envelope: u8,
    volume: u8,
    dac_enabled: bool,
    length: Length<u8>,
}

impl Channel for Ch4 {
    fn clock(&mut self, div_apu: u8) {
        if div_apu % 2 == 0 && self.length.clock() {
            self.enabled = false;
        }
    }

    fn clear(&mut self) {
        *self = Ch4 {
            length: self.length,
            ..Default::default()
        };
        self.length.clear();
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff20 => 0xff,
            0xff21 => {
                let volume = self.initial_volume << 4;
                let dir = self.envelope_dir << 3;
                let pace = self.envelope_pace;
                volume | dir | pace
            }
            0xff22 => {
                let clock = self.clock_shift << 4;
                let width = self.lfsr_width << 3;
                let div = self.clock_divider;
                clock | width | div
            }
            0xff23 => {
                let length = if self.length.enable { 1 << 6 } else { 0 };
                0b1011_1111 | length
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, div_apu: u8, addr: u16, val: u8, _: bool) {
        // log::debug!("Ch4: write: {addr:04x?} = {val:02x?}");
        match addr {
            // NR41
            0xff20 => {
                self.length.length = 64 - extract(val, 0b0011_1111);
                log::debug!("Ch4: write length: {}", self.length.length);
            }
            // NR42
            0xff21 => {
                self.initial_volume = extract(val, 0b1111_0000);
                self.envelope_dir = extract(val, 0b0000_1000);
                self.envelope_pace = extract(val, 0b0000_0111);
                self.dac_enabled = (val & 0b1111_1000) > 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            // NR43
            0xff22 => {
                self.clock_shift = extract(val, 0b1111_0000);
                self.lfsr_width = extract(val, 0b0000_1000);
                self.clock_divider = extract(val, 0b0000_0111);
            }
            // NR44
            0xff23 => {
                let length_enable = (val & 0b0100_0000) > 0;
                if self.length.write_nrx4(length_enable, div_apu) {
                    self.enabled = false;
                }
                if (val & 0b1000_0000) > 0 {
                    self.envelope = 0;
                    self.volume = self.initial_volume;

                    self.length.trigger(64, div_apu);
                    // TODO: lfsr

                    self.enabled = true;
                    if !self.dac_enabled {
                        self.enabled = false;
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    fn sample(&self) -> f32 {
        0.0
    }

    fn clock_fast(&mut self) {
        todo!()
    }
}
