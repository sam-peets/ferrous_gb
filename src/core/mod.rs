pub mod cpu;
pub mod mmu;
mod ppu;
pub mod register;

#[derive(Debug, Default)]
pub struct Buttons {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub start: bool,
    pub select: bool,
    pub a: bool,
    pub b: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    HBlank = 0,
    OamScan = 1,
    Drawing = 2,
    VBlank = 3,
}
