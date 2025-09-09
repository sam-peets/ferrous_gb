use anyhow::anyhow;

const BOOT: &[u8] = include_bytes!("../../dmg_boot.bin");

#[derive(Debug)]
pub struct Mmu {
    booting: bool,
    interrupt: u8,
    rom: Vec<u8>,
    rom_banks: Vec<Vec<u8>>,
    cur_bank: usize,
    vram: Vec<u8>,
    exram: Vec<u8>,
    wram: Vec<u8>,
    oam: Vec<u8>,
    hram: Vec<u8>,
}

impl Mmu {
    pub fn new(rom: Vec<u8>) -> Self {
        let low_rom = rom[0..0x3fff].to_vec();
        let mut rom_banks = Vec::new();
        for i in 1..rom.len() / 0x3fff {
            rom_banks.push(rom[(i * 0x3fff)..((i + 1) * 0x3fff)].to_vec());
        }

        Self {
            booting: true,
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
                if self.booting {
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
            0xfea0..=0xfefe => Err(anyhow!("prohibited read at {a:x?}")),
            0xff00..=0xff7f => match a {
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
            } // echo ram
            0xfe00..=0xfe9f => {
                self.oam[a - 0xfe00] = val;
                Ok(())
            }
            0xfea0..=0xfefe => Err(anyhow!("mmu: write: prohibited write at {a:x?}")),
            0xff00..=0xff7f => match a {
                _ => Err(anyhow!("mmu: write: unimplemented IO reg read at {a:x?}")),
            },
            0xff80..=0xfffe => {
                self.hram[a - 0xff80] = val;
                Ok(())
            }
            0xffff => {
                self.interrupt = val;
                Ok(())
            }
            a => Err(anyhow!("mmu: write: read out of bounds at {a:x?}")),
        }
    }
}
