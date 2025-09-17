use anyhow::anyhow;

use crate::core::mbc::{Mbc, mbc1::Mbc1, mbc3::Mbc3, rom_only::RomOnly};

pub mod cpu;
pub mod mbc;
pub mod mmu;
mod ppu;
pub mod register;

#[derive(Debug, Default)]
pub struct Buttons {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub start: bool,
    pub select: bool,
    pub a: bool,
    pub b: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    HBlank = 0,
    OamScan = 1,
    Drawing = 2,
    VBlank = 3,
}

#[derive(Debug)]
pub struct CartridgeHeader {
    pub title: String,
    pub cartridge_type: Mapper,
    pub rom_banks: usize,
    pub ram_banks: usize,
    pub mbc: Box<dyn Mbc>,
}

impl CartridgeHeader {
    pub fn new(rom: &Vec<u8>) -> anyhow::Result<Self> {
        let title = String::from_utf8_lossy(&rom[0x134..=0x143]).to_string();
        let cartridge_type = rom[0x147].try_into()?;
        let rom_banks = match rom[0x148] {
            0x00 => 2,
            0x01 => 4,
            0x02 => 8,
            0x03 => 16,
            0x04 => 32,
            0x05 => 64,
            0x06 => 128,
            0x07 => 256,
            0x08 => 512,
            0x52 => 72,
            0x53 => 80,
            0x54 => 96,
            _ => return Err(anyhow!("CartridgeHeader: unknown ROM size")),
        };
        let ram_banks = match rom[0x149] {
            0x00 => 1, // technically zero, but making it 1 here for compat with no-mapper
            0x02 => 1,
            0x03 => 4,
            0x04 => 16,
            0x05 => 8,
            _ => return Err(anyhow!("CartridgeHeader: unknown RAM size")),
        };

        let mbc: Box<dyn Mbc> = match cartridge_type {
            Mapper::RomOnly => Box::new(RomOnly::try_from(rom.clone())?),
            Mapper::Mbc1 => Box::new(Mbc1::new(rom.clone(), rom_banks, ram_banks, false)),
            Mapper::Mbc1Ram => Box::new(Mbc1::new(rom.clone(), rom_banks, ram_banks, false)),
            Mapper::Mbc1RamBattery => Box::new(Mbc1::new(rom.clone(), rom_banks, ram_banks, true)),
            Mapper::Mbc3RamBattery => {
                Box::new(Mbc3::new(rom.clone(), rom_banks, ram_banks, true, false))
            }
            Mapper::Mbc3TimerRamBattery => {
                Box::new(Mbc3::new(rom.clone(), rom_banks, ram_banks, true, true))
            }
            Mapper::Mbc3 => Box::new(Mbc3::new(rom.clone(), rom_banks, ram_banks, false, false)),
            m => todo!("mmu: unimplemented mapper: {m:?}"),
        };

        let header = CartridgeHeader {
            title,
            cartridge_type,
            rom_banks,
            ram_banks,
            mbc,
        };
        log::info!("CartridgeHeader: new: {header:?}");
        Ok(header)
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub enum Mapper {
    RomOnly = 0x00,
    Mbc1 = 0x01,
    Mbc1Ram = 0x02,
    Mbc1RamBattery = 0x03,
    Mbc3TimerRamBattery = 0x10,
    Mbc3 = 0x11,
    Mbc3RamBattery = 0x13,
}

impl TryFrom<u8> for Mapper {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Mapper::RomOnly),
            0x01 => Ok(Mapper::Mbc1),
            0x02 => Ok(Mapper::Mbc1Ram),
            0x03 => Ok(Mapper::Mbc1RamBattery),
            0x10 => Ok(Mapper::Mbc3TimerRamBattery),
            0x11 => Ok(Mapper::Mbc3),
            0x13 => Ok(Mapper::Mbc3RamBattery),
            _ => Err(anyhow!("unknown mapper: 0x{value:02x?}")),
        }
    }
}
