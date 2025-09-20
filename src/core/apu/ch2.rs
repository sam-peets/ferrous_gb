use crate::core::{apu::Channel, util::extract};

#[derive(Debug, Default)]
pub struct Ch2 {
    // NR21
    duty: u8,
    length: u8,

    // NR22
    initial_volume: u8,
    envelope_dir: u8,
    envelope_pace: u8,

    // NR23 and NR24
    period: u16,

    // NR24
    length_enable: bool,

    pub enabled: bool,
    envelope: u8,
    volume: u8,
    dac_enabled: bool,
}

impl Channel for Ch2 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff16 => {
                let duty = self.duty << 6;
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
                let length = if self.length_enable { 1 << 6 } else { 0 };
                length | 0b1011_1111
            }
            _ => unreachable!(),
        }
    }
    fn write(&mut self, div_apu: u8, addr: u16, val: u8) {
        // log::debug!("Ch2: write: {addr:04x?} = {val:02x?}");
        match addr {
            0xff16 => {
                self.duty = extract(val, 0b1100_0000);
                self.length = 64 - extract(val, 0b0011_1111);
                log::debug!("Ch2: write length: {}", self.length);
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
                if (val & 0b1000_0000) > 0 {
                    self.enabled = true;
                    self.envelope = 0;
                    if self.length == 0 {
                        self.length = if div_apu % 2 == 0 { 63 } else { 64 };
                    }

                    self.volume = self.initial_volume;
                    if !self.dac_enabled {
                        self.enabled = false;
                    }
                }
                let length_enable_old = self.length_enable;
                self.length_enable = (val & 0b0100_0000) > 0;
                log::debug!(
                    "Ch2: extra clock cases: {} ({}) {} {} {}",
                    div_apu,
                    div_apu % 2,
                    length_enable_old,
                    self.length_enable,
                    self.length
                );
                if ((div_apu % 2) == 0)
                    && !length_enable_old
                    && self.length_enable
                    && self.length != 0
                {
                    self.clock_length();
                }

                self.period = ((val as u16 & 0b0000_0111) << 8) | self.period & 0xff;
            }
            _ => unreachable!(),
        }
    }
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
            log::debug!("ch2: clock length: {} -> {}", self.length, new_length);
            self.length = new_length;
        }
    }

    fn clear(&mut self) {
        *self = Default::default();
    }
}
