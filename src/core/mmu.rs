use anyhow::anyhow;

use crate::core::{Buttons, Mode, apu::Apu, mbc::CartridgeHeader};

const BOOT: &[u8] = include_bytes!("../../assets/bootix_dmg.bin");

#[derive(Default, Debug)]
pub struct IoRegisters {
    pub joyp: u8,      // 0xff00
    pub sc: u8,        // 0xff02
    pub tima: u8,      // 0xff05
    pub tma: u8,       // 0xff06
    pub tac: u8,       // 0xff07
    pub interrupt: u8, // 0xff0f
    pub lcdc: u8,      // 0xff40
    pub stat: u8,      // 0xff41
    pub scy: u8,       // 0xff42
    pub scx: u8,       // 0xff43
    pub ly: u8,        // 0xff44
    pub lyc: u8,       // 0xff45
    pub dma: u8,       // 0xff46
    pub bgp: u8,       // 0xff47
    pub obp0: u8,      // 0xff48
    pub obp1: u8,      // 0xff49
    pub wy: u8,        // 0xff4a
    pub wx: u8,        // 0xff4b
    pub bank: u8,      // 0xff50 - bootrom mapping control
}

#[derive(Debug)]
pub struct Mmu {
    pub ie: u8,
    vram: Vec<u8>,
    wram: Vec<u8>,
    oam: Vec<u8>,
    hram: Vec<u8>,
    pub io: IoRegisters,
    pub dma_requsted: bool,
    pub buttons: Buttons,
    pub ppu_mode: Mode,
    pub cartridge: CartridgeHeader,
    pub sys: u16,
    pub apu: Apu,
}

impl Mmu {
    pub fn new(rom: &[u8], sample_rate: u32) -> anyhow::Result<Self> {
        let io = IoRegisters {
            ..Default::default()
        };

        let header = CartridgeHeader::new(rom)?;

        let mmu = Self {
            io,
            ie: 0,
            vram: vec![0; 0x2000],
            wram: vec![0; 0x2000],
            oam: vec![0; 0x100],
            hram: vec![0; 0x7f],
            dma_requsted: false,
            buttons: Buttons::default(),
            ppu_mode: Mode::OamScan,
            cartridge: header,
            sys: 0,
            apu: Apu::new(sample_rate),
        };
        Ok(mmu)
    }

    pub fn read(&self, addr: u16) -> anyhow::Result<u8> {
        log::trace!("read: reading {addr:x?}");
        let a = addr as usize;
        match a {
            0x0..=0x7fff | 0xa000..=0xbfff => {
                if (0x0..=0xff).contains(&a) && self.io.bank == 0 {
                    Ok(BOOT[a])
                } else {
                    Ok(self.cartridge.mbc.read(addr)?)
                }
            }
            0x8000..=0x9fff => Ok(self.vram[a - 0x8000]),
            0xc000..=0xdfff => Ok(self.wram[a - 0xc000]),
            0xe000..=0xfdff => self.read(addr - 0x2000), // echo ram
            0xfe00..=0xfe9f => Ok(self.oam[a - 0xfe00]),
            // 0xfea0..=0xfeff => Err(anyhow!("prohibited read at {a:x?}")),
            0xfea0..=0xfeff => Ok(0xff), // invalid read, just return 0xff
            0xff00..=0xff7f => match a {
                0xff00 => {
                    let joyp = self.io.joyp & 0b0011_0000;
                    let buttons = self.buttons.as_joyp(joyp);
                    Ok(joyp | buttons)
                }
                0xff02 => Ok(self.io.sc),
                0xff04 => Ok(((self.sys & 0xff00) >> 8) as u8),
                0xff05 => Ok(self.io.tima),
                0xff06 => Ok(self.io.tma),
                0xff07 => Ok(self.io.tac),
                0xff0f => Ok(self.io.interrupt),
                0xff10..=0xff14 | 0xff16..=0xff1e | 0xff20..=0xff26 | 0xff30..=0xff3f => {
                    self.apu.read(addr, self.sys)
                }
                0xff40 => Ok(self.io.lcdc),
                0xff41 => {
                    let mut v = self.io.stat;
                    if self.io.ly == self.io.lyc {
                        v |= 0b0000_0100;
                    }
                    v |= self.ppu_mode as u8;
                    Ok(v)
                }
                0xff42 => Ok(self.io.scy),
                0xff43 => Ok(self.io.scx),
                0xff44 => Ok(self.io.ly),
                // 0xff44 => Ok(0x90),
                0xff45 => Ok(self.io.lyc),
                0xff46 => Ok(self.io.dma),
                0xff47 => Ok(self.io.bgp),
                0xff48 => Ok(self.io.obp0),
                0xff49 => Ok(self.io.obp1),
                0xff4a => Ok(self.io.wy),
                0xff4b => Ok(self.io.wx),
                0xff10..=0xff3f => {
                    log::trace!("FIXME: mmu: sound register read: {a:x?}");
                    Ok(0xff)
                }
                _ => {
                    log::warn!("unimplemented IO reg read at {a:x?}");
                    Ok(0xff)
                }
            },
            0xff80..=0xfffe => Ok(self.hram[a - 0xff80]),
            0xffff => Ok(self.ie),
            a => Err(anyhow!("read out of bounds at {a:x?}")),
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) -> anyhow::Result<()> {
        log::trace!("write: writing {val:x?} to {addr:x?}");
        let a = addr as usize;
        match a {
            0x0..=0x7fff | 0xa000..=0xbfff => Ok(self.cartridge.mbc.write(addr, val)?),
            0x8000..=0x9fff => {
                self.vram[a - 0x8000] = val;
                Ok(())
            }
            0xc000..=0xdfff => {
                self.wram[a - 0xc000] = val;
                Ok(())
            }
            0xe000..=0xfdff => {
                log::warn!("writing {val:x?} to echo ram at {a:x?}, is this ok?");
                self.write(addr - 0x2000, val)
            }
            0xfe00..=0xfe9f => {
                self.oam[a - 0xfe00] = val;
                Ok(())
            }
            0xfea0..=0xfeff => Ok(()), // invalid write, do nothing
            0xff00..=0xff7f => match a {
                0xff00 => {
                    self.io.joyp = val & 0b00110000;
                    Ok(())
                }
                0xff01 => {
                    // TODO: serial, logging for now for blargg
                    // print!("{}", val as char);
                    Ok(())
                }
                0xff02 => {
                    self.io.sc = val;
                    Ok(())
                }
                0xff04 => {
                    // any write resets the divider/system clock to 0
                    self.sys = 0;
                    Ok(())
                }
                0xff05 => {
                    self.io.tima = val;
                    Ok(())
                }
                0xff06 => {
                    self.io.tma = val;
                    Ok(())
                }
                0xff07 => {
                    self.io.tac = val;
                    Ok(())
                }
                0xff0f => {
                    log::debug!("mmu: IF write: 0x{val:x?}");
                    self.io.interrupt = val;
                    Ok(())
                }
                0xff40 => {
                    log::debug!("mmu: LCDC write: 0x{val:x?}");
                    self.io.lcdc = val;
                    Ok(())
                }
                0xff41 => {
                    log::debug!("mmu: STAT write: 0x{val:x?}");
                    self.io.stat = val & 0b01111000;
                    Ok(())
                }
                0xff42 => {
                    log::debug!("mmu: SCY write: 0x{val:x?}");
                    self.io.scy = val;
                    Ok(())
                }
                0xff43 => {
                    log::debug!("mmu: SCX write: 0x{val:x?}");
                    self.io.scx = val;
                    Ok(())
                }
                0xff44 => {
                    // read-only
                    // self.io.ly = val;
                    Ok(())
                }
                0xff45 => {
                    log::debug!("mmu: LYC write: 0x{val:x?}");
                    self.io.lyc = val;
                    Ok(())
                }
                0xff46 => {
                    self.io.dma = val;
                    self.dma_requsted = true;
                    Ok(())
                }
                0xff47 => {
                    log::debug!("mmu: BGP write: 0x{val:x?}");
                    self.io.bgp = val;
                    Ok(())
                }
                0xff48 => {
                    self.io.obp0 = val;
                    Ok(())
                }
                0xff49 => {
                    self.io.obp1 = val;
                    Ok(())
                }
                0xff4a => {
                    self.io.wy = val;
                    Ok(())
                }
                0xff4b => {
                    self.io.wx = val;
                    Ok(())
                }
                0xff50 => {
                    self.io.bank = val;
                    Ok(())
                }
                0xff10..=0xff14 | 0xff16..=0xff1e | 0xff20..=0xff26 | 0xff30..=0xff3f => {
                    self.apu.write(addr, val, self.sys)
                }

                _ => {
                    log::warn!("FIXME: mmu: write: unimplemented IO reg write at 0x{a:x?}");
                    Ok(())
                }
            },
            0xff80..=0xfffe => {
                self.hram[a - 0xff80] = val;
                Ok(())
            }
            0xffff => {
                log::debug!("write: IE write: 0x{val:02x?}");
                self.ie = val;
                Ok(())
            }
            a => Err(anyhow!("mmu: write: write out of bounds at {a:x?}")),
        }
    }
}
