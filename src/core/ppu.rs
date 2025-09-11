use crate::core::mmu::Mmu;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

#[derive(Debug)]
enum Mode {
    HBlank = 0,
    OamScan = 1,
    Drawing = 2,
    VBlank = 3,
}

#[derive(Debug)]
pub struct Ppu {
    screen: Vec<u8>,
    dot: usize,
    mode: Mode,
    penalty: usize,
    lx: u8,
}
pub fn bit(x: u8, i: u8) -> u8 {
    (x & (1 << i)) >> i
}
impl Ppu {
    pub fn new() -> Self {
        Self {
            dot: 0,
            mode: Mode::OamScan,
            penalty: 0,
            screen: vec![0; 160 * 144],
            lx: 0,
        }
    }
    // pub fn frame(&mut self, mmu: &mut Mmu) -> anyhow::Result<Vec<u8>> {
    //     let mut f = vec![0; 160 * 144];
    //     let base = if (mmu.io.lcdc & 0b00010000) > 0 {
    //         0x8000
    //     } else {
    //         0x8800
    //     } as u16;
    //     for x in 0..16 {
    //         for y in 0..8 {
    //             for line in 0..8 {
    //                 let screen_base = (y * 8 + line) * 160 + x * 8;
    //                 let vram_base = (x * y * 16) + line * 2;
    //                 let b1 = mmu.read(base + vram_base)?;
    //                 let b2 = mmu.read(base + vram_base + 1)?;
    //                 for i in 0..=7 {
    //                     let bit1 = bit(b1, 7 - i);
    //                     let bit2 = bit(b2, 7 - i) << 1;
    //                     f[(screen_base + i as u16) as usize] = bit1 | bit2;
    //                 }
    //             }
    //         }
    //     }

    //     Ok(f)
    // }

    pub fn frame(&mut self, mmu: &mut Mmu) -> anyhow::Result<Vec<u8>> {
        Ok(self
            .screen
            .iter()
            .map(|x| match x {
                0b00 => mmu.io.bgp & 0b00000011,
                0b01 => (mmu.io.bgp & 0b00001100) >> 2,
                0b10 => (mmu.io.bgp & 0b00110000) >> 4,
                0b11 => (mmu.io.bgp & 0b11000000) >> 6,
                _ => unreachable!("ppu: frame: invalid color {x:02x?}"),
            })
            .collect())
    }
    pub fn clock(&mut self, mmu: &mut Mmu) -> anyhow::Result<()> {
        // println!(
        // "clock: dot: {} lx: {} ly: {} mode: {:?}",
        // self.dot, self.lx, mmu.io.ly, self.mode
        // );
        if self.penalty > 0 {
            self.penalty -= 1;
            self.dot += 1;
            return Ok(());
        }

        match self.mode {
            Mode::HBlank => {
                if self.dot == 455 {
                    mmu.io.ly += 1;
                    if mmu.io.ly == 144 {
                        self.mode = Mode::VBlank;
                        mmu.io.interrupt |= 0b00000001; // request vblank interrupt
                    } else {
                        self.mode = Mode::OamScan;
                    }
                }
            }
            Mode::OamScan => {
                // TODO
                if self.dot == 79 {
                    self.mode = Mode::Drawing;
                    self.penalty = 12 + (mmu.io.scx % 8) as usize;
                }
            }
            Mode::Drawing => {
                // BG
                let screen_idx = mmu.io.ly as usize * WIDTH + self.lx as usize;
                let vram_base = if (mmu.io.lcdc & 0b00010000) > 0 {
                    0x8000
                } else {
                    0x8800
                } as u16;
                let bg_tilemap_base = if (mmu.io.lcdc & 0b00001000) > 0 {
                    0x9c00
                } else {
                    0x9800
                } as u16;

                let (dx, _) = mmu.io.scx.overflowing_add(self.lx);
                let (dy, _) = mmu.io.scy.overflowing_add(mmu.io.ly);
                let tile_x = (dx / 8) as u16;
                let tile_y = (dy / 8) as u16;

                let tile = mmu.read(bg_tilemap_base + (tile_y * 32 + tile_x))?;
                let tile_base = vram_base + (tile as u16) * 16;
                let tile_row = tile_base + 2 * (dy % 8) as u16;
                let tile_col_d = dx % 8;

                let b1 = mmu.read(tile_row)?;
                let b1 = bit(b1, 7 - tile_col_d);
                let b2 = mmu.read(tile_row + 1)?;
                let b2 = bit(b2, 7 - tile_col_d);
                let color = (b2 << 1) | b1;
                self.screen[screen_idx] = color;

                self.lx += 1;
                if self.lx == 160 {
                    self.lx = 0;
                    self.mode = Mode::HBlank;
                }
            }
            Mode::VBlank => {
                if self.dot == 455 && mmu.io.ly == 153 {
                    self.mode = Mode::OamScan;
                    mmu.io.ly = 0;
                } else if self.dot == 455 {
                    mmu.io.ly += 1;
                }
            }
        }

        self.dot += 1;
        if self.dot == 456 {
            self.dot = 0;
        }
        Ok(())
    }
}
