struct Apu {
    ch1_sweep: u8,
    ch1_length_duty: u8,
    ch1_volume_envelope: u8,
    ch1_period_low: u8,
    ch1_period_high_control: u8,
    ch2_length_duty: u8,
    ch2_volume_envelope: u8,
    ch2_period_low: u8,
    ch2_period_high_control: u8,
    ch3_dac_enable: u8,
    ch3_length: u8,
    ch3_output: u8,
    ch3_period_low: u8,
    ch3_period_high_control: u8,
    ch4_length: u8,
    ch4_volume_envelope: u8,
    ch4_frequency_randomness: u8,
    ch4_control: u8,
    master_volume_vin_panning: u8,
    sound_panning: u8,
    sound_toggle: u8,
    wave: u8,
}

impl Apu {
    pub fn clock() {}
}
