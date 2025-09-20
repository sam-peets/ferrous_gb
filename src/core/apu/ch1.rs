use crate::core::{apu::Channel, util::extract};

#[derive(Debug, Default)]
pub struct Ch1 {
    // NR10
    sweep_pace: u8,
    sweep_direction: u8,
    sweep_step: u8,

    // NR11
    duty: u8,
    length: u8,

    // NR12
    initial_volume: u8,
    envelope_dir: u8,
    envelope_pace: u8,

    // NR13 and NR14
    period: u16,

    // NR14
    length_enable: bool,

    pub enabled: bool,
    envelope: u8,
    volume: u8,
    dac_enabled: bool,
}

impl Channel for Ch1 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff10 => {
                let step = self.sweep_step;
                let direction = self.sweep_direction << 3;
                let pace = self.sweep_pace << 4;
                0b1000_0000 | pace | direction | step
            }
            0xff11 => {
                let duty = self.duty << 6;
                0b0011_1111 | duty
            }
            0xff12 => {
                let volume = self.initial_volume << 4;
                let dir = self.envelope_dir << 3;
                let pace = self.envelope_pace;
                volume | dir | pace
            }
            0xff13 => 0xff, // write-only
            0xff14 => {
                let length = if self.length_enable { 1 << 6 } else { 0 };
                length | 0b1011_1111
            }
            _ => unreachable!(),
        }
    }
    fn write(&mut self, div_apu: u8, addr: u16, val: u8) {
        // log::debug!("Ch1: write: {addr:04x?} = {val:02x?}");
        match addr {
            0xff10 => {
                self.sweep_pace = extract(val, 0b0111_0000);
                self.sweep_direction = extract(val, 0b0000_1000);
                self.sweep_step = extract(val, 0b0000_0111);
            }
            0xff11 => {
                self.duty = extract(val, 0b1100_0000);
                self.length = 64 - extract(val, 0b0011_1111);
                log::debug!("Ch1: write length: {}", self.length);
            }
            0xff12 => {
                self.initial_volume = extract(val, 0b1111_0000);
                self.envelope_dir = extract(val, 0b0000_1000);
                self.envelope_pace = extract(val, 0b0000_0111);

                self.dac_enabled = (val & 0b1111_1000) > 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xff13 => {
                self.period = (self.period & 0xff00) | val as u16;
            }
            0xff14 => {
                self.period = ((val as u16 & 0b0000_0111) << 8) | self.period & 0xff;
                if (val & 0b1000_0000) > 0 {
                    self.enabled = true;
                    if self.length == 0 {
                        self.length = if div_apu % 2 == 0 { 63 } else { 64 };
                    }
                    self.envelope = 0;
                    self.volume = self.initial_volume;
                    if !self.dac_enabled {
                        self.enabled = false;
                    }
                }
                let length_enable_old = self.length_enable;
                self.length_enable = (val & 0b0100_0000) > 0;
                log::debug!(
                    "Ch1: extra clock cases: {} ({}) {} {} {}",
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
            log::debug!("ch1: clock length: {} -> {}", self.length, new_length);
            self.length = new_length;
        }
    }

    fn clear(&mut self) {
        *self = Default::default();
    }
}
