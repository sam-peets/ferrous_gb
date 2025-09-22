mod ch1;
mod ch2;
mod ch3;
mod ch4;
mod duty_cycle;
mod sweep;

use anyhow::anyhow;

use crate::core::apu::{ch1::Ch1, ch2::Ch2, ch3::Ch3, ch4::Ch4};
#[derive(Debug, Default)]
pub struct Apu {
    nr50: u8,       // master volume & vin panning
    nr51: u8,       // sound panning
    wave: [u8; 16], // wave pattern RAM (0xff30-ff3f)

    ch1: Ch1,
    ch2: Ch2,
    ch3: Ch3,
    ch4: Ch4,

    div_apu: u8,
    enabled: bool,
    sys_old: u16,
}

trait Channel {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, div_apu: u8, addr: u16, val: u8, enabled: bool);
    fn clock(&mut self, div_apu: u8);
    fn clear(&mut self);
    fn clock_length(&mut self);
    fn sample(&self);
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
                0xff30..=0xff3f => self.wave[(addr - 0xff30) as usize] = val,
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
            0xff30..=0xff3f => self.wave[(addr - 0xff30) as usize],
            _ => return Err(anyhow!("Apu: invalid register write: {addr:04x?}")),
        };
        log::debug!("Apu: read: [{sys:04x?}] {addr:04x?} = {v:02x?}");

        Ok(v)
    }
}

impl Apu {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn clock(&mut self, sys: u16) {
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
}
