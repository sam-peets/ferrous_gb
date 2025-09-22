use num_traits::{PrimInt, WrappingSub};

#[derive(Debug, Default, Clone, Copy)]
pub struct Length<T: PrimInt> {
    pub length: T,
    pub enable: bool,
}

impl<T: PrimInt + WrappingSub> Length<T> {
    /// Clocks the length and returns true if it expired
    pub fn clock(&mut self) -> bool {
        if self.enable && self.length > T::zero() {
            // log::debug!("ch1: clock length: {} -> {}", self.length, new_length);
            self.length = self.length.wrapping_sub(&T::one());
            self.length == T::zero()
        } else {
            false
        }
    }

    pub fn write_nrx4(&mut self, length_enable: bool, div_apu: u8) -> bool {
        let prev_enabled = self.enable;
        self.enable = length_enable;
        if ((div_apu % 2) == 1) && self.enable && !prev_enabled && self.length != T::zero() {
            log::debug!("Length: clocking length from NRx4 write: div_apu: {div_apu:?}");
            self.clock()
        } else {
            false
        }
    }

    pub fn trigger(&mut self, reset: T, div_apu: u8) {
        if self.length == T::zero() {
            self.length = if (div_apu % 2) == 1 && self.enable {
                reset - T::one()
            } else {
                reset
            };
        }
    }
    pub fn clear(&mut self) {
        // length shouldn't be reset
        self.enable = false;
    }
}
