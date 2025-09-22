pub const PATTERN_0: &[u8] = &[1, 1, 1, 1, 1, 1, 1, 0];
pub const PATTERN_1: &[u8] = &[0, 1, 1, 1, 1, 1, 1, 0];
pub const PATTERN_2: &[u8] = &[0, 1, 1, 1, 1, 0, 0, 0];
pub const PATTERN_3: &[u8] = &[1, 0, 0, 0, 0, 0, 0, 1];
pub const PATTERN_LEN: usize = PATTERN_0.len();

#[derive(Debug, Default)]
pub struct DutyCycle {
    counter: Option<u16>,
    position: usize,
    pub pattern: u8,
}

impl DutyCycle {
    pub fn sample(&self) -> u8 {
        match self.pattern {
            0b00 => PATTERN_0[self.position],
            0b01 => PATTERN_1[self.position],
            0b10 => PATTERN_2[self.position],
            0b11 => PATTERN_3[self.position],
            _ => unreachable!(),
        }
    }
    pub fn clock(&mut self, period: u16) {
        if let Some(counter) = self.counter {
            let next = counter + 1;
            if next > 0x7ff {
                self.counter = Some(period);
                self.position = (self.position + 1) % PATTERN_LEN;
            } else {
                self.counter = Some(next);
            }
        } else {
            self.counter = Some(period);
        }
    }
}
