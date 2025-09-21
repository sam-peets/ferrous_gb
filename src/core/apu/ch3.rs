use crate::core::{apu::Channel, util::extract};

#[derive(Debug, Default)]
pub struct Ch3 {
    // NR30
    dac_enabled: bool,

    // NR31
    length: u16,

    // NR32
    initial_volume: u8,

    // NR32 & NR33
    period: u16,

    // NR34
    length_enable: bool,

    pub enabled: bool,
    volume: u8,
}

impl Ch3 {}

impl Channel for Ch3 {
    fn clock(&mut self, div_apu: u8) {
        if div_apu % 2 == 0 && self.length_enable && self.length > 0 {
            self.clock_length();
        }
    }
    fn clock_length(&mut self) {
        if self.length_enable && self.length > 0 {
            let new_length = self.length.wrapping_sub(1);
            if new_length == 0 {
                self.enabled = false;
            }
            log::debug!("ch3: clock length: {} -> {}", self.length, new_length);
            self.length = new_length;
        }
    }

    fn clear(&mut self) {
        *self = Default::default();
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff1a => {
                let dac_enabled = if self.dac_enabled { 1 << 7 } else { 0 };
                dac_enabled | 0b0111_1111
            }
            0xff1b => 0xff, // write-only
            0xff1c => {
                let initial_volume = self.initial_volume << 5;
                initial_volume | 0b1001_1111
            }
            0xff1d => 0xff,
            0xff1e => {
                let length_enable = if self.length_enable { 1 << 6 } else { 0 };
                length_enable | 0b1011_1111
            }

            _ => unreachable!(),
        }
    }

    fn write(&mut self, div_apu: u8, addr: u16, val: u8) {
        // log::debug!("Ch3: write: {addr:04x?} = {val:02x?}");
        match addr {
            0xff1a => {
                self.dac_enabled = (val & 0b1000_0000) > 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xff1b => {
                self.length = 256 - (val as u16);
                log::debug!("Ch3: write length: {}", self.length);
            }
            0xff1c => {
                self.initial_volume = extract(val, 0b0110_0000);
            }
            0xff1d => {
                self.period = (self.period & 0xff00) | val as u16;
            }
            0xff1e => {
                if (val & 0b1000_0000) > 0 {
                    if self.length == 0 {
                        self.length = 256;
                    }
                    self.volume = self.initial_volume;
                    self.enabled = true;
                    if !self.dac_enabled {
                        self.enabled = false;
                    }
                }

                self.period = (self.period & 0x00ff) | (((val & 0b0000_0111) as u16) << 8);
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
