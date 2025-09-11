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
