use crate::core::{apu::Channel, util::extract};

#[derive(Debug, Default)]
pub struct Ch1 {
    // NR10
    sweep_pace: u8,
    sweep_direction: u8,
    sweep_step: u8,

    // NR11
    duty: u8,
    initial_length: u8,

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
    length: u8,
    volume: u8,
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
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0xff10 => {
                self.sweep_pace = extract(val, 0b0111_0000);
                self.sweep_direction = extract(val, 0b0000_1000);
                self.sweep_step = extract(val, 0b0000_0111);
            }
            0xff11 => {
                self.duty = extract(val, 0b1100_0000);
                self.initial_length = extract(val, 0b0011_1111);
            }
            0xff12 => {
                self.initial_volume = extract(val, 0b1111_0000);
                self.envelope_dir = extract(val, 0b0000_1000);
                self.envelope_pace = extract(val, 0b0000_0111);
            }
            0xff13 => {
                self.period = (self.period & 0xff00) | val as u16;
            }
            0xff14 => {
                self.period = ((val as u16 & 0b0000_0111) << 8) | self.period & 0xff;
                self.length_enable = (val & 0b0100_0000) > 0;
                if (val & 0b1000_0000) > 0 {
                    self.enabled = true;
                    self.envelope = 0;
                    self.length = self.initial_length;
                    self.volume = self.initial_volume;
                }
            }
            _ => unreachable!(),
        }
    }
    fn clock(&mut self, div_apu: u8) {
        if !self.enabled {
            return;
        }
        // // nr10
        // let pace = (self.nr10 & 0b0111_0000) >> 4;
        // let direction = (self.nr10 & 0b0000_1000) >> 4;
        // let individual_step = (self.nr10 & 0b0000_0111) >> 4;
        // let lt = (((self.nr14 as u16) & 0b0000_0111) << 8) | self.nr13 as u16;
        // let step = 2u8.pow(individual_step as u32);
        // let d_lt = lt / step as u16;
        // let lt_next = if direction == 0 { lt + d_lt } else { lt - d_lt };
        // if lt_next > 0x7ff && direction == 0 {
        //     // would overflow & direction is addition
        //     // pandocs: this happens when pace is 0 as well

        //     self.enabled = false;
        // } else if pace != 0 && (div_apu % (pace * 4) == 0) {
        //     let lt_next_high = ((lt_next & 0xff00) >> 8) as u8;
        //     let lt_next_low = (lt_next & 0x00ff) as u8;
        //     self.nr13 = lt_next_low;
        //     self.nr14 = (self.nr14 & 0b1111_1000) | lt_next_high;
        // }

        // // nr11
        // let duty_cyle = (self.nr11 & 0b1100_0000) >> 6;
        // let length = self.nr11 & 0b0011_1111;
        // let new_length = if div_apu % 2 == 0 {
        //     let nl = length.wrapping_add(1);
        //     if nl == 64 {
        //         self.enabled = false;
        //     }
        //     nl
        // } else {
        //     length
        // };
        // self.nr11 = (duty_cyle << 6) | length;
    }

    fn clear(&mut self) {
        *self = Default::default();
    }
}
