use anyhow::anyhow;

use crate::core::mbc::Mbc;

#[derive(Debug)]
pub struct Mbc1 {
    rom: Vec<Vec<u8>>,
    ram: Vec<Vec<u8>>,
    bank1: u8,
    bank2: u8,
    ram_enable: bool,
    rom_banks: usize,
    ram_banks: usize,
    battery: bool,
    mode: u8,
}

impl Mbc for Mbc1 {
    fn read(&self, addr: u16) -> anyhow::Result<u8> {
        let addr = addr as usize;
        let bank1 = self.bank1 as usize;
        let bank2 = self.bank2 as usize;
        match addr {
            0x0000..=0x3fff => match self.mode {
                0 => Ok(self.rom[0][addr]),
                1 => Ok(self.rom[(bank2 << 5) % self.rom_banks][addr]),
                _ => unreachable!("Mbc1: invalid addressing mode"),
            },
            0x4000..=0x7fff => Ok(self.rom[((bank2 << 5) | bank1) % self.rom_banks][addr - 0x4000]),
            0xa000..=0xbfff => {
                if self.ram_enable {
                    match self.mode {
                        0 => Ok(self.ram[0][addr - 0xa000]),
                        1 => Ok(self.ram[bank2 % self.ram_banks][addr - 0xa000]),
                        _ => unreachable!("Mbc1: invalid addressing mode"),
                    }
                } else {
                    Ok(0xff)
                }
            }
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
                log::debug!("Mbc1: write bank1: {val:02x?}");
                let mut val = val & 0b0001_1111;
                if val == 0 {
                    val = 1;
                }
                self.bank1 = val;
                Ok(())
            }
            0x4000..=0x5fff => {
                if self.ram_banks == 1 {
                    return Ok(());
                }
                log::debug!("Mbc1: write bank2: {val:02x?}");
                let val = val & 0b0000_0011;
                self.bank2 = val;
                // TODO: upper rom bank switches
                Ok(())
            }
            0x6000..=0x7fff => {
                log::debug!("Mbc1: write mode: {val:02x?}");
                self.mode = val & 0b0000_0001;
                Ok(())
            }
            0xa000..=0xbfff => {
                if self.ram_enable {
                    let bank2 = self.bank2 as usize;
                    match self.mode {
                        0 => self.ram[0][addr - 0xa000] = val,
                        1 => self.ram[bank2][addr - 0xa000] = val,
                        _ => unreachable!("Mbc1: invalid addressing mode"),
                    }
                    Ok(())
                } else {
                    Ok(())
                }
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
            ram,
            ram_enable: false,
            rom_banks,
            ram_banks,
            battery,
            bank1: 1,
            bank2: 0,
            mode: 0,
        }
    }
}
