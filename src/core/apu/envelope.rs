#[derive(Debug, Default)]
pub struct Envelope {
    pub initial_volume: u8,
    pub direction: u8,
    pub pace: u8,
    pub volume: u8,
    pub timer: u8,
}

impl Envelope {
    pub fn trigger(&mut self) {
        self.timer = self.pace;
        self.volume = self.initial_volume;
    }
    pub fn clock(&mut self) {
        if self.pace == 0 {
            // envelope is disabled
            return;
        }
        if self.timer > 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.timer = self.pace;
            if self.direction == 1 && self.volume < 0xf {
                // decrease
                self.volume += 1;
            } else if self.volume > 0x0 {
                // increase
                self.volume -= 1;
            }
        }
    }
}
