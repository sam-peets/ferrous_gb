mod ch1;
mod ch2;
mod ch3;
mod ch4;
mod duty_cycle;
pub mod envelope;
mod length;

use std::sync::{Arc, RwLock};

use anyhow::anyhow;

use crate::{
    core::apu::{ch1::Ch1, ch2::Ch2, ch3::Ch3, ch4::Ch4},
    screen::ApuSamples,
};
#[derive(Debug, Default)]
pub struct Apu {
    nr50: u8, // master volume & vin panning
    nr51: u8, // sound panning

    ch1: Ch1,
    ch2: Ch2,
    ch3: Ch3,
    ch4: Ch4,

    div_apu: u8,
    enabled: bool,
    sys_old: u16,
    pub cur_sample: ApuSamples,
    sample_rate: u32,
    capacitor: f32,
}

trait Channel {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, div_apu: u8, addr: u16, val: u8, enabled: bool);
    // Clocks every APU cycle (512hz)
    fn clock(&mut self, div_apu: u8);

    /// Clocks every M-cycle
    fn clock_fast(&mut self);
    fn clear(&mut self);
    fn sample(&self) -> f32;
}

impl Apu {
    fn clear_regs(&mut self) {
        self.ch1.clear();
        self.ch2.clear();
        self.ch3.clear();
        self.ch4.clear();

        self.nr50 = 0;
        self.nr51 = 0;
    }
    pub fn write(&mut self, addr: u16, val: u8, sys: u16) -> anyhow::Result<()> {
        log::debug!("Apu: write: [{sys:04x?}] {addr:04x?} = {val:02x?}");
        if self.enabled
            || matches!(
                addr,
                0xff11 | 0xff16 | 0xff1b | 0xff20 | 0xff26 | 0xff30..=0xff3f
            )
        {
            // audio is enabled or writing to nr52/wave ram
            match addr {
                0xff10..=0xff14 => self.ch1.write(self.div_apu, addr, val, self.enabled),
                0xff16..=0xff19 => self.ch2.write(self.div_apu, addr, val, self.enabled),
                0xff1a..=0xff1e => self.ch3.write(self.div_apu, addr, val, self.enabled),
                0xff20..=0xff23 => self.ch4.write(self.div_apu, addr, val, self.enabled),
                0xff24 => self.nr50 = val,
                0xff25 => self.nr51 = val,
                0xff26 => {
                    let set_enabled = val & 0b1000_0000;
                    if set_enabled > 0 && !self.enabled {
                        // disabled -> enabled transition
                        self.div_apu = 0;
                    }
                    self.enabled = set_enabled > 0;
                    if set_enabled == 0 {
                        self.clear_regs();
                    }
                }
                0xff30..=0xff3f => self.ch3.write(self.div_apu, addr, val, self.enabled),
                _ => return Err(anyhow!("Apu: invalid register write: {addr:04x?}")),
            };
        }

        Ok(())
    }
    pub fn read(&self, addr: u16, sys: u16) -> anyhow::Result<u8> {
        let v = match addr {
            0xff10..=0xff14 => self.ch1.read(addr),
            0xff16..=0xff19 => self.ch2.read(addr),
            0xff1a..=0xff1e => self.ch3.read(addr),
            0xff20..=0xff23 => self.ch4.read(addr),
            0xff24 => self.nr50,
            0xff25 => self.nr51,
            0xff26 => {
                let ch_enabled = {
                    let mut enabled = 0;
                    if self.ch1.enabled {
                        enabled |= 0b1
                    }
                    if self.ch2.enabled {
                        enabled |= 0b1 << 1
                    }
                    if self.ch3.enabled {
                        enabled |= 0b1 << 2
                    }
                    if self.ch4.enabled {
                        enabled |= 0b1 << 3
                    }
                    enabled
                };
                let enabled = if self.enabled { 1 << 7 } else { 0 };
                enabled | 0b0111_0000 | ch_enabled
            }
            0xff30..=0xff3f => self.ch3.read(addr),
            _ => return Err(anyhow!("Apu: invalid register write: {addr:04x?}")),
        };
        log::debug!("Apu: read: [{sys:04x?}] {addr:04x?} = {v:02x?}");

        Ok(v)
    }

    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            ..Default::default()
        }
    }

    pub fn clock(&mut self, sys: u16) {
        // clock every M-cycle
        if sys % 4 == 0 {
            self.ch1.clock_fast();
            self.ch2.clock_fast();
            self.ch3.clock_fast();
            self.ch4.clock_fast();
        }
        if sys % (4194304 / self.sample_rate) as u16 == 0 {
            let sample = self.sample();
            let mut cur_sample = self
                .cur_sample
                .write()
                .expect("Apu: failed to unlock current sample for writing");
            cur_sample.push_back(sample);
        }

        // check for a falling edge
        let old_set = (self.sys_old & 0b0001_0000_0000_0000) > 0;
        let cur_unset = (sys & 0b0001_0000_0000_0000) == 0;
        self.sys_old = sys;
        if !(old_set && cur_unset) {
            return;
        }
        log::debug!("Apu: clock! {sys:04x?}");
        if !self.enabled {
            // apu is disabled, don't do anything
            return;
        }

        self.ch1.clock(self.div_apu % 8);
        self.ch2.clock(self.div_apu % 8);
        self.ch3.clock(self.div_apu % 8);
        self.ch4.clock(self.div_apu % 8);
        self.div_apu = self.div_apu.wrapping_add(1) % 8;
    }

    /// adapted from https://gbdev.gg8.se/wiki/articles/Gameboy_sound_hardware
    fn high_pass(&mut self, input: f32) -> f32 {
        let mut out = 0.0;
        if self.ch1.dac_enabled
            || self.ch2.dac_enabled
            || self.ch3.dac_enabled
            || self.ch4.dac_enabled
        {
            out = input - self.capacitor;
            self.capacitor =
                input - out * 0.999958_f32.powf((4194304.0 / self.sample_rate as f32) / 2.0);
        }
        out
    }

    pub fn sample(&mut self) -> (f32, f32) {
        let ch1_sample = self.ch1.sample();
        let ch2_sample = self.ch2.sample();
        let ch3_sample = self.ch3.sample();
        let ch4_sample = self.ch4.sample();
        let left_volume = (self.nr50 & 0b0111_0000) >> 4;
        let right_volume = self.nr50 & 0b0000_0111;

        let left = {
            let mut mix = 0.0;
            if (self.nr51 & (1 << 7)) > 0 {
                mix += ch4_sample
            }
            if (self.nr51 & (1 << 6)) > 0 {
                mix += ch3_sample
            }
            if (self.nr51 & (1 << 5)) > 0 {
                mix += ch2_sample
            }
            if (self.nr51 & (1 << 4)) > 0 {
                mix += ch1_sample
            }
            self.high_pass(mix * (left_volume + 1) as f32 / 8.0)
        };
        let right = {
            let mut mix = 0.0;
            if (self.nr51 & (1 << 3)) > 0 {
                mix += ch4_sample
            }
            if (self.nr51 & (1 << 2)) > 0 {
                mix += ch3_sample
            }
            if (self.nr51 & (1 << 1)) > 0 {
                mix += ch2_sample
            }
            if (self.nr51 & (1 << 0)) > 0 {
                mix += ch1_sample
            }
            self.high_pass(mix * (right_volume + 1) as f32 / 8.0)
        };
        (left, right)
    }
}
