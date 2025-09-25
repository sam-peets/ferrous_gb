use std::ops::Not;

pub mod apu;
pub mod cpu;
pub mod mbc;
pub mod mmu;
pub mod ppu;
mod util;

#[derive(Debug, Default, Clone, Copy)]
pub enum ButtonState {
    #[default]
    Released,
    Pressed,
}

impl From<bool> for ButtonState {
    fn from(value: bool) -> Self {
        if value {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        }
    }
}

impl ButtonState {
    #[allow(dead_code)]
    pub fn is_pressed(self) -> bool {
        matches!(self, ButtonState::Pressed)
    }
    pub fn is_released(self) -> bool {
        matches!(self, ButtonState::Released)
    }
}

impl Not for ButtonState {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            ButtonState::Released => ButtonState::Pressed,
            ButtonState::Pressed => ButtonState::Released,
        }
    }
}
impl Buttons {
    pub fn as_joyp(&self, mode: u8) -> u8 {
        match mode {
            0b10 => {
                let mut r = 0;
                if self.down.is_released() {
                    r |= 0b1000;
                }
                if self.up.is_released() {
                    r |= 0b0100;
                }
                if self.left.is_released() {
                    r |= 0b0010;
                }
                if self.right.is_released() {
                    r |= 0b0001;
                }
                r
            }
            0b01 => {
                let mut r = 0;
                if self.start.is_released() {
                    r |= 0b1000;
                }
                if self.select.is_released() {
                    r |= 0b0100;
                }
                if self.b.is_released() {
                    r |= 0b0010;
                }
                if self.a.is_released() {
                    r |= 0b0001;
                }
                r
            }
            _ => 0b0000_1111,
        }
    }
}

#[derive(Debug, Default)]
pub struct Buttons {
    pub up: ButtonState,
    pub down: ButtonState,
    pub left: ButtonState,
    pub right: ButtonState,
    pub start: ButtonState,
    pub select: ButtonState,
    pub a: ButtonState,
    pub b: ButtonState,
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    HBlank = 0,
    VBlank = 1,
    OamScan = 2,
    Drawing = 3,
}
