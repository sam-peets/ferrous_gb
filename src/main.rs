use std::{
    fs::{self, File},
    io::{BufReader, Read},
};

use crate::core::cpu::Cpu;

mod core;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args: Vec<String> = std::env::args().collect();

    let rom = fs::read(args[1].clone())?;

    let mut cpu = Cpu::new(rom);

    loop {
        cpu.cycle()?;
    }

    Ok(())
}
