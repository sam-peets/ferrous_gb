use anyhow::anyhow;

use crate::core::mbc::Mbc;

#[derive(Debug)]
pub struct Mbc1 {
    rom: Vec<Vec<u8>>,
    rom_bank: usize,
    ram: Vec<Vec<u8>>,
    ram_bank: usize,
    ram_enable: bool,
    rom_banks: usize,
    ram_banks: usize,
    battery: bool,
}

impl Mbc for Mbc1 {
    fn read(&self, addr: u16) -> anyhow::Result<u8> {
        let addr = addr as usize;
        match addr {
            0x0000..=0x3fff => Ok(self.rom[0][addr]),
            0x4000..=0x7fff => Ok(self.rom[self.rom_bank][addr - 0x4000]),
            0xa000..=0xbfff => Ok(self.ram[self.ram_bank][addr - 0xa000]),
            _ => Err(anyhow!("Mbc1: invalid read: {addr:04x?}")),
        }
    }

    fn write(&mut self, addr: u16, val: u8) -> anyhow::Result<()> {
        let addr = addr as usize;
        match addr {
            0x0000..=0x1fff => {
                // RAM enable
                log::debug!("Mbc1: ram enable write: {val:02x?}");
                let v = val & 0x0f;
                self.ram_enable = v == 0x0a;
                Ok(())
            }
            0x2000..=0x3fff => {
                log::debug!("Mbc1: bank switch: {val:02x?}");
                let mut val = val & 0b0001_1111;
                if val == 0 {
                    val = 1;
                }
                self.rom_bank = val as usize;
                Ok(())
            }
            0x4000..=0x5fff => {
                if self.ram_banks == 1 {
                    return Ok(());
                }
                log::debug!("Mbc1: ram switch: {val:02x?}");
                let val = val & 0b0000_0011;
                self.ram_bank = val as usize;
                // TODO: upper rom bank switches
                Ok(())
            }
            0x6000..=0x7fff => {
                todo!("Mbc1: banking mode select")
            }
            _ => Err(anyhow!("Mbc1: invalid write: {addr:04x?}")),
        }
    }
}

impl Mbc1 {
    pub fn new(rom: Vec<u8>, rom_banks: usize, ram_banks: usize, battery: bool) -> Self {
        let rom = rom.chunks(0x4000).map(|x| x.to_vec()).collect();
        let ram = vec![vec![0; 0x2000]; ram_banks];
        Self {
            rom,
            rom_bank: 0,
            ram,
            ram_bank: 0,
            ram_enable: false,
            rom_banks,
            ram_banks,
            battery,
        }
    }
}
