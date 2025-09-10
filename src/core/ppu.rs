#[derive(Debug)]
pub struct Ppu {}

impl Ppu {
    pub fn new() -> Self {
        Self {}
    }
    pub fn frame(&mut self) -> Vec<u8> {
        vec![0; 160 * 144]
    }
    pub fn clock(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
