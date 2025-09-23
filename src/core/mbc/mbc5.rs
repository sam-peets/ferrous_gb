use anyhow::anyhow;

use crate::core::mbc::Mbc;

#[derive(Debug)]
pub struct Mbc5 {
    rom: Vec<Vec<u8>>,
    ram: Vec<Vec<u8>>,
    rom_bank_low: u8,
    rom_bank_high: u8,
    ram_bank: u8,
    ram_enable: bool,
    rom_banks: usize,
    ram_banks: usize,
    battery: bool,
}

impl Mbc for Mbc5 {
    fn read(&self, addr: u16) -> anyhow::Result<u8> {
        let addr = addr as usize;
        let bank = (((self.rom_bank_high & 1) as usize) << 8) | self.rom_bank_low as usize;
        match addr {
            0x0000..=0x3fff => Ok(self.rom[0][addr]),
            0x4000..=0x7fff => Ok(self.rom[bank % self.rom_banks][addr - 0x4000]),
            0xa000..=0xbfff => {
                if self.ram_enable {
                    Ok(self.ram[self.ram_bank as usize][addr - 0xa000])
                } else {
                    Ok(0xff)
                }
            }
            _ => Err(anyhow!("Mbc5: invalid read: {addr:04x?}")),
        }
    }

    fn write(&mut self, addr: u16, val: u8) -> anyhow::Result<()> {
        let addr = addr as usize;
        match addr {
            0x0000..=0x1fff => {
                // RAM enable
                log::debug!("Mbc5: ram enable write: {val:02x?}");
                let v = val & 0x0f;
                self.ram_enable = v == 0x0a;
                Ok(())
            }
            0x2000..=0x2fff => {
                log::debug!("Mbc5: write rom_bank_low: {val:02x?}");
                self.rom_bank_low = val;
                Ok(())
            }
            0x3000..=0x3fff => {
                log::debug!("Mbc5: write bank2: {val:02x?}");
                let val = val & 0b0000_0001;
                self.rom_bank_high = val;
                Ok(())
            }
            0x4000..=0x5fff => {
                log::debug!("Mbc5: write ram bank: {val:02x?}");
                let val = val & 0x0f;
                self.ram_bank = val;
                Ok(())
            }

            0xa000..=0xbfff => {
                if self.ram_enable {
                    self.ram[self.ram_bank as usize][addr - 0xa000] = val;
                    Ok(())
                } else {
                    Ok(())
                }
            }
            _ => Err(anyhow!("Mbc5: invalid write: {addr:04x?}")),
        }
    }
}

impl Mbc5 {
    pub fn new(rom: Vec<u8>, rom_banks: usize, ram_banks: usize, battery: bool) -> Self {
        let rom = rom.chunks(0x4000).map(|x| x.to_vec()).collect();
        let ram = vec![vec![0; 0x2000]; ram_banks];
        Self {
            rom,
            ram,
            ram_enable: false,
            rom_banks,
            ram_banks,
            battery,
            ram_bank: 0,
            rom_bank_low: 1,
            rom_bank_high: 0,
        }
    }
}
