#[derive(Debug, Default)]
pub struct Sweep {
    pub pace: u8,
    pub direction: u8,
    pub shift: u8,

    pub period_shadow: u16,
    pub enabled: bool,
    pub timer: u8,
    pub used_negative: bool,
}

impl Sweep {
    /// Returns a 2-tuple describing
    #[must_use]
    pub fn clock(&mut self, enabled: bool) -> (Option<u16>, Option<bool>) {
        let ret = if enabled && self.pace != 0 && self.enabled && self.timer == 0 {
            log::debug!(
                "Ch1: clock sweep: shadow: {:04x?}, shift: {}, direction: {}, pace: {}",
                self.period_shadow,
                self.shift,
                self.direction,
                self.pace,
            );
            let (n_period, update_enabled) = self.overflow_check();
            log::debug!("Ch1: clock_sweep: n_period: {n_period:04x?}");
            let mut update_period = None;
            let mut update_enabled_2 = None;
            if self.shift != 0 {
                // write back to shadow and period
                self.period_shadow = n_period & 0x7ff;
                update_period = Some(self.period_shadow);
                let (_, upd) = self.overflow_check();
                update_enabled_2 = upd;
            }

            let update_enabled = {
                if update_enabled.is_some() {
                    update_enabled
                } else if update_enabled_2.is_some() {
                    update_enabled_2
                } else {
                    None
                }
            };
            (update_period, update_enabled)
        } else {
            (None, None)
        };
        let (n_sweep_timer, underflow) = self.timer.overflowing_sub(1);
        self.timer = if underflow {
            if self.pace == 0 { 7 } else { self.pace - 1 }
        } else {
            n_sweep_timer
        };
        ret
    }
    //
    #[must_use]
    pub fn overflow_check(&mut self) -> (u16, Option<bool>) {
        log::debug!(
            "Ch1: overflow check: shadow: {:04x?}, shift: {}, direction: {}, pace: {}",
            self.period_shadow,
            self.shift,
            self.direction,
            self.pace,
        );
        let d_period = self.period_shadow >> self.shift;
        let n_period = if self.direction == 0 {
            self.period_shadow + d_period
        } else {
            log::debug!("Ch1: seen negative mode");
            self.used_negative = true;
            self.period_shadow - d_period
        };
        log::debug!("Ch1: n_period: 0x{n_period:04x?}");
        if n_period > 0x07ff {
            log::debug!("Ch1: sweep overflow");
            (n_period & 0x7ff, Some(false))
        } else {
            (n_period & 0x7ff, None)
        }
    }

    pub fn trigger(&mut self, period: u16) -> Option<bool> {
        self.used_negative = false;
        self.period_shadow = period;
        self.timer = if self.pace == 0 { 7 } else { self.pace - 1 };
        self.enabled = self.pace > 0 || self.shift > 0;
        let update_enabled = if self.shift > 0 {
            log::debug!("sweep: overflow check on trigger");
            let (_, update_enabled) = self.overflow_check();
            update_enabled
        } else {
            None
        };
        log::debug!("sweep: sweep_enabled: {}", self.enabled);
        update_enabled
    }
}
