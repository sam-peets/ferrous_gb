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
    lx: usize,
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
    pub fn frame(&mut self, mmu: &mut Mmu) -> anyhow::Result<Vec<u8>> {
        let mut f = vec![0; 160 * 144];
        let base: u16 = 0x8000;
        for x in 0..16 {
            for y in 0..8 {
                for line in 0..8 {
                    let screen_base = (y * 8 + line) * 160 + x * 8;
                    let vram_base = (x * y * 16) + line * 2;
                    let b1 = mmu.read(base + vram_base)?;
                    let b2 = mmu.read(base + vram_base + 1)?;
                    for i in 0..=7 {
                        let bit1 = bit(b1, 7 - i);
                        let bit2 = bit(b2, 7 - i) << 1;
                        f[(screen_base + i as u16) as usize] = bit1 | bit2;
                    }
                }
            }
        }

        Ok(f)
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
                let idx = mmu.io.ly as usize * WIDTH + self.lx;
                // draw a pixel...
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
