use crate::core::{
    apu::{Channel, duty_cycle::DutyCycle, envelope::Envelope, length::Length, sweep::Sweep},
    mmu::mmio::{self, NR10, NR11, NR12, NR13, NR14},
    util::extract,
};

#[derive(Debug, Default)]
pub struct Ch1 {
    // NR13 and NR14
    period: u16,

    pub enabled: bool,
    pub dac_enabled: bool,

    duty_cycle: DutyCycle,
    length: Length<u8>,
    envelope: Envelope,
    sweep: Sweep,
}

impl Channel for Ch1 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            NR10 => {
                let shift = self.sweep.shift;
                let direction = self.sweep.direction << 3;
                let pace = self.sweep.pace << 4;
                0b1000_0000 | pace | direction | shift
            }
            NR11 => {
                let duty = self.duty_cycle.pattern << 6;
                0b0011_1111 | duty
            }
            NR12 => {
                let volume = self.envelope.initial_volume << 4;
                let dir = self.envelope.direction << 3;
                let pace = self.envelope.pace;
                volume | dir | pace
            }
            NR13 => 0xff, // write-only
            NR14 => {
                let length = if self.length.enable { 1 << 6 } else { 0 };
                length | 0b1011_1111
            }
            _ => unreachable!(),
        }
    }
    fn write(&mut self, div_apu: u8, addr: u16, val: u8, enabled: bool) {
        // log::debug!("Ch1: write: {addr:04x?} = {val:02x?}");
        match addr {
            NR10 => {
                self.sweep.pace = extract(val, 0b0111_0000);
                self.sweep.direction = extract(val, 0b0000_1000);
                self.sweep.shift = extract(val, 0b0000_0111);
                if self.sweep.direction == 0 && self.sweep.used_negative {
                    // "Clearing the sweep negate mode bit in NR10 after at least one sweep calculation
                    //  has been made using the negate mode since the last trigger causes the channel to be immediately disabled."
                    log::debug!("Ch1: negative -> positive switch, disabling channel");
                    self.enabled = false;
                }
            }
            NR11 => {
                // length registers are writable when apu is disabled, but not duty
                if enabled {
                    self.duty_cycle.pattern = extract(val, 0b1100_0000);
                }
                self.length.length = 64 - extract(val, 0b0011_1111);
                log::debug!("Ch1: write length: {}", self.length.length);
            }
            NR12 => {
                self.envelope.initial_volume = extract(val, 0b1111_0000);
                self.envelope.direction = extract(val, 0b0000_1000);
                self.envelope.pace = extract(val, 0b0000_0111);

                self.dac_enabled = (val & 0b1111_1000) > 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            NR13 => {
                self.period = (self.period & 0xff00) | u16::from(val);
            }
            NR14 => {
                self.period = ((u16::from(val) & 0b0000_0111) << 8) | self.period & 0xff;
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

                    if let Some(enabled) = self.sweep.trigger(self.period) {
                        self.enabled = enabled;
                    }
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
            let (update_period, update_enabled) = self.sweep.clock(self.enabled);
            if let Some(period) = update_period {
                self.period = period;
            }
            if let Some(enabled) = update_enabled {
                self.enabled = enabled;
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
            let volume = f32::from(self.envelope.volume) / 15.0;
            let duty = f32::from(self.duty_cycle.sample()) * 2.0 - 1.0;
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
