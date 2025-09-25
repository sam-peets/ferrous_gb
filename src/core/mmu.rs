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

    pub fn read(&self, addr: u16) -> u8 {
        log::trace!("read: reading {addr:x?}");
        let a = addr as usize;
        match a {
            0x0..=0x7fff | 0xa000..=0xbfff => {
                if (0x0..=0xff).contains(&a) && self.io.bank == 0 {
                    BOOT[a]
                } else {
                    self.cartridge.mbc.read(addr)
                }
            }
            0x8000..=0x9fff => self.vram[a - 0x8000],
            0xc000..=0xdfff => self.wram[a - 0xc000],
            0xe000..=0xfdff => self.read(addr - 0x2000), // echo ram
            0xfe00..=0xfe9f => self.oam[a - 0xfe00],
            // 0xfea0..=0xfeff => Err(anyhow!("prohibited read at {a:x?}")),
            0xfea0..=0xfeff => 0xff, // invalid read, just return 0xff
            0xff00..=0xff7f => match a {
                0xff00 => {
                    let joyp = self.io.joyp & 0b0011_0000;
                    let buttons = self.buttons.as_joyp(joyp);
                    joyp | buttons
                }
                0xff02 => self.io.sc,
                0xff04 => ((self.sys & 0xff00) >> 8) as u8,
                0xff05 => self.io.tima,
                0xff06 => self.io.tma,
                0xff07 => self.io.tac,
                0xff0f => self.io.interrupt,
                0xff10..=0xff14 | 0xff16..=0xff1e | 0xff20..=0xff26 | 0xff30..=0xff3f => {
                    self.apu.read(addr, self.sys)
                }
                0xff40 => self.io.lcdc,
                0xff41 => {
                    let mut v = self.io.stat;
                    if self.io.ly == self.io.lyc {
                        v |= 0b0000_0100;
                    }
                    v |= self.ppu_mode as u8;
                    v
                }
                0xff42 => self.io.scy,
                0xff43 => self.io.scx,
                0xff44 => self.io.ly,
                0xff45 => self.io.lyc,
                0xff46 => self.io.dma,
                0xff47 => self.io.bgp,
                0xff48 => self.io.obp0,
                0xff49 => self.io.obp1,
                0xff4a => self.io.wy,
                0xff4b => self.io.wx,
                _ => {
                    log::warn!("unimplemented IO reg read at {a:x?}");
                    0xff
                }
            },
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
            0x8000..=0x9fff => {
                self.vram[a - 0x8000] = val;
            }
            0xc000..=0xdfff => {
                self.wram[a - 0xc000] = val;
            }
            0xe000..=0xfdff => {
                log::warn!("writing {val:x?} to echo ram at {a:x?}, is this ok?");
                self.write(addr - 0x2000, val)
            }
            0xfe00..=0xfe9f => {
                self.oam[a - 0xfe00] = val;
            }
            0xfea0..=0xfeff => (), // invalid write, do nothing
            0xff00..=0xff7f => match a {
                0xff00 => {
                    self.io.joyp = val & 0b0011_0000;
                }
                0xff01 => {
                    // TODO: serial, logging for now for blargg
                    // print!("{}", val as char);
                }
                0xff02 => {
                    self.io.sc = val;
                }
                0xff04 => {
                    // any write resets the divider/system clock to 0
                    self.sys = 0;
                }
                0xff05 => {
                    self.io.tima = val;
                }
                0xff06 => {
                    self.io.tma = val;
                }
                0xff07 => {
                    self.io.tac = val;
                }
                0xff0f => {
                    log::debug!("mmu: IF write: 0x{val:x?}");
                    self.io.interrupt = val;
                }
                0xff40 => {
                    log::debug!("mmu: LCDC write: 0x{val:x?}");
                    self.io.lcdc = val;
                }
                0xff41 => {
                    log::debug!("mmu: STAT write: 0x{val:x?}");
                    self.io.stat = val & 0b0111_1000;
                }
                0xff42 => {
                    log::debug!("mmu: SCY write: 0x{val:x?}");
                    self.io.scy = val;
                }
                0xff43 => {
                    log::debug!("mmu: SCX write: 0x{val:x?}");
                    self.io.scx = val;
                }
                0xff44 => {
                    // read-only
                }
                0xff45 => {
                    log::debug!("mmu: LYC write: 0x{val:x?}");
                    self.io.lyc = val;
                }
                0xff46 => {
                    self.io.dma = val;
                    self.dma_requsted = true;
                }
                0xff47 => {
                    log::debug!("mmu: BGP write: 0x{val:x?}");
                    self.io.bgp = val;
                }
                0xff48 => {
                    self.io.obp0 = val;
                }
                0xff49 => {
                    self.io.obp1 = val;
                }
                0xff4a => {
                    self.io.wy = val;
                }
                0xff4b => {
                    self.io.wx = val;
                }
                0xff50 => {
                    self.io.bank = val;
                }
                0xff10..=0xff14 | 0xff16..=0xff1e | 0xff20..=0xff26 | 0xff30..=0xff3f => {
                    self.apu.write(addr, val, self.sys);
                }

                _ => {
                    log::warn!("FIXME: mmu: write: unimplemented IO reg write at 0x{a:x?}");
                }
            },
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
