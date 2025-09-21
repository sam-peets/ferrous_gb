use crate::core::{apu::Channel, util::extract};

#[derive(Debug, Default)]
pub struct Ch1 {
    // NR10
    sweep_pace: u8,
    sweep_direction: u8,
    sweep_shift: u8,

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

    // internal sweep registers
    sweep_period_shadow: u16,
    sweep_enabled: bool,
    sweep_timer: u8,
    sweep_used_negative: bool,
}

impl Ch1 {
    fn clock_sweep(&mut self) {
        if self.enabled && self.sweep_pace != 0 {
            log::debug!(
                "Ch1: clock sweep: period: {:04x?}, shadow: {:04x?}, shift: {}, direction: {}, pace: {}",
                self.period,
                self.sweep_period_shadow,
                self.sweep_shift,
                self.sweep_direction,
                self.sweep_pace,
            );
            let n_period = self.sweep_overflow_check();
            log::debug!("Ch1: clock_sweep: n_period: {:04x?}", n_period);
            if self.sweep_shift != 0 {
                // write back to shadow and period
                self.sweep_period_shadow = n_period & 0x7ff;
                self.period = self.sweep_period_shadow;
                self.sweep_overflow_check();
            }
        }
    }
    fn sweep_overflow_check(&mut self) -> u16 {
        log::debug!(
            "Ch1: overflow check: period: {:04x?}, shadow: {:04x?}, shift: {}, direction: {}, pace: {}",
            self.period,
            self.sweep_period_shadow,
            self.sweep_shift,
            self.sweep_direction,
            self.sweep_pace,
        );
        let d_period = self.sweep_period_shadow >> self.sweep_shift;
        let n_period = if self.sweep_direction == 0 {
            self.sweep_period_shadow + d_period
        } else {
            log::debug!("Ch1: seen negative mode");
            self.sweep_used_negative = true;
            // let period_2s_complement = (!(d_period | (!0x7ff)) + 1) & 0x7ff;
            // log::debug!("Ch1: negative: {:02x?}", period_2s_complement);
            // self.sweep_period_shadow.wrapping_add(period_2s_complement) & 0x7ff
            self.sweep_period_shadow - d_period
        };
        log::debug!("Ch1: n_period: 0x{n_period:04x?}");
        if n_period > 0x07ff {
            log::debug!("Ch1: sweep overflow");
            self.enabled = false;
        }
        n_period & 0x7ff
    }
}

impl Channel for Ch1 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff10 => {
                let shift = self.sweep_shift;
                let direction = self.sweep_direction << 3;
                let pace = self.sweep_pace << 4;
                0b1000_0000 | pace | direction | shift
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
                self.sweep_shift = extract(val, 0b0000_0111);
                if self.sweep_direction == 0 && self.sweep_used_negative {
                    // "Clearing the sweep negate mode bit in NR10 after at least one sweep calculation
                    //  has been made using the negate mode since the last trigger causes the channel to be immediately disabled."
                    log::debug!("Ch1: negative -> positive switch, disabling channel");
                    self.enabled = false;
                }
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
                let length_enable_old = self.length_enable;
                self.length_enable = (val & 0b0100_0000) > 0;
                if (val & 0b1000_0000) > 0 {
                    self.enabled = true;
                    if !self.dac_enabled {
                        self.enabled = false;
                    }
                    if self.length == 0 {
                        self.length = 64;
                    }
                    self.envelope = 0;
                    self.volume = self.initial_volume;

                    self.sweep_used_negative = false;
                    self.sweep_period_shadow = self.period;
                    self.sweep_timer = if self.sweep_pace == 0 {
                        7
                    } else {
                        self.sweep_pace - 1
                    };
                    self.sweep_enabled = self.sweep_pace > 0 || self.sweep_shift > 0;
                    if self.sweep_shift > 0 {
                        log::debug!("Ch1: sweep overflow check on trigger");
                        self.sweep_overflow_check();
                    }
                    log::debug!("Ch1: sweep_enabled: {}", self.sweep_enabled);
                }
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

    fn clock(&mut self, div_apu: u8) {
        if div_apu % 2 == 0 {
            log::debug!("Ch1: clocking length from clock: div_apu: {div_apu:?}");
            self.clock_length();
        }

        if ((div_apu + 2) % 8) % 4 == 0 {
            log::debug!(
                "Ch1: clocking sweep: div_apu: {div_apu:?}, sweep_timer: {}",
                self.sweep_timer
            );

            if self.sweep_enabled && self.sweep_timer == 0 {
                log::debug!(
                    "Ch1: really really clocking sweep: div_apu: {}, sweep_timer: {}",
                    div_apu,
                    self.sweep_timer
                );
                self.clock_sweep();
            }
            let (n_sweep_timer, underflow) = self.sweep_timer.overflowing_sub(1);
            self.sweep_timer = if underflow {
                if self.sweep_pace == 0 {
                    7
                } else {
                    self.sweep_pace - 1
                }
            } else {
                n_sweep_timer
            }
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
