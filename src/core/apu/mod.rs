use anyhow::anyhow;

#[derive(Debug, Default)]
pub struct ApuRegisters {
    nr10: u8,       // ch1 sweep
    nr11: u8,       // ch1 length timer & duty cycle
    nr12: u8,       // ch1 volume & envelope
    nr13: u8,       // ch1 period low
    nr14: u8,       // ch1 period high & control
    nr21: u8,       // ch2 length timer & duty cycle
    nr22: u8,       // ch2 volume & envelope
    nr23: u8,       // ch2 period low
    nr24: u8,       // ch2 period high & control
    nr30: u8,       // ch3 dac enable
    nr31: u8,       // ch3 length timer
    nr32: u8,       // ch3 output level
    nr33: u8,       // ch3 period low
    nr34: u8,       // ch3 period high & control
    nr41: u8,       // ch4 length timer
    nr42: u8,       // ch4 volume and envelope
    nr43: u8,       // ch4 frequency & randomness
    nr44: u8,       // ch4 control
    nr50: u8,       // master volume & vin panning
    nr51: u8,       // sound panning
    nr52: u8,       // audio master control
    wave: [u8; 16], // wave pattern RAM (0xff30-ff3f)
}

impl ApuRegisters {
    fn clear_regs(&mut self) {
        self.nr10 = 0;
        self.nr11 = 0;
        self.nr12 = 0;
        self.nr13 = 0;
        self.nr14 = 0;
        self.nr21 = 0;
        self.nr22 = 0;
        self.nr23 = 0;
        self.nr24 = 0;
        self.nr30 = 0;
        self.nr31 = 0;
        self.nr32 = 0;
        self.nr33 = 0;
        self.nr34 = 0;
        self.nr41 = 0;
        self.nr42 = 0;
        self.nr43 = 0;
        self.nr44 = 0;
        self.nr50 = 0;
        self.nr51 = 0;
    }
    pub fn write(&mut self, addr: u16, val: u8) -> anyhow::Result<()> {
        if (self.nr52 & 0b1000_0000) > 0 {
            // audio is enabled
            match addr {
                0xff10 => self.nr10 = val,
                0xff11 => self.nr11 = val,
                0xff12 => self.nr12 = val,
                0xff13 => self.nr13 = val,
                0xff14 => self.nr14 = val,
                0xff16 => self.nr21 = val,
                0xff17 => self.nr22 = val,
                0xff18 => self.nr23 = val,
                0xff19 => self.nr24 = val,
                0xff1a => self.nr30 = val,
                0xff1b => self.nr31 = val,
                0xff1c => self.nr32 = val,
                0xff1d => self.nr33 = val,
                0xff1e => self.nr34 = val,
                0xff20 => self.nr41 = val,
                0xff21 => self.nr42 = val,
                0xff22 => self.nr43 = val,
                0xff23 => self.nr44 = val,
                0xff24 => self.nr50 = val,
                0xff25 => self.nr51 = val,
                0xff26 => {
                    self.nr52 = val & 0b1000_0000;
                    if (self.nr52 & 0b1000_0000) == 0 {
                        self.clear_regs();
                    }
                }
                0xff30..=0xff3f => self.wave[(addr - 0xff30) as usize] = val,
                _ => return Err(anyhow!("Apu: invalid register write: {addr:04x?}")),
            };
        } else {
            match addr {
                0xff26 => self.nr52 = val & 0b1000_0000,
                0xff30..=0xff3f => self.wave[(addr - 0xff30) as usize] = val,
                _ => {} // writes are discarded if apu is off
            }
        }

        Ok(())
    }
    pub fn read(&self, addr: u16) -> anyhow::Result<u8> {
        let v = match addr {
            0xff10 => self.nr10 | 0x80,
            0xff11 => self.nr11 | 0x3f,
            0xff12 => self.nr12,
            0xff13 => 0xff,
            0xff14 => self.nr14 | 0xbf,
            0xff16 => self.nr21 | 0x3f,
            0xff17 => self.nr22,
            0xff18 => 0xff,
            0xff19 => self.nr24 | 0xbf,
            0xff1a => self.nr30 | 0x7f,
            0xff1b => 0xff,
            0xff1c => self.nr32 | 0x9f,
            0xff1d => 0xff,
            0xff1e => self.nr34 | 0xbf,
            0xff20 => 0xff,
            0xff21 => self.nr42,
            0xff22 => self.nr43,
            0xff23 => self.nr44 | 0xbf,
            0xff24 => self.nr50,
            0xff25 => self.nr51,
            0xff26 => self.nr52 | 0x70,
            0xff30..=0xff3f => self.wave[(addr - 0xff30) as usize],
            _ => return Err(anyhow!("Apu: invalid register write: {addr:04x?}")),
        };

        Ok(v)
    }
}

#[derive(Debug, Default)]
pub struct Apu {
    pub registers: ApuRegisters,
    div_apu: u8,
    ch1_enabled: bool,
    ch2_enabled: bool,
    ch3_enabled: bool,
    ch4_enabled: bool,
}
impl Apu {
    pub fn new() -> Self {
        Default::default()
    }

    fn clock_ch1(&mut self) {
        // sweep
        let pace = (self.registers.nr10 & 0b0111_0000) >> 4;
        let direction = (self.registers.nr10 & 0b0000_1000) >> 4;
        let individual_step = (self.registers.nr10 & 0b0000_0111) >> 4;
        let lt = (((self.registers.nr14 as u16) & 0b0000_0111) << 8) | self.registers.nr13 as u16;
        let step = 2u8.pow(individual_step as u32);
        let d_lt = lt / step as u16;
        let lt_next = if direction == 0 { lt + d_lt } else { lt - d_lt };
        if lt_next > 0x7ff && direction == 0 {
            // would overflow & direction is addition
            // pandocs: this happens when pace is 0 as well

            // TODO: turn off the channel
            self.ch1_enabled = false;
        } else if pace != 0 && (self.div_apu % (pace * 4) == 0) {
            let lt_next_high = ((lt_next & 0xff00) >> 8) as u8;
            let lt_next_low = (lt_next & 0x00ff) as u8;
            self.registers.nr13 = lt_next_low;
            self.registers.nr14 = (self.registers.nr14 & 0b1111_1000) | lt_next_high;
        }
    }

    pub fn clock(&mut self, div: u8) {
        // TODO: check the falling edge instead of mod
        if (div % 0b0010_0000) != 0 {
            return;
        }
        if (self.registers.nr52 & 0b1000_0000) == 0 {
            // apu is disabled, don't do anything
            return;
        }

        self.clock_ch1();

        self.registers.nr52 = (self.registers.nr52 & 0b1111_0000) | {
            let mut enabled = 0;
            if self.ch1_enabled {
                enabled |= 0b1
            }
            if self.ch2_enabled {
                enabled |= 0b1 << 1
            }
            if self.ch3_enabled {
                enabled |= 0b1 << 2
            }
            if self.ch4_enabled {
                enabled |= 0b1 << 3
            }
            enabled
        };
        self.div_apu = self.div_apu.wrapping_add(1);
    }
}
