pub mod mmio;

use crate::core::{
    Buttons, Memory, Mode, apu::Apu, mbc::CartridgeHeader, mmu::mmio::Mmio, ppu::Ppu,
};

const BOOT: &[u8] = include_bytes!("../../../assets/bootix_dmg.bin");

#[derive(Debug)]
pub struct Mmu {
    pub ie: u8,
    wram: Vec<u8>,
    hram: Vec<u8>,
    pub mmio: Mmio,
    pub cartridge: CartridgeHeader,
}

impl Mmu {
    pub fn new(rom: &[u8], sample_rate: u32) -> anyhow::Result<Self> {
        let io = Mmio::new(sample_rate);

        let header = CartridgeHeader::new(rom)?;

        let mmu = Self {
            mmio: io,
            ie: 0,
            wram: vec![0; 0x2000],
            hram: vec![0; 0x7f],
            cartridge: header,
        };
        Ok(mmu)
    }

    pub fn read(&self, addr: u16) -> u8 {
        log::trace!("read: reading {addr:x?}");
        let a = addr as usize;
        match a {
            0x0..=0x7fff | 0xa000..=0xbfff => {
                if (0x0..=0xff).contains(&a) && self.mmio.read(mmio::BANK) == 0 {
                    BOOT[a]
                } else {
                    self.cartridge.mbc.read(addr)
                }
            }
            0x8000..=0x9fff | 0xfe00..=0xfe9f => self.mmio.read(addr),
            0xc000..=0xdfff => self.wram[a - 0xc000],
            0xe000..=0xfdff => self.read(addr - 0x2000), // echo ram
            // 0xfea0..=0xfeff => Err(anyhow!("prohibited read at {a:x?}")),
            0xfea0..=0xfeff => 0xff, // invalid read, just return 0xff
            0xff00..=0xff7f => self.mmio.read(addr),
            0xff80..=0xfffe => self.hram[a - 0xff80],
            0xffff => self.ie,
            _ => {
                log::warn!("read out of bounds at 0x{addr:04x?}");
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        log::trace!("write: writing {val:x?} to {addr:x?}");
        let a = addr as usize;
        match a {
            0x0..=0x7fff | 0xa000..=0xbfff => self.cartridge.mbc.write(addr, val),
            0xc000..=0xdfff => {
                self.wram[a - 0xc000] = val;
            }
            0xe000..=0xfdff => {
                log::warn!("writing {val:x?} to echo ram at {a:x?}, is this ok?");
                self.write(addr - 0x2000, val)
            }
            0xfea0..=0xfeff => (), // invalid write, do nothing
            0xff00..=0xff7f => self.mmio.write(addr, val),
            0x8000..=0x9fff | 0xfe00..=0xfe9f => self.mmio.write(addr, val),
            0xff80..=0xfffe => {
                self.hram[a - 0xff80] = val;
            }
            0xffff => {
                log::debug!("write: IE write: 0x{val:02x?}");
                self.ie = val;
            }
            _ => log::warn!("mmu: write: write out of bounds at {a:x?}"),
        }
    }
}
