use crate::core::{Mode, mmu::Mmu};

const WIDTH: usize = 160;
#[allow(dead_code)] // We never use this... but doesn't it feel weird not to have it as a constant?
const HEIGHT: usize = 144;
const OAM_BASE: u16 = 0xfe00;

#[derive(Debug)]
struct Object {
    y: u8,
    x: u8,
    tile: u8,
    attributes: u8,
}

#[derive(Debug)]
pub struct Ppu {
    screen: Vec<u8>,
    dot: usize,
    penalty: usize,
    lx: u8,
    objects: Vec<Object>, // list of objects found during OAM scan
    window_y: u8,
    window_y_update: bool,
    wy_condition: bool,
    wx_condition: bool,
}
pub fn bit(x: u8, i: u8) -> u8 {
    (x & (1 << i)) >> i
}
impl Ppu {
    pub fn new() -> Self {
        Self {
            dot: 0,
            penalty: 0,
            screen: vec![0; 160 * 144],
            lx: 0,
            objects: Vec::new(),
            window_y: 0,
            window_y_update: false,
            wy_condition: false,
            wx_condition: false,
        }
    }

    pub fn frame(&mut self, mmu: &mut Mmu) -> Vec<u8> {
        if (mmu.io.lcdc & 0b1000_0000) != 0 {
            // lcd is enabled
            self.screen.clone()
        } else {
            vec![0; 160 * 144]
        }
    }

    fn draw_bg(&mut self, mmu: &mut Mmu) -> anyhow::Result<u8> {
        let bg_tilemap_base: u16 = if (mmu.io.lcdc & 0b0000_1000) > 0 {
            0x9c00
        } else {
            0x9800
        };

        let (dx, _) = mmu.io.scx.overflowing_add(self.lx);
        let (dy, _) = mmu.io.scy.overflowing_add(mmu.io.ly);
        let tile_x = u16::from(dx / 8);
        let tile_y = u16::from(dy / 8);

        let tile = mmu.read(bg_tilemap_base + (tile_y * 32 + tile_x))?;

        let tile_base = {
            if (mmu.io.lcdc & 0b0001_0000) > 0 {
                0x8000 + u16::from(tile) * 16
            } else if tile <= 127 {
                0x9000 + u16::from(tile) * 16
            } else {
                0x8800 + u16::from(tile - 128) * 16
            }
        };
        let tile_row = tile_base + 2 * u16::from(dy % 8);
        let tile_col_d = dx % 8;

        let b1 = mmu.read(tile_row)?;
        let b1 = bit(b1, 7 - tile_col_d);
        let b2 = mmu.read(tile_row + 1)?;
        let b2 = bit(b2, 7 - tile_col_d);
        let color = (b2 << 1) | b1;

        Ok(color)
    }

    fn draw_window(&mut self, mmu: &mut Mmu) -> anyhow::Result<u8> {
        let window_tilemap_base: u16 = if (mmu.io.lcdc & 0b0100_0000) > 0 {
            0x9c00
        } else {
            0x9800
        };

        let (dx, _) = self.lx.overflowing_sub(mmu.io.wx.overflowing_sub(7).0);
        let dy = self.window_y;
        let tile_x = u16::from(dx / 8);
        let tile_y = u16::from(dy / 8);

        let tile = mmu.read(window_tilemap_base + (tile_y * 32 + tile_x))?;

        let tile_base = {
            if (mmu.io.lcdc & 0b0001_0000) > 0 {
                0x8000 + u16::from(tile) * 16
            } else if tile <= 127 {
                0x9000 + u16::from(tile) * 16
            } else {
                0x8800 + u16::from(tile - 128) * 16
            }
        };
        let tile_row = tile_base + 2 * u16::from(dy % 8);
        let tile_col_d = dx % 8;

        let b1 = mmu.read(tile_row)?;
        let b1 = bit(b1, 7 - tile_col_d);
        let b2 = mmu.read(tile_row + 1)?;
        let b2 = bit(b2, 7 - tile_col_d);
        let color = (b2 << 1) | b1;

        Ok(color)
    }

    fn draw_objects(&mut self, mmu: &mut Mmu) -> anyhow::Result<Option<(u8, bool, u8)>> {
        let vram_base = 0x8000_u16;
        let lx = self.lx + 8;
        let ly = mmu.io.ly + 16;
        let height = if (mmu.io.lcdc & 0b0000_0100) != 0 {
            // TODO: confirm when sprite hight is checked, is it during OAM scan? drawing?
            16_u8
        } else {
            8_u8
        };

        for obj in &self.objects {
            if !((obj.x <= lx) && ((obj.x + 8) > lx)) {
                // object isn't on the column
                continue;
            }
            // object is on the current dot
            let dx = if (obj.attributes & 0b0010_0000) > 0 {
                let (x, _) = 7_u8.overflowing_sub(lx - obj.x);
                x
            } else {
                lx - obj.x
            };
            let dy = if (obj.attributes & 0b0100_0000) > 0 {
                let (y, _) = (height - 1).overflowing_sub(ly - obj.y);
                y
            } else {
                ly - obj.y
            };
            // ignore the last bit for tall objects
            let tile = if height == 16 {
                obj.tile & 0b1111_1110
            } else {
                obj.tile
            };

            // TODO: this is mostly the same logic as draw_bg, extract this to a function
            let tile_base = vram_base + u16::from(tile) * 16;
            let tile_row = tile_base + 2 * u16::from(dy);
            let b1 = mmu.read(tile_row)?;
            let b1 = bit(b1, 7 - dx);
            let b2 = mmu.read(tile_row + 1)?;
            let b2 = bit(b2, 7 - dx);
            let color = (b2 << 1) | b1;

            if color == 0b00 {
                continue;
            }
            let palette = if (obj.attributes & 0b0001_0000) > 0 {
                mmu.io.obp1
            } else {
                mmu.io.obp0
            };

            return Ok(Some((color, (obj.attributes & 0b1000_0000) > 0, palette)));
        }
        Ok(None)
    }

    fn clock_hblank(&mut self, mmu: &mut Mmu) {
        if self.dot == 455 {
            if mmu.io.ly == 143 {
                mmu.ppu_mode = Mode::VBlank;
                mmu.io.interrupt |= 0b0000_0001; // request vblank interrupt
                self.window_y = 0;
                if (mmu.io.stat & 0b0001_0000) > 0 {
                    // raise STAT interrupt for mode 1
                    mmu.io.interrupt |= 0b0000_0010;
                }
            } else {
                mmu.ppu_mode = Mode::OamScan;
                self.wx_condition = false;
                if (mmu.io.stat & 0b0010_0000) > 0 {
                    // raise STAT interrupt for mode 2
                    mmu.io.interrupt |= 0b0000_0010;
                }
            }
        }
    }

    fn clock_oamscan(&mut self, mmu: &mut Mmu) -> anyhow::Result<()> {
        // TODO: cheating by reading all at once, what's the precise timing here?
        if self.dot == 0 {
            self.objects.clear();
            for i in 0..40 {
                if self.objects.len() == 10 {
                    break;
                }
                let y = mmu.read(OAM_BASE + i * 4)?;
                let ly = mmu.io.ly + 16; // object y pos is offset by 16

                let height = if (mmu.io.lcdc & 0b0000_0100) != 0 {
                    // 8x16 sprites
                    16
                } else {
                    // 8x8 sprites
                    8
                };
                if !((ly >= y) && ((y + height) > ly)) {
                    // object isn't on the line
                    continue;
                }

                // object is on the line, we should draw it
                let obj = Object {
                    y,
                    x: mmu.read(OAM_BASE + i * 4 + 1)?,
                    tile: mmu.read(OAM_BASE + i * 4 + 2)?,
                    attributes: mmu.read(OAM_BASE + i * 4 + 3)?,
                };
                self.objects.push(obj);
            }
            self.objects.sort_by_key(|obj| obj.x);
        }

        if self.dot == 79 {
            mmu.ppu_mode = Mode::Drawing;
            // self.penalty = 12 + (mmu.io.scx % 8) as usize;
            if mmu.io.ly == mmu.io.wy {
                self.wy_condition = true;
            }
        }
        Ok(())
    }

    fn clock_drawing(&mut self, mmu: &mut Mmu) -> anyhow::Result<()> {
        if self.lx + 7 >= mmu.io.wx {
            self.wx_condition = true;
        }
        let mut bg_dot = None;
        let mut window_dot = None;
        let mut object_dot = None;
        let screen_idx = mmu.io.ly as usize * WIDTH + self.lx as usize;

        if (mmu.io.lcdc & 0b0000_0001) > 0 {
            // BG enabled
            bg_dot = Some(self.draw_bg(mmu)?);

            // window is only drawn if BG is enabled
            if (mmu.io.lcdc & 0b0010_0000) > 0 && self.wy_condition && self.wx_condition {
                self.window_y_update = true;
                window_dot = Some(self.draw_window(mmu)?);
            }
        } else {
            // BG disabled, set to white
            self.screen[screen_idx] = mmu.io.bgp & 0b0000_0011;
        }

        if (mmu.io.lcdc & 0b0000_0010) > 0 {
            object_dot = self.draw_objects(mmu)?;
        }

        let bg_idx = window_dot.unwrap_or(bg_dot.unwrap_or(0));

        let obj_color = if let Some((dot, priority, palette)) = object_dot {
            if priority && bg_idx != 0 {
                None
            } else {
                Some(match dot {
                    0b00 => palette & 0b0000_0011,
                    0b01 => (palette & 0b0000_1100) >> 2,
                    0b10 => (palette & 0b0011_0000) >> 4,
                    0b11 => (palette & 0b1100_0000) >> 6,
                    _ => unreachable!("ppu: draw_objects: invalid color {dot:02x?}"),
                })
            }
        } else {
            None
        };

        let color = if let Some(color) = obj_color {
            color
        } else {
            match bg_idx {
                0b00 => mmu.io.bgp & 0b0000_0011,
                0b01 => (mmu.io.bgp & 0b0000_1100) >> 2,
                0b10 => (mmu.io.bgp & 0b0011_0000) >> 4,
                0b11 => (mmu.io.bgp & 0b1100_0000) >> 6,
                _ => unreachable!("ppu: draw_window: invalid color {bg_idx:02x?}"),
            }
        };

        self.screen[screen_idx] = color;

        self.lx += 1;
        if self.lx == 160 {
            self.lx = 0;
            mmu.ppu_mode = Mode::HBlank;
            if self.window_y_update {
                self.window_y += 1;
                self.window_y_update = false;
            }
            if (mmu.io.stat & 0b0000_1000) > 0 {
                // raise STAT interrupt for mode 0
                mmu.io.interrupt |= 0b0000_0010;
            }
        }
        Ok(())
    }

    fn clock_vblank(&mut self, mmu: &mut Mmu) {
        if self.dot == 455 && mmu.io.ly == 153 {
            // end of frame
            mmu.ppu_mode = Mode::OamScan;
            self.wy_condition = false;
            self.window_y = 0;
            if (mmu.io.stat & 0b0010_0000) > 0 {
                // raise STAT interrupt for mode 2
                mmu.io.interrupt |= 0b0000_0010;
            }
        }
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

        match mmu.ppu_mode {
            Mode::HBlank => self.clock_hblank(mmu),
            Mode::OamScan => self.clock_oamscan(mmu)?,
            Mode::Drawing => self.clock_drawing(mmu)?,
            Mode::VBlank => self.clock_vblank(mmu),
        }

        self.dot += 1;
        if self.dot == 456 {
            // overflow
            self.dot = 0;
            mmu.io.ly += 1;
            if mmu.io.ly == 154 {
                // overflow
                mmu.io.ly = 0;
            }

            // log::debug!("{} {} {}", mmu.io.stat, mmu.io.ly, mmu.io.lyc);
            if (mmu.io.stat & 0b0100_0000) > 0 && mmu.io.ly == mmu.io.lyc {
                log::debug!("lyc interrupt");
                mmu.io.interrupt |= 0b0000_0010;
            }
        }

        Ok(())
    }

    pub fn dump_vram(mmu: &mut Mmu) -> anyhow::Result<Vec<u8>> {
        let mut out = vec![0; 32768];
        let base = 0x8000;

        for bank in 0..4 {
            let bank_base = base + bank * 16 * 8 * 16;

            for tile_x in 0..16 {
                for tile_y in 0..8 {
                    for y in 0..8 {
                        let screen_base = bank * 16 * 8 * 64 + (tile_y * 8 + y) * 128 + tile_x * 8;
                        let vram_base = (tile_y * 16 * 16) + (tile_x * 16) + y * 2;
                        let b1 = mmu.read(bank_base + vram_base)?;
                        let b2 = mmu.read(bank_base + vram_base + 1)?;
                        for x in 0..=7 {
                            let bit1 = bit(b1, 7 - x);
                            let bit2 = bit(b2, 7 - x) << 1;
                            out[(screen_base + u16::from(x)) as usize] = bit1 | bit2;
                        }
                    }
                }
            }
        }

        Ok(out)
    }
}
