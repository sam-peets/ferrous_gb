use std::ptr::fn_addr_eq;

use anyhow::anyhow;

use crate::core::{
    mmu::Mmu,
    register::{CpuRegisters, Register},
};

#[derive(Debug)]
pub struct Cpu {
    registers: CpuRegisters,
    mmu: Mmu,
    delay: usize,
}

impl Cpu {
    pub fn new(rom: Vec<u8>) -> Self {
        Cpu {
            registers: CpuRegisters::default(),
            mmu: Mmu::new(rom),
            delay: 0,
        }
    }

    pub fn cycle(&mut self) -> anyhow::Result<()> {
        if self.delay > 0 {
            self.delay -= 1;
            return Ok(());
        }

        log::trace!("cycle: cpu state: {:?}", self.registers);
        let opcode = self.mmu.read(self.registers.pc.read())?;
        match opcode {
            0x01 | 0x11 | 0x21 | 0x31 => self.ld_r16_u16(opcode)?,
            0xa8..=0xad | 0xaf => self.xor_a_r(opcode)?,
            0x22 => self.ld_hli_a()?,
            0x32 => self.ld_hld_a()?,
            0xcb => {
                let opcode = self.mmu.read(self.registers.pc.read() + 1)?;
                match opcode {
                    0x40..=0x45
                    | 0x47..=0x4d
                    | 0x4f
                    | 0x50..=0x55
                    | 0x57..=0x5d
                    | 0x5f
                    | 0x60..=0x65
                    | 0x67..=0x6d
                    | 0x6f
                    | 0x70..=0x75
                    | 0x77..=0x7d
                    | 0x7f => self.bit_b_r(opcode),
                    _ => Err(anyhow!("cycle: unknown CB opcode: {opcode:x?}")),
                }
            }?,
            _ => {
                return Err(anyhow!("cycle: unknown opcode: {opcode:x?}"));
            }
        }
        log::trace!("cycle: opcode {opcode:x?}");
        Ok(())
    }

    fn ld_r16_u16(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = (opcode & 0b00110000) >> 4;
        let arg_high = self.mmu.read(self.registers.pc.read() + 1)? as u16;
        let arg_low = self.mmu.read(self.registers.pc.read() + 1)? as u16;
        let arg = (arg_high << 8) | arg_low;
        self.registers.get_r16(reg).write(arg);
        self.registers.pc += 3;
        self.delay += 2;
        Ok(())
    }

    fn xor_a_r(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let arg = self.registers.af.high;
        let target = self.registers.get_r8(reg);
        target.write(arg.into());

        if target.read() == 0 {
            self.registers.af.low.z = true;
        }
        self.registers.af.low.c = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 1;
        self.delay += 0;
        Ok(())
    }

    fn ld_hli_a(&mut self) -> anyhow::Result<()> {
        let addr: u16 = self.registers.hl.into();
        self.mmu.write(addr, self.registers.af.high.into())?;
        self.registers.hl += 1;
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }
    fn ld_hld_a(&mut self) -> anyhow::Result<()> {
        let addr: u16 = self.registers.hl.into();
        self.mmu.write(addr, self.registers.af.high.into())?;
        self.registers.hl -= 1;
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }

    fn bit_b_r(&mut self, opcode: u8) -> anyhow::Result<()> {
        let register = opcode & 0b00000111;
        let bit = (opcode & 0b00111000) >> 3;
        let target = self.registers.get_r8(register);
        self.registers.af.low.z = (target.read() & (1 << bit)) > 0;
        self.registers.af.low.h = true;
        self.registers.af.low.n = false;
        self.delay += 1;
        self.registers.pc += 2;
        Ok(())
    }

    fn jr_cond_i8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let offset: u8 = self.mmu.read(self.registers.pc.read() + 1)?;
        let offset = offset as i8;
        let offset = offset as i16 + 2; // per reference
        let cond = self.registers.get_cond(opcode);
        log::trace!("jr_cond_i8: conditional jump to 0x{offset:x?}, {cond}");

        if cond {
            let pc = self.registers.pc.read() as i16;
            let pc = pc + offset;
            let pc = pc as u16;
            self.registers.pc.write(pc);
        } else {
            todo!()
        }
        Ok(())
    }
}
