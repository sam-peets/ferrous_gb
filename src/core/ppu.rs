use crate::core::{Memory, Mode, mmu::Mmu, util::bit};

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
    vram: Vec<u8>,
    oam: Vec<u8>,

    lcdc: u8, // 0xff40
    stat: u8, // 0xff41
    scy: u8,  // 0xff42
    scx: u8,  // 0xff43
    ly: u8,   // 0xff44
    lyc: u8,  // 0xff45
    bgp: u8,  // 0xff47
    obp0: u8, // 0xff48
    obp1: u8, // 0xff49
    wy: u8,   // 0xff4a
    wx: u8,   // 0xff4b

    pub vblank_if: bool,
    pub stat_if: bool,
    pub mode: Mode,
}

impl Memory for Ppu {
    fn read(&self, addr: u16) -> u8 {
        let a = addr as usize;
        match addr {
            0xff40 => self.lcdc,
            0xff41 => {
                let mut v = self.stat;
                if self.ly == self.lyc {
                    v |= 0b0000_0100;
                }
                v |= self.mode as u8;
                v
            }
            0xff42 => self.scy,
            0xff43 => self.scx,
            0xff44 => self.ly,
            0xff45 => self.lyc,
            0xff47 => self.bgp,
            0xff48 => self.obp0,
            0xff49 => self.obp1,
            0xff4a => self.wy,
            0xff4b => self.wx,
            0x8000..=0x9fff => self.vram[a - 0x8000],
            0xfe00..=0xfe9f => self.oam[a - 0xfe00],
            _ => unreachable!("Ppu: Memory: invalid read 0x{addr:04x?}"),
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        let a = addr as usize;
        match addr {
            0x8000..=0x9fff => {
                self.vram[a - 0x8000] = val;
            }
            0xfe00..=0xfe9f => {
                self.oam[a - 0xfe00] = val;
            }
            0xff40 => {
                log::debug!("mmu: LCDC write: 0x{val:x?}");
                self.lcdc = val;
            }
            0xff41 => {
                log::debug!("mmu: STAT write: 0x{val:x?}");
                self.stat = val & 0b0111_1000;
            }
            0xff42 => {
                log::debug!("mmu: SCY write: 0x{val:x?}");
                self.scy = val;
            }
            0xff43 => {
                log::debug!("mmu: SCX write: 0x{val:x?}");
                self.scx = val;
            }
            0xff44 => {
                // read-only
            }
            0xff45 => {
                log::debug!("mmu: LYC write: 0x{val:x?}");
                self.lyc = val;
            }
            0xff47 => {
                log::debug!("mmu: BGP write: 0x{val:x?}");
                self.bgp = val;
            }
            0xff48 => {
                self.obp0 = val;
            }
            0xff49 => {
                self.obp1 = val;
            }
            0xff4a => {
                self.wy = val;
            }
            0xff4b => {
                self.wx = val;
            }
            _ => unreachable!("Ppu: Memory: invalid write 0x{addr:04x?}"),
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
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
            vram: vec![0; 0x2000],
            oam: vec![0; 0x100],
            mode: Mode::OamScan,
            lcdc: 0,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wy: 0,
            wx: 0,
            vblank_if: false,
            stat_if: false,
        }
    }
}

impl Ppu {
    pub fn new() -> Self {
        Ppu::default()
    }

    pub fn frame(&mut self) -> Vec<u8> {
        if (self.lcdc & 0b1000_0000) != 0 {
            // lcd is enabled
            self.screen.clone()
        } else {
            vec![0; 160 * 144]
        }
    }

    fn draw_bg(&mut self) -> u8 {
        let bg_tilemap_base: u16 = if (self.lcdc & 0b0000_1000) > 0 {
            0x9c00
        } else {
            0x9800
        };

        let (dx, _) = self.scx.overflowing_add(self.lx);
        let (dy, _) = self.scy.overflowing_add(self.ly);
        let tile_x = u16::from(dx / 8);
        let tile_y = u16::from(dy / 8);

        let tile = self.read(bg_tilemap_base + (tile_y * 32 + tile_x));

        let tile_base = {
            if (self.lcdc & 0b0001_0000) > 0 {
                0x8000 + u16::from(tile) * 16
            } else if tile <= 127 {
                0x9000 + u16::from(tile) * 16
            } else {
                0x8800 + u16::from(tile - 128) * 16
            }
        };
        let tile_row = tile_base + 2 * u16::from(dy % 8);
        let tile_col_d = dx % 8;

        let b1 = self.read(tile_row);
        let b1 = bit(b1, 7 - tile_col_d);
        let b2 = self.read(tile_row + 1);
        let b2 = bit(b2, 7 - tile_col_d);
        (b2 << 1) | b1
    }

    fn draw_window(&mut self) -> u8 {
        let window_tilemap_base: u16 = if (self.lcdc & 0b0100_0000) > 0 {
            0x9c00
        } else {
            0x9800
        };

        let (dx, _) = self.lx.overflowing_sub(self.wx.overflowing_sub(7).0);
        let dy = self.window_y;
        let tile_x = u16::from(dx / 8);
        let tile_y = u16::from(dy / 8);

        let tile = self.read(window_tilemap_base + (tile_y * 32 + tile_x));

        let tile_base = {
            if (self.lcdc & 0b0001_0000) > 0 {
                0x8000 + u16::from(tile) * 16
            } else if tile <= 127 {
                0x9000 + u16::from(tile) * 16
            } else {
                0x8800 + u16::from(tile - 128) * 16
            }
        };
        let tile_row = tile_base + 2 * u16::from(dy % 8);
        let tile_col_d = dx % 8;

        let b1 = self.read(tile_row);
        let b1 = bit(b1, 7 - tile_col_d);
        let b2 = self.read(tile_row + 1);
        let b2 = bit(b2, 7 - tile_col_d);
        let color = (b2 << 1) | b1;

        color
    }

    fn draw_objects(&mut self) -> Option<(u8, bool, u8)> {
        let vram_base = 0x8000_u16;
        let lx = self.lx + 8;
        let ly = self.ly + 16;
        let height = if (self.lcdc & 0b0000_0100) != 0 {
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
            let b1 = self.read(tile_row);
            let b1 = bit(b1, 7 - dx);
            let b2 = self.read(tile_row + 1);
            let b2 = bit(b2, 7 - dx);
            let color = (b2 << 1) | b1;

            if color == 0b00 {
                continue;
            }
            let palette = if (obj.attributes & 0b0001_0000) > 0 {
                self.obp1
            } else {
                self.obp0
            };

            return Some((color, (obj.attributes & 0b1000_0000) > 0, palette));
        }
        None
    }

    fn clock_hblank(&mut self) {
        if self.dot == 455 {
            if self.ly == 143 {
                self.mode = Mode::VBlank;
                self.vblank_if = true;
                self.window_y = 0;
                if (self.stat & 0b0001_0000) > 0 {
                    // raise STAT interrupt for mode 1
                    self.stat_if = true;
                }
            } else {
                self.mode = Mode::OamScan;
                self.wx_condition = false;
                if (self.stat & 0b0010_0000) > 0 {
                    // raise STAT interrupt for mode 2
                    self.stat_if = true;
                }
            }
        }
    }

    fn clock_oamscan(&mut self) {
        // TODO: cheating by reading all at once, what's the precise timing here?
        if self.dot == 0 {
            self.objects.clear();
            for i in 0..40 {
                if self.objects.len() == 10 {
                    break;
                }
                let y = self.read(OAM_BASE + i * 4);
                let ly = self.ly + 16; // object y pos is offset by 16

                let height = if (self.lcdc & 0b0000_0100) != 0 {
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
                    x: self.read(OAM_BASE + i * 4 + 1),
                    tile: self.read(OAM_BASE + i * 4 + 2),
                    attributes: self.read(OAM_BASE + i * 4 + 3),
                };
                self.objects.push(obj);
            }
            self.objects.sort_by_key(|obj| obj.x);
        }

        if self.dot == 79 {
            self.mode = Mode::Drawing;
            // self.penalty = 12 + (mmu.io.scx % 8) as usize;
            if self.ly == self.wy {
                self.wy_condition = true;
            }
        }
    }

    fn clock_drawing(&mut self) {
        if self.lx + 7 >= self.wx {
            self.wx_condition = true;
        }
        let mut bg_dot = None;
        let mut window_dot = None;
        let mut object_dot = None;
        let screen_idx = self.ly as usize * WIDTH + self.lx as usize;

        if (self.lcdc & 0b0000_0001) > 0 {
            // BG enabled
            bg_dot = Some(self.draw_bg());

            // window is only drawn if BG is enabled
            if (self.lcdc & 0b0010_0000) > 0 && self.wy_condition && self.wx_condition {
                self.window_y_update = true;
                window_dot = Some(self.draw_window());
            }
        } else {
            // BG disabled, set to white
            self.screen[screen_idx] = self.bgp & 0b0000_0011;
        }

        if (self.lcdc & 0b0000_0010) > 0 {
            object_dot = self.draw_objects();
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
                0b00 => self.bgp & 0b0000_0011,
                0b01 => (self.bgp & 0b0000_1100) >> 2,
                0b10 => (self.bgp & 0b0011_0000) >> 4,
                0b11 => (self.bgp & 0b1100_0000) >> 6,
                _ => unreachable!("ppu: draw_window: invalid color {bg_idx:02x?}"),
            }
        };

        self.screen[screen_idx] = color;

        self.lx += 1;
        if self.lx == 160 {
            self.lx = 0;
            self.mode = Mode::HBlank;
            if self.window_y_update {
                self.window_y += 1;
                self.window_y_update = false;
            }
            if (self.stat & 0b0000_1000) > 0 {
                // raise STAT interrupt for mode 0
                self.stat_if = true;
            }
        }
    }

    fn clock_vblank(&mut self) {
        if self.dot == 455 && self.ly == 153 {
            // end of frame
            self.mode = Mode::OamScan;
            self.wy_condition = false;
            self.window_y = 0;
            if (self.stat & 0b0010_0000) > 0 {
                // raise STAT interrupt for mode 2
                self.stat_if = true;
            }
        }
    }
    pub fn clock(&mut self) {
        // println!(
        // "clock: dot: {} lx: {} ly: {} mode: {:?}",
        // self.dot, self.lx, mmu.io.ly, self.mode
        // );
        if self.penalty > 0 {
            self.penalty -= 1;
            self.dot += 1;
            return;
        }

        match self.mode {
            Mode::HBlank => self.clock_hblank(),
            Mode::OamScan => self.clock_oamscan(),
            Mode::Drawing => self.clock_drawing(),
            Mode::VBlank => self.clock_vblank(),
        }

        self.dot += 1;
        if self.dot == 456 {
            // overflow
            self.dot = 0;
            self.ly += 1;
            if self.ly == 154 {
                // overflow
                self.ly = 0;
            }

            // log::debug!("{} {} {}", mmu.io.stat, mmu.io.ly, mmu.io.lyc);
            if (self.stat & 0b0100_0000) > 0 && self.ly == self.lyc {
                log::debug!("lyc interrupt");
                self.stat_if = true;
            }
        }
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
                        let b1 = mmu.read(bank_base + vram_base);
                        let b2 = mmu.read(bank_base + vram_base + 1);
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
