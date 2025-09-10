use crate::core::mmu::Mmu;

#[derive(Debug)]
pub struct Ppu {
    lx: usize,
}
pub fn bit(x: u8, i: u8) -> u8 {
    (x & (1 << i)) >> i
}
impl Ppu {
    pub fn new() -> Self {
        Self { lx: 0 }
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
        self.lx += 1;
        if self.lx == 456 {
            self.lx = 0;
            mmu.io.ly += 1;
        }
        if mmu.io.ly == 154 {
            mmu.io.ly = 0;
            log::info!("ppu: frame done")
        }
        Ok(())
    }
}
