use std::fmt::Debug;

pub mod mbc1;
pub mod mbc3;
pub mod rom_only;

pub trait Mbc: Debug {
    fn read(&self, addr: u16) -> anyhow::Result<u8>;
    fn write(&mut self, addr: u16, val: u8) -> anyhow::Result<()>;
}
