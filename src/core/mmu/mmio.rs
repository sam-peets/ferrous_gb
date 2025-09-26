use crate::core::{Buttons, Memory, apu::Apu, ppu::Ppu, util::extract};

pub const JOYP: u16 = 0xff00;
pub const SB: u16 = 0xff01;
pub const SC: u16 = 0xff02;
pub const DIV: u16 = 0xff04;
pub const TIMA: u16 = 0xff05;
pub const TMA: u16 = 0xff06;
pub const TAC: u16 = 0xff07;
pub const IF: u16 = 0xff0f;
pub const NR10: u16 = 0xff10;
pub const NR11: u16 = 0xff11;
pub const NR12: u16 = 0xff12;
pub const NR13: u16 = 0xff13;
pub const NR14: u16 = 0xff14;
pub const NR21: u16 = 0xff16;
pub const NR22: u16 = 0xff17;
pub const NR23: u16 = 0xff18;
pub const NR24: u16 = 0xff19;
pub const NR30: u16 = 0xff1a;
pub const NR31: u16 = 0xff1b;
pub const NR32: u16 = 0xff1c;
pub const NR33: u16 = 0xff1d;
pub const NR34: u16 = 0xff1e;
pub const NR41: u16 = 0xff20;
pub const NR42: u16 = 0xff21;
pub const NR43: u16 = 0xff22;
pub const NR44: u16 = 0xff23;
pub const NR50: u16 = 0xff24;
pub const NR51: u16 = 0xff25;
pub const NR52: u16 = 0xff26;
pub const LCDC: u16 = 0xff40;
pub const STAT: u16 = 0xff41;
pub const SCY: u16 = 0xff42;
pub const SCX: u16 = 0xff43;
pub const LY: u16 = 0xff44;
pub const LYC: u16 = 0xff45;
pub const DMA: u16 = 0xff46;
pub const BGP: u16 = 0xff47;
pub const OBP0: u16 = 0xff48;
pub const OBP1: u16 = 0xff49;
pub const WY: u16 = 0xff4a;
pub const WX: u16 = 0xff4b;
pub const BANK: u16 = 0xff50;
pub const IE: u16 = 0xffff;

#[derive(Default, Debug)]
pub struct Mmio {
    joyp: u8, // 0xff00
    sc: u8,   // 0xff02
    tima: u8, // 0xff05
    tma: u8,  // 0xff06
    tac: u8,  // 0xff07
    dma: u8,  // 0xff46
    bank: u8, // 0xff50 - bootrom mapping control

    pub buttons: Buttons,
    pub apu: Apu,
    pub ppu: Ppu,
    pub sys: u16,
    pub dma_requsted: bool,
    timer_if: bool,
    serial_if: bool,
    joypad_if: bool,
}

impl Memory for Mmio {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9fff | 0xfe00..=0xfe9f => self.ppu.read(addr),
            JOYP => {
                let joyp = self.joyp & 0b0011_0000;
                let buttons = self.buttons.as_joyp(joyp >> 4);
                joyp | buttons
            }
            SC => self.sc,
            0xff04 => ((self.sys & 0xff00) >> 8) as u8,
            0xff05 => self.tima,
            0xff06 => self.tma,
            0xff07 => self.tac,
            0xff0f => {
                let mut interrupt = 0;
                if self.ppu.vblank_if {
                    interrupt |= 1;
                }
                if self.ppu.stat_if {
                    interrupt |= 1 << 1;
                }
                if self.timer_if {
                    interrupt |= 1 << 2;
                }
                if self.serial_if {
                    interrupt |= 1 << 3;
                }
                if self.joypad_if {
                    interrupt |= 1 << 4;
                }
                interrupt
            }
            0xff10..=0xff14 | 0xff16..=0xff1e | 0xff20..=0xff26 | 0xff30..=0xff3f => {
                self.apu.read(addr, self.sys)
            }
            0xff40..=0xff45 | 0xff47..=0xff4b => self.ppu.read(addr), // PPU registers
            0xff46 => self.dma,
            BANK => self.bank,
            _ => {
                log::warn!("unimplemented IO reg read at {addr:x?}");
                0xff
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0xff40..=0xff45 | 0xff47..=0xff4b => self.ppu.write(addr, val), // PPU registers
            0x8000..=0x9fff | 0xfe00..=0xfe9f => self.ppu.write(addr, val), // PPU memory
            0xff00 => {
                self.joyp = val & 0b0011_0000;
            }
            0xff01 => {
                // TODO: serial, logging for now for blargg
                // print!("{}", val as char);
            }
            0xff02 => {
                self.sc = val;
            }
            0xff04 => {
                // any write resets the divider/system clock to 0
                self.sys = 0;
            }
            0xff05 => {
                self.tima = val;
            }
            0xff06 => {
                self.tma = val;
            }
            0xff07 => {
                self.tac = val;
            }
            0xff0f => {
                log::debug!("mmu: IF write: 0x{val:x?}");
                self.ppu.vblank_if = (val & 0b0000_0001) > 0;
                self.ppu.stat_if = (val & 0b0000_0010) > 0;
                self.timer_if = (val & 0b0000_0100) > 0;
                self.serial_if = (val & 0b0000_1000) > 0;
                self.joypad_if = (val & 0b0001_0000) > 0;
            }
            0xff46 => {
                self.dma = val;
                self.dma_requsted = true;
            }
            0xff50 => {
                self.bank = val;
            }
            0xff10..=0xff14 | 0xff16..=0xff1e | 0xff20..=0xff26 | 0xff30..=0xff3f => {
                self.apu.write(addr, val, self.sys);
            }

            _ => {
                log::warn!("FIXME: mmu: write: unimplemented IO reg write at 0x{addr:x?}");
            }
        }
    }
}

impl Mmio {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            apu: Apu::new(sample_rate),
            ..Default::default()
        }
    }
}
