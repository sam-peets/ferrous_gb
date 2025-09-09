use std::{
    ops::{AddAssign, DivAssign, MulAssign, SubAssign},
    process::Output,
};

#[derive(Copy, Clone, Debug, Default)]
pub struct Register16 {
    pub high: Register8,
    pub low: Register8,
}

impl AddAssign<u16> for Register16 {
    fn add_assign(&mut self, rhs: u16) {
        let s: u16 = (*self).into();
        *self = (s + rhs).into()
    }
}

impl SubAssign<u16> for Register16 {
    fn sub_assign(&mut self, rhs: u16) {
        let s: u16 = (*self).into();
        *self = (s - rhs).into()
    }
}

impl MulAssign<u16> for Register16 {
    fn mul_assign(&mut self, rhs: u16) {
        let s: u16 = (*self).into();
        *self = (s * rhs).into()
    }
}

impl DivAssign<u16> for Register16 {
    fn div_assign(&mut self, rhs: u16) {
        let s: u16 = (*self).into();
        *self = (s / rhs).into()
    }
}

impl Register for Register16 {
    type Item = u16;

    fn read(&self) -> Self::Item {
        (*self).into()
    }

    fn write(&mut self, val: Self::Item) {
        *self = val.into();
    }
}

impl From<Register16> for u16 {
    fn from(val: Register16) -> Self {
        let high: u8 = val.high.into();
        let high = high as u16;
        let low: u8 = val.low.into();
        let low = low as u16;
        (high << 8) | low
    }
}

impl From<u16> for Register16 {
    fn from(value: u16) -> Self {
        let high = ((value & 0xff00) >> 8) as u8;
        let low = (value & 0x00ff) as u8;

        Register16 {
            high: high.into(),
            low: low.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct FlagRegister {
    pub z: bool,
    pub n: bool,
    pub h: bool,
    pub c: bool,
}

impl From<FlagRegister> for u8 {
    fn from(val: FlagRegister) -> Self {
        let mut x = 0;
        if val.z {
            x |= 1 << 7;
        }
        if val.n {
            x |= 1 << 6;
        }
        if val.h {
            x |= 1 << 5;
        }
        if val.c {
            x |= 1 << 4;
        }
        x
    }
}

impl From<u8> for FlagRegister {
    fn from(value: u8) -> Self {
        let z = (value & (1 << 7)) > 0;
        let n = (value & (1 << 6)) > 0;
        let h = (value & (1 << 5)) > 0;
        let c = (value & (1 << 4)) > 0;
        FlagRegister { z, n, h, c }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Register8(pub u8);

impl From<Register8> for u8 {
    fn from(value: Register8) -> Self {
        value.0
    }
}

impl From<u8> for Register8 {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct RegisterAF {
    pub high: Register8,
    pub low: FlagRegister,
}

impl From<u16> for RegisterAF {
    fn from(value: u16) -> Self {
        let high = ((value & 0xff00) >> 8) as u8;
        let low = (value & 0x00ff) as u8;
        RegisterAF {
            high: high.into(),
            low: low.into(),
        }
    }
}

impl From<RegisterAF> for u16 {
    fn from(value: RegisterAF) -> Self {
        let high: u8 = value.high.into();
        let high: u16 = high as u16;
        let low: u8 = value.low.into();
        let low: u16 = low as u16;
        (high << 8) | low
    }
}

impl Register for RegisterAF {
    type Item = u16;

    fn read(&self) -> Self::Item {
        (*self).into()
    }

    fn write(&mut self, val: Self::Item) {
        *self = val.into();
    }
}

pub trait Register {
    type Item;
    fn read(&self) -> Self::Item;
    fn write(&mut self, val: Self::Item);
}

impl Register for Register8 {
    type Item = u8;
    fn read(&self) -> Self::Item {
        self.0
    }

    fn write(&mut self, val: Self::Item) {
        self.0 = val;
    }
}

impl Register for FlagRegister {
    type Item = u8;

    fn read(&self) -> Self::Item {
        (*self).into()
    }

    fn write(&mut self, val: Self::Item) {
        *self = val.into();
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CpuRegisters {
    pub af: RegisterAF,
    pub bc: Register16,
    pub de: Register16,
    pub hl: Register16,
    pub sp: Register16,
    pub pc: Register16,
}

impl CpuRegisters {
    pub fn get_r16(&mut self, code: u8) -> &mut dyn Register<Item = u16> {
        match code {
            0b00 => &mut self.bc,
            0b01 => &mut self.de,
            0b10 => &mut self.hl,
            0b11 => &mut self.sp,
            _ => unreachable!(),
        }
    }

    pub fn get_r8(&mut self, code: u8) -> &mut dyn Register<Item = u8> {
        match code {
            0b000 => &mut self.bc.high,
            0b001 => &mut self.bc.low,
            0b010 => &mut self.de.high,
            0b011 => &mut self.de.low,
            0b100 => &mut self.hl.high,
            0b101 => &mut self.hl.low,
            0b111 => &mut self.af.high,
            _ => unreachable!(),
        }
    }

    pub fn get_cond(&self, code: u8) -> bool {
        match code {
            0b00 => self.af.low.z == false,
            0b01 => self.af.low.z == true,
            0b10 => self.af.low.c == false,
            0b11 => self.af.low.c == true,
            _ => unreachable!("get_cond: unknown code: 0x{code:x?}"),
        }
    }
}
