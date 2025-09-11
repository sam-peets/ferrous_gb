use anyhow::anyhow;

const BOOT: &[u8] = include_bytes!("../../dmg_boot.bin");

#[derive(Default, Debug)]
pub struct IoRegisters {
    pub joyp: u8,      // ff00
    pub sc: u8,        // ff02
    pub interrupt: u8, // 0xff0f
    pub lcdc: u8,      // 0xff40
    pub stat: u8,      // 0xff41
    pub scy: u8,       // 0xff42
    pub scx: u8,       // 0xff43
    pub ly: u8,        // 0xff44
    pub bgp: u8,       // 0xff47
    pub obp0: u8,      // 0xff48
    pub obp1: u8,      // 0xff49
    pub bank: u8,      // 0xff50 - bootrom mapping control
}

#[derive(Debug)]
pub struct Mmu {
    interrupt: u8,
    rom: Vec<u8>,
    rom_banks: Vec<Vec<u8>>,
    cur_bank: usize,
    vram: Vec<u8>,
    exram: Vec<u8>,
    wram: Vec<u8>,
    oam: Vec<u8>,
    hram: Vec<u8>,
    pub io: IoRegisters,
}

impl Mmu {
    pub fn new(rom: Vec<u8>) -> Self {
        let low_rom = rom[0..0x3fff].to_vec();
        let mut rom_banks = Vec::new();
        for i in 1..rom.len() / 0x3fff {
            rom_banks.push(rom[(i * 0x4000)..((i + 1) * 0x4000)].to_vec());
        }

        let io = IoRegisters {
            ..Default::default()
        };

        Self {
            io,
            interrupt: 0,
            rom: low_rom,
            rom_banks,
            cur_bank: 0,
            vram: vec![0; 0x2000],
            exram: vec![0; 0x2000],
            wram: vec![0; 0x2000],
            oam: vec![0; 0x100],
            hram: vec![0; 0x7f],
        }
    }

    pub fn read(&self, addr: u16) -> anyhow::Result<u8> {
        log::trace!("read: reading {addr:x?}");
        let a = addr as usize;
        match a {
            0x0..=0xff => {
                if self.io.bank == 0 {
                    Ok(BOOT[a])
                } else {
                    Ok(self.rom[a])
                }
            }
            0x100..=0x3fff => Ok(self.rom[a]),
            0x4000..=0x7fff => Ok(self.rom_banks[self.cur_bank][a - 0x4000]),
            0x8000..=0x9fff => Ok(self.vram[a - 0x8000]),
            0xa000..=0xbfff => Ok(self.exram[a - 0xa000]),
            0xc000..=0xdfff => Ok(self.wram[a - 0xc000]),
            0xe000..=0xfdff => self.read(addr - 0x2000), // echo ram
            0xfe00..=0xfe9f => Ok(self.oam[a - 0xfe00]),
            // 0xfea0..=0xfeff => Err(anyhow!("prohibited read at {a:x?}")),
            0xfea0..=0xfeff => Ok(0xff), // invalid read, just return 0xff
            0xff00..=0xff7f => match a {
                0xff00 => Ok(self.io.joyp | 0b00001111),
                0xff02 => Ok(self.io.sc),
                0xff0f => Ok(self.io.interrupt),
                0xff40 => Ok(self.io.lcdc),
                0xff41 => Ok(self.io.stat),
                0xff42 => Ok(self.io.scy),
                0xff43 => Ok(self.io.scx),
                0xff44 => Ok(self.io.ly),
                // 0xff44 => Ok(0x90),
                0xff48 => Ok(self.io.obp0),
                0xff49 => Ok(self.io.obp1),
                _ => Err(anyhow!("unimplemented IO reg read at {a:x?}")),
            },
            0xff80..=0xfffe => Ok(self.hram[a - 0xff80]),
            0xffff => Ok(self.interrupt),
            a => Err(anyhow!("read out of bounds at {a:x?}")),
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) -> anyhow::Result<()> {
        log::trace!("write: writing {val:x?} to {addr:x?}");
        let a = addr as usize;
        match a {
            0x0..=0x3fff => {
                self.rom[a] = val;
                Ok(())
            }
            0x4000..=0x7fff => {
                self.rom_banks[self.cur_bank][a - 0x4000] = val;
                Ok(())
            }
            0x8000..=0x9fff => {
                self.vram[a - 0x8000] = val;
                Ok(())
            }
            0xa000..=0xbfff => {
                self.exram[a - 0xa000] = val;
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
                    self.io.joyp = val;
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
                0xff0f => {
                    log::info!("mmu: IF write: 0x{val:x?}");
                    self.io.interrupt = val;
                    Ok(())
                }
                0xff40 => {
                    log::info!("mmu: LCDC write: 0x{val:x?}");
                    self.io.lcdc = val;
                    Ok(())
                }
                0xff41 => {
                    log::info!("mmu: STAT write: 0x{val:x?}");
                    self.io.stat = val;
                    Ok(())
                }
                0xff42 => {
                    log::info!("mmu: SCY write: 0x{val:x?}");
                    self.io.scy = val;
                    Ok(())
                }
                0xff43 => {
                    log::info!("mmu: SCX write: 0x{val:x?}");
                    self.io.scx = val;
                    Ok(())
                }
                0xff44 => {
                    // read-only
                    // self.io.ly = val;
                    Ok(())
                }
                0xff47 => {
                    log::info!("mmu: BGP write: 0x{val:x?}");
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
                0xff50 => {
                    self.io.bank = val;
                    Ok(())
                }
                0xff10..=0xff3f => {
                    log::trace!("FIXME: mmu: sound register write: {a:x?} {val:x?}");
                    Ok(())
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
                self.interrupt = val;
                Ok(())
            }
            a => Err(anyhow!("mmu: write: write out of bounds at {a:x?}")),
        }
    }
}
