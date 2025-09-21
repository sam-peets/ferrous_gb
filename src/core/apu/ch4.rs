use crate::core::{apu::Channel, util::extract};

#[derive(Debug, Default)]
pub struct Ch4 {
    // NR41
    length: u8,

    // NR42
    initial_volume: u8,
    envelope_dir: u8,
    envelope_pace: u8,

    // NR43
    clock_shift: u8,
    lfsr_width: u8,
    clock_divider: u8,

    // NR44
    length_enable: bool,

    pub enabled: bool,
    envelope: u8,
    volume: u8,
    dac_enabled: bool,
}

impl Channel for Ch4 {
    fn clock(&mut self, div_apu: u8) {
        if div_apu % 2 == 0 {
            self.clock_length();
        }
    }
    fn clock_length(&mut self) {
        if self.length_enable && self.length > 0 {
            let new_length = self.length.wrapping_sub(1);
            if new_length == 0 {
                self.enabled = false;
            }
            log::debug!("ch4: clock length: {} -> {}", self.length, new_length);
            self.length = new_length;
        }
    }

    fn clear(&mut self) {
        *self = Default::default();
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
                let length = if self.length_enable { 1 << 6 } else { 0 };
                0b1011_1111 | length
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, div_apu: u8, addr: u16, val: u8) {
        // log::debug!("Ch4: write: {addr:04x?} = {val:02x?}");
        match addr {
            // NR41
            0xff20 => {
                self.length = 64 - extract(val, 0b0011_1111);
                log::debug!("Ch4: write length: {}", self.length);
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
                if (val & 0b1000_0000) > 0 {
                    self.envelope = 0;
                    self.volume = self.initial_volume;
                    if self.length == 0 {
                        self.length = 64;
                    }
                    // TODO: lfsr

                    self.enabled = true;
                    if !self.dac_enabled {
                        self.enabled = false;
                    }
                }

                let length_enable_old = self.length_enable;
                self.length_enable = (val & 0b0100_0000) > 0;
                if ((div_apu % 2) == 0)
                    && !length_enable_old
                    && self.length_enable
                    && self.length != 0
                {
                    log::debug!("Ch1: clocking length from trigger: div_apu: {div_apu:?}");
                    self.clock_length();
                }
            }
            _ => unreachable!(),
        }
    }
}
