use anyhow::anyhow;

use crate::core::mbc::Mbc;

#[derive(Debug)]
pub struct RomOnly {
    rom: Vec<u8>,
    ram: Vec<u8>,
}

impl TryFrom<&[u8]> for RomOnly {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let rom = value[0..=0x7fff].to_vec();
        let ram = vec![0u8; 0x2000];
        Ok(RomOnly { rom, ram })
    }
}

impl Mbc for RomOnly {
    fn read(&self, addr: u16) -> anyhow::Result<u8> {
        let addr = addr as usize;
        match addr {
            0..=0x7fff => Ok(self.rom[addr]),
            0xa000..=0xbfff => Ok(self.ram[addr - 0xa000]),
            _ => Err(anyhow!("NoMapper: invalid read: 0x{addr:04x?}")),
        }
    }

    fn write(&mut self, addr: u16, val: u8) -> anyhow::Result<()> {
        let addr = addr as usize;
        match addr {
            0..=0x7fff => Ok(()), // do nothing
            0xa000..=0xbfff => {
                self.ram[addr - 0xa000] = val;
                Ok(())
            }
            _ => Err(anyhow!("NoMapper: invalid write: 0x{addr:04x?}")),
        }
    }
}
