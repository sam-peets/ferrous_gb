pub mod cpu;
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

pub struct CartridgeHeader {
    pub title: String,
    pub cartridge_type: u8,
    pub rom_size: u8,
    pub ram_size: u8,
}

impl CartridgeHeader {
    pub fn new(rom: Vec<u8>) -> anyhow::Result<Self> {
        let title = String::from_utf8(rom[0x134..=0x143].to_vec())?;
        let cartridge_type = rom[0x147];
        let rom_size = rom[0x148];
        let ram_size = rom[0x149];

        Ok(CartridgeHeader {
            title,
            cartridge_type,
            rom_size,
            ram_size,
        })
    }
}
