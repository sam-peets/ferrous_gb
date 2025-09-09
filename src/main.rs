use std::{
    fs::{self, File},
    io::{BufReader, Read},
};

use crate::core::cpu::Cpu;

mod core;

fn main() -> anyhow::Result<()> {
    let env = env_logger::Env::default().default_filter_or("trace");
    env_logger::Builder::from_env(env)
        .target(env_logger::Target::Stdout)
        .init();
    let args: Vec<String> = std::env::args().collect();

    let rom = fs::read(args[1].clone())?;

    let mut cpu = Cpu::new(rom);

    loop {
        match cpu.cycle() {
            Ok(()) => {}
            Err(e) => {
                log::info!("main: failed after {} cycles: {e}", cpu.cycles);
                Err(e)?;
            }
        }
    }

    Ok(())
}
