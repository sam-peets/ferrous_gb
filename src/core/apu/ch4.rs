use crate::core::{
    apu::{Channel, envelope::Envelope, length::Length},
    util::extract,
};

#[derive(Debug, Default)]
pub struct Ch4 {
    // NR43
    clock_shift: u8,
    lfsr_width: u8,
    clock_divider: u8,

    pub enabled: bool,
    pub dac_enabled: bool,
    length: Length<u8>,
    lfsr: u16,
    timer: usize,
    envelope: Envelope,
}

impl Ch4 {
    fn reset_timer(&mut self) {
        let divisor = match self.clock_divider {
            0 => 8,
            1 => 16,
            2 => 32,
            3 => 48,
            4 => 64,
            5 => 80,
            6 => 96,
            7 => 112,
            _ => unreachable!(),
        } / 4;
        self.timer = divisor << self.clock_shift;
    }

    fn update_lfsr(&mut self) {
        let xnor = u16::from((self.lfsr & 0b01) == ((self.lfsr & 0b10) >> 1));
        self.lfsr &= !(1 << 15);
        self.lfsr |= xnor << 15;
        if self.lfsr_width == 1 {
            self.lfsr &= !(1 << 7);
            self.lfsr |= xnor << 7;
        }
        self.lfsr >>= 1;
    }
}

impl Channel for Ch4 {
    fn clock(&mut self, div_apu: u8) {
        if div_apu % 2 == 0 && self.length.clock() {
            self.enabled = false;
        }
        if div_apu == 7 {
            self.envelope.clock();
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
                let volume = self.envelope.initial_volume << 4;
                let dir = self.envelope.direction << 3;
                let pace = self.envelope.pace;
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
        // println!("Ch4: write: {addr:04x?} = {val:02x?}");
        match addr {
            // NR41
            0xff20 => {
                self.length.length = 64 - extract(val, 0b0011_1111);
                log::debug!("Ch4: write length: {}", self.length.length);
            }
            // NR42
            0xff21 => {
                self.envelope.initial_volume = extract(val, 0b1111_0000);
                self.envelope.direction = extract(val, 0b0000_1000);
                self.envelope.pace = extract(val, 0b0000_0111);
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
                // println!("ch4: trigger: {val:08b}");
                if self.length.write_nrx4(length_enable, div_apu) {
                    self.enabled = false;
                }
                if (val & 0b1000_0000) > 0 {
                    self.envelope.trigger();
                    self.length.trigger(64, div_apu);
                    self.reset_timer();
                    self.lfsr = 0;

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
        if self.dac_enabled && self.enabled {
            let volume = f32::from(self.envelope.volume) / 15.0;
            let sample = f32::from(self.lfsr & 1) * 2.0 - 1.0;
            volume * sample
        } else {
            0.0
        }
    }

    fn clock_fast(&mut self) {
        if self.enabled {
            self.timer -= 1;
            if self.timer == 0 {
                self.reset_timer();
                self.update_lfsr();
            }
        }
    }
}
