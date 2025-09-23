use crate::core::{
    apu::{Channel, duty_cycle::DutyCycle, envelope::Envelope, length::Length},
    util::extract,
};

#[derive(Debug, Default)]
pub struct Ch1 {
    // NR10
    sweep_pace: u8,
    sweep_direction: u8,
    sweep_shift: u8,

    // NR13 and NR14
    period: u16,

    pub enabled: bool,
    pub dac_enabled: bool,

    // internal sweep registers
    sweep_period_shadow: u16,
    sweep_enabled: bool,
    sweep_timer: u8,
    sweep_used_negative: bool,

    duty_cycle: DutyCycle,
    length: Length<u8>,
    envelope: Envelope,
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
                let duty = self.duty_cycle.pattern << 6;
                0b0011_1111 | duty
            }
            0xff12 => {
                let volume = self.envelope.initial_volume << 4;
                let dir = self.envelope.direction << 3;
                let pace = self.envelope.pace;
                volume | dir | pace
            }
            0xff13 => 0xff, // write-only
            0xff14 => {
                let length = if self.length.enable { 1 << 6 } else { 0 };
                length | 0b1011_1111
            }
            _ => unreachable!(),
        }
    }
    fn write(&mut self, div_apu: u8, addr: u16, val: u8, enabled: bool) {
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
                // length registers are writable when apu is disabled, but not duty
                if enabled {
                    self.duty_cycle.pattern = extract(val, 0b1100_0000);
                }
                self.length.length = 64 - extract(val, 0b0011_1111);
                log::debug!("Ch1: write length: {}", self.length.length);
            }
            0xff12 => {
                self.envelope.initial_volume = extract(val, 0b1111_0000);
                self.envelope.direction = extract(val, 0b0000_1000);
                self.envelope.pace = extract(val, 0b0000_0111);

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
                let trigger = (val & 0b1000_0000) > 0;
                let length_enable = (val & 0b0100_0000) > 0;
                if self.length.write_nrx4(length_enable, div_apu) {
                    self.enabled = false;
                }

                if trigger {
                    self.enabled = true;
                    if !self.dac_enabled {
                        self.enabled = false;
                    }
                    self.length.trigger(64, div_apu);
                    self.envelope.trigger();

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
            }
            _ => unreachable!(),
        }
    }

    fn clock(&mut self, div_apu: u8) {
        if div_apu % 2 == 0 && self.length.clock() {
            self.enabled = false;
        }
        if div_apu == 7 {
            self.envelope.clock();
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

    fn clear(&mut self) {
        *self = Ch1 {
            length: self.length,
            ..Default::default()
        };
        self.length.clear();
    }

    fn sample(&self) -> f32 {
        if self.dac_enabled && self.enabled {
            let volume = self.envelope.volume as f32 / 15.0;
            let duty = self.duty_cycle.sample() as f32 * 2.0 - 1.0;
            duty * volume
        } else {
            0.0
        }
    }

    fn clock_fast(&mut self) {
        if self.enabled {
            self.duty_cycle.clock(self.period);
        }
    }
}
