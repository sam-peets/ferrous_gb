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
    pub cycles: usize,
}

impl Cpu {
    pub fn new(rom: Vec<u8>) -> Self {
        Cpu {
            registers: CpuRegisters::default(),
            mmu: Mmu::new(rom),
            delay: 0,
            cycles: 0,
        }
    }

    pub fn cycle(&mut self) -> anyhow::Result<()> {
        match self.delay {
            0 => {}
            1 => {
                self.delay -= 1;
            }
            _ => {
                self.delay -= 1;
                return Ok(());
            }
        }

        if self.mmu.booting && self.registers.pc.read() == 0x100 {
            self.mmu.booting = false;
            log::info!("cycle: finished booting");
        }

        self.cycles += 1;

        log::trace!("cycle: cpu state: {:?}", self.registers);
        let opcode = self.mmu.read(self.registers.pc.read())?;
        log::trace!(
            "cycle: opcode 0x{opcode:x?}, pc: 0x{:x?}",
            self.registers.pc.read()
        );
        match opcode {
            0x01 | 0x11 | 0x21 | 0x31 => self.ld_r16_u16(opcode)?,
            0xa8..=0xad | 0xaf => self.xor_a_r(opcode)?,
            0x22 => self.ld_ptr_hli_a()?,
            0x32 => self.ld_ptr_hld_a()?,
            0x20 | 0x30 | 0x28 | 0x38 => self.jr_cond_i8(opcode)?,
            0x06 | 0x16 | 0x26 | 0x0e | 0x1e | 0x2e | 0x3e => self.ld_r8_u8(opcode)?,
            0xe2 => self.ld_ptr_ff00_c_a()?,
            0x04 | 0x14 | 0x24 | 0x0c | 0x1c | 0x2c | 0x3c => self.inc_r8(opcode)?,
            0x70..=0x75 | 0x77 => self.ld_ptr_hl_r(opcode)?,
            0xe0 => self.ld_ptr_ff00_u8_a()?,
            0x0a => self.ld_a_ptr_bc()?,
            0x1a => self.ld_a_ptr_de()?,
            0xcd => self.call_u16()?,
            0x40..=0x45
            | 0x47..=0x4d
            | 0x4f
            | 0x50..=0x55
            | 0x57..=0x5d
            | 0x5f
            | 0x60..=0x65
            | 0x67..=0x6d
            | 0x6f
            | 0x78..=0x7d
            | 0x7f => self.ld_r8_r8(opcode)?,
            0xc5 | 0xd5 | 0xe5 | 0xf5 => self.push_r16(opcode)?,
            0xc1 | 0xd1 | 0xe1 | 0xf1 => self.pop_r16(opcode)?,
            0x17 => self.rla()?,
            0x05 | 0x15 | 0x25 | 0x0d | 0x1d | 0x2d | 0x3d => self.dec_r8(opcode)?,
            0x03 | 0x13 | 0x23 | 0x33 => self.inc_r16(opcode)?,
            0xc9 => self.ret()?,
            0xfe => self.cp_a_u8()?,
            0xea => self.ld_ptr_u16_a()?,
            0x18 => self.jr_i8()?,
            0xf0 => self.ld_a_ptr_ff00_u8()?,
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
                    | 0x7f => self.bit_b_r8(opcode),
                    0x10..=0x15 | 0x17 => self.rl_r8(opcode),
                    _ => Err(anyhow!("cycle: unknown CB opcode: {opcode:x?}")),
                }
            }?,
            _ => {
                return Err(anyhow!("cycle: unknown opcode: {opcode:x?}"));
            }
        }
        Ok(())
    }

    fn ld_r16_u16(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = (opcode & 0b00110000) >> 4;
        let arg_high = self.mmu.read(self.registers.pc.read() + 2)? as u16;
        let arg_low = self.mmu.read(self.registers.pc.read() + 1)? as u16;
        let arg = (arg_high << 8) | arg_low;
        log::trace!("ld_r16_u16: {arg:x?}");
        self.registers.get_r16_ss(reg).write(arg);
        self.registers.pc += 3;
        self.delay += 3;
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
        self.delay += 1;
        Ok(())
    }

    fn ld_ptr_hli_a(&mut self) -> anyhow::Result<()> {
        let addr: u16 = self.registers.hl.into();
        self.mmu.write(addr, self.registers.af.high.into())?;
        self.registers.hl += 1;
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }
    fn ld_ptr_hld_a(&mut self) -> anyhow::Result<()> {
        let addr: u16 = self.registers.hl.into();
        self.mmu.write(addr, self.registers.af.high.into())?;
        self.registers.hl -= 1;
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn bit_b_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let register = opcode & 0b00000111;
        let bit = (opcode & 0b00111000) >> 3;
        let target = self.registers.get_r8(register);
        self.registers.af.low.z = (target.read() & (1 << bit)) == 0;
        self.registers.af.low.h = true;
        self.registers.af.low.n = false;
        self.delay += 2;
        self.registers.pc += 2;
        Ok(())
    }

    fn jr_cond_i8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let offset: u8 = self.mmu.read(self.registers.pc.read() + 1)?;
        let offset = offset as i8 as i16;
        let code = (opcode & 0b00011000) >> 3;
        let cond = self.registers.get_cond(code);
        log::trace!("jr_cond_i8: conditional jump to {offset}, {cond}");
        self.registers.pc += 2;
        if cond {
            let pc = self.registers.pc.read();
            let (pc, _) = pc.overflowing_add_signed(offset);
            self.registers.pc.write(pc);
            self.delay += 3;
        } else {
            self.delay += 2;
        }
        Ok(())
    }

    fn ld_r8_u8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = (opcode & 0b00111000) >> 3;
        let arg = self.mmu.read(self.registers.pc.read() + 1)?;
        self.registers.get_r8(reg).write(arg);
        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn ld_ptr_ff00_c_a(&mut self) -> anyhow::Result<()> {
        let addr = 0xff00 | (self.registers.bc.low.read() as u16);
        log::trace!("ld_ff00_c_a: writing {addr:x?}");
        self.mmu.write(addr, self.registers.af.high.read())?;
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn inc_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = (opcode & 0b00111000) >> 3;
        let target = self.registers.get_r8(reg);
        let val = target.read();
        let (new_val, _) = val.overflowing_add(1);
        target.write(new_val);
        self.registers.af.low.z = new_val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = (val & 0x0f) + 1 > 0x0f;
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }

    fn ld_ptr_hl_r(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let addr = self.registers.hl.read();
        self.mmu.write(addr, self.registers.get_r8(reg).read())?;
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn ld_ptr_ff00_u8_a(&mut self) -> anyhow::Result<()> {
        let offset = self.mmu.read(self.registers.pc.read() + 1)? as u16;
        let addr = 0xff00 | offset;
        log::trace!("ld_ff00_u8_a: writing {addr:x?}");
        self.mmu.write(addr, self.registers.af.high.read())?;
        self.registers.pc += 2;
        self.delay += 3;
        Ok(())
    }

    fn ld_a_ptr_bc(&mut self) -> anyhow::Result<()> {
        let addr = self.registers.bc.read();
        self.registers.af.high.write(self.mmu.read(addr)?);
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn ld_a_ptr_de(&mut self) -> anyhow::Result<()> {
        let addr = self.registers.de.read();
        self.registers.af.high.write(self.mmu.read(addr)?);
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn call_u16(&mut self) -> anyhow::Result<()> {
        let addr_high = self.mmu.read(self.registers.pc.read() + 2)? as u16;
        let addr_low = self.mmu.read(self.registers.pc.read() + 1)? as u16;
        let addr = (addr_high << 8) | addr_low;
        self.registers.pc += 3;
        self.mmu
            .write(self.registers.sp.read() - 1, self.registers.pc.high.read())?;
        self.mmu
            .write(self.registers.sp.read() - 2, self.registers.pc.low.read())?;
        self.registers.pc.write(addr);
        self.registers.sp -= 2;
        log::trace!("call_u16: calling subroutine 0x{addr:x?}");
        Ok(())
    }

    fn ld_r8_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let dest = (opcode & 0b00111000) >> 3;
        let src = opcode & 0b00000111;

        let src_value = {
            let r = self.registers.get_r8(src);
            r.read()
        };
        let dest = self.registers.get_r8(dest);
        dest.write(src_value);

        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }

    fn push_r16(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = (opcode & 0b00110000) >> 4;
        let val = self.registers.get_r16_qq(reg).read();
        let high = ((val & 0xff00) >> 8) as u8;
        let low = (val & 0x00ff) as u8;
        self.mmu.write(self.registers.sp.read() - 1, high)?;
        self.mmu.write(self.registers.sp.read() - 2, low)?;
        self.registers.sp -= 2;
        self.registers.pc += 1;
        self.delay += 4;
        Ok(())
    }

    fn rl_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let val = self.registers.get_r8(reg).read();
        let b7 = (val & 0b10000000) > 0;
        let val = {
            let val = val << 1;
            if self.registers.af.low.c {
                val | 0b00000001
            } else {
                val
            }
        };
        self.registers.get_r8(reg).write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = false;
        self.registers.af.low.c = b7;

        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn rla(&mut self) -> anyhow::Result<()> {
        let val = self.registers.af.high.read();
        let b7 = (val & 0b10000000) > 0;
        let val = {
            let val = val << 1;
            if self.registers.af.low.c {
                val | 0b00000001
            } else {
                val
            }
        };
        self.registers.af.high.write(val);

        self.registers.af.low.z = false;
        self.registers.af.low.n = false;
        self.registers.af.low.h = false;
        self.registers.af.low.c = b7;

        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }

    fn pop_r16(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = (opcode & 0b00110000) >> 4;
        let low = self.mmu.read(self.registers.sp.read())?;
        let high = self.mmu.read(self.registers.sp.read() + 1)?;
        let val = ((high as u16) << 8) | (low as u16);
        self.registers.get_r16_qq(reg).write(val);
        self.registers.sp += 2;
        self.registers.pc += 1;
        self.delay += 3;
        Ok(())
    }

    fn dec_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = (opcode & 0b00111000) >> 3;
        let target = self.registers.get_r8(reg);
        let val = target.read();
        let (new_val, _) = val.overflowing_sub(1);
        target.write(new_val);
        self.registers.af.low.z = new_val == 0;
        self.registers.af.low.n = true;
        self.registers.af.low.h = {
            let (x, _) = (val & 0x0f).overflowing_sub(1 & 0x0f);
            (x & 0x10) > 0
        };
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }

    fn inc_r16(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = (opcode & 0b00110000) >> 4;
        let target = self.registers.get_r16_ss(reg);
        let val = target.read();
        let (new_val, _) = val.overflowing_add(1);
        target.write(new_val);
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn ret(&mut self) -> anyhow::Result<()> {
        let low = self.mmu.read(self.registers.sp.read())?;
        let high = self.mmu.read(self.registers.sp.read() + 1)?;
        self.registers.pc.high.write(high);
        self.registers.pc.low.write(low);
        self.registers.sp += 2;
        self.delay += 4;
        Ok(())
    }

    fn cp_a_u8(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.pc.read() + 1)?;
        let a = self.registers.af.high.read();

        self.registers.af.low.z = a == arg;
        self.registers.af.low.n = true;
        self.registers.af.low.c = arg > a;
        self.registers.af.low.h = {
            let (x, _) = (arg & 0x0f).overflowing_sub(a & 0x0f);
            (x & 0x10) > 0
        };

        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn ld_ptr_u16_a(&mut self) -> anyhow::Result<()> {
        let low = self.mmu.read(self.registers.pc.read() + 1)?;
        let high = self.mmu.read(self.registers.pc.read() + 2)?;
        let addr = ((high as u16) << 8) | (low as u16);
        self.mmu.write(addr, self.registers.af.high.read())?;
        self.registers.pc += 3;
        self.delay += 4;
        Ok(())
    }

    fn jr_i8(&mut self) -> anyhow::Result<()> {
        let offset: u8 = self.mmu.read(self.registers.pc.read() + 1)?;
        let offset = offset as i8 as i16;
        log::trace!("jr_i8: unconditional jump to {offset}");
        self.registers.pc += 2;

        let pc = self.registers.pc.read();
        let (pc, _) = pc.overflowing_add_signed(offset);
        self.registers.pc.write(pc);

        self.delay += 3;

        Ok(())
    }

    fn ld_a_ptr_ff00_u8(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.pc.read() + 1)? as u16;
        let addr = 0xff00 | arg;
        let val = self.mmu.read(addr)?;
        self.registers.af.high.write(val);

        self.registers.pc += 2;
        self.delay += 3;

        Ok(())
    }
}
