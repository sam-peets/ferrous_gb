use anyhow::anyhow;

use crate::core::{
    mmu::Mmu,
    ppu::Ppu,
    register::{CpuRegisters, Register},
};

#[derive(Debug)]
pub struct Cpu {
    registers: CpuRegisters,
    pub mmu: Mmu,
    delay: usize,
    pub cycles: usize,
    pub ppu: Ppu,
    ime: bool,
    pub logging: bool,
    pub halted: bool,
    pub dma_idx: u8,
}

impl Cpu {
    pub fn new(rom: Vec<u8>) -> anyhow::Result<Self> {
        let cpu = Cpu {
            registers: CpuRegisters::default(),
            mmu: Mmu::new(rom)?,
            delay: 0,
            cycles: 0,
            ppu: Ppu::new(),
            ime: false,
            logging: true,
            halted: false,
            dma_idx: 0,
        };
        Ok(cpu)
    }

    pub fn new_fastboot(rom: Vec<u8>) -> anyhow::Result<Self> {
        let mut cpu = Cpu::new(rom)?;
        cpu.registers.af.write(0x01b0);
        cpu.registers.bc.write(0x0013);
        cpu.registers.de.write(0x00d8);
        cpu.registers.hl.write(0x014d);
        cpu.registers.pc.write(0x0100);
        cpu.registers.sp.write(0xfffe);
        cpu.mmu.io.bank = 0xff;
        Ok(cpu)
    }

    pub fn call_interrupt(&mut self, addr: u16, b: u8) -> anyhow::Result<()> {
        self.mmu.io.interrupt &= !(1 << b);
        self.ime = false;
        self.mmu
            .write(self.registers.sp.read() - 1, self.registers.pc.high.read())?;
        self.mmu
            .write(self.registers.sp.read() - 2, self.registers.pc.low.read())?;
        self.registers.sp -= 2;
        self.registers.pc.write(addr);
        self.halted = false;
        Ok(())
    }

    pub fn cycle(&mut self, clock: i32) -> anyhow::Result<()> {
        if (self.mmu.io.tac & 0b00000100) > 0 {
            // timer is enabled, tick it
            let interval = match self.mmu.io.tac & 0b0000_0011 {
                0b00 => 1024,
                0b01 => 16,
                0b10 => 64,
                0b11 => 256,
                _ => unreachable!(),
            };
            if clock % interval == 0 {
                let (val, overflow) = self.mmu.io.tima.overflowing_add(1);
                self.mmu.io.tima = val;
                if overflow {
                    log::debug!("timer overflow");
                    self.mmu.io.interrupt |= 0b00000100;
                    self.mmu.io.tima = self.mmu.io.tma;
                }
            }
        }
        if clock % 256 == 0 {
            let (val, _) = self.mmu.io.div.overflowing_add(1);
            self.mmu.io.div = val;
        }

        if clock % 4 != 0 {
            // not an m-cycle
            return Ok(());
        }

        if self.mmu.dma_requsted {
            self.dma_idx = 161; // TODO: verify
            self.mmu.dma_requsted = false;
            log::debug!("cycle: DMA: starting transfer")
        }

        if self.dma_idx > 160 {
            // apparently there's 2 delay cycles
            self.dma_idx -= 1;
        } else if self.dma_idx > 0 {
            let offset = 160 - self.dma_idx as u16;
            let (src, _) = ((self.mmu.io.dma as u16) << 8).overflowing_add(offset);
            let (dest, _) = 0xfe00u16.overflowing_add(offset); // 0xfe00 is the base address for OAM
            self.mmu.write(dest, self.mmu.read(src)?)?;
            self.dma_idx -= 1;
            log::trace!("cycle: DMA: copied from 0x{src:04x?} to 0x{dest:04x?}");
        }

        self.cycles += 1;
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

        log::trace!("cycle: cpu state: {:?}", self.registers);

        if self.ime && (self.mmu.ie & self.mmu.io.interrupt) > 0 {
            // interrupts are enabled and at least one has been requested
            log::debug!("interrupts are enabled and one has been requested");
            if (self.mmu.ie & self.mmu.io.interrupt & 0b00000001) > 0 {
                // vblank
                log::debug!("cycle: servicing vblank interrupt");
                self.call_interrupt(0x40, 0)?;
            } else if (self.mmu.ie & self.mmu.io.interrupt & 0b00000010) > 0 {
                // lcd
                log::debug!("cycle: servicing lcd interrupt");
                self.call_interrupt(0x48, 1)?;
            } else if (self.mmu.ie & self.mmu.io.interrupt & 0b00000100) > 0 {
                // timer
                log::debug!("cycle: servicing timer interrupt");
                self.call_interrupt(0x50, 2)?;
            } else if (self.mmu.ie & self.mmu.io.interrupt & 0b00001000) > 0 {
                // serial
                log::debug!("cycle: servicing serial interrupt");
                self.call_interrupt(0x58, 3)?;
            } else if (self.mmu.ie & self.mmu.io.interrupt & 0b00010000) > 0 {
                // joypad
                log::debug!("cycle: servicing joypad interrupt");
                self.call_interrupt(0x60, 4)?;
            }
        }

        if self.halted {
            if (self.mmu.ie & self.mmu.io.interrupt) > 0 {
                self.halted = false;
            }
            return Ok(());
        }

        let opcode = self.mmu.read(self.registers.pc.read())?;
        log::trace!(
            "cycle: opcode 0x{opcode:x?}, pc: 0x{:x?}",
            self.registers.pc.read()
        );
        if self.logging {
            println!(
                "A: {:02X?} F: {:02X?} B: {:02X?} C: {:02X?} D: {:02X?} E: {:02X?} H: {:02X?} L: {:02X?} SP: {:04X?} PC: 00:{:04X?} ({:02X?} {:02X?} {:02X?} {:02X?})",
                self.registers.af.high.read(),
                self.registers.af.low.read(),
                self.registers.bc.high.read(),
                self.registers.bc.low.read(),
                self.registers.de.high.read(),
                self.registers.de.low.read(),
                self.registers.hl.high.read(),
                self.registers.hl.low.read(),
                self.registers.sp.read(),
                self.registers.pc.read(),
                self.mmu.read(self.registers.pc.read())?,
                self.mmu.read(self.registers.pc.read() + 1)?,
                self.mmu.read(self.registers.pc.read() + 2)?,
                self.mmu.read(self.registers.pc.read() + 3)?,
            );
            println!(
                "IME: {} IE: {:x?} IF: {:x?}",
                self.ime, self.mmu.ie, self.mmu.io.interrupt
            );
        }

        match opcode {
            0x01 | 0x11 | 0x21 | 0x31 => self.ld_r16_u16(opcode)?,
            0xa8..=0xad | 0xaf => self.xor_a_r8(opcode)?,
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
            0x90..=0x95 | 0x97 => self.sub_a_r8(opcode)?,
            0xbe => self.cp_a_ptr_hl()?,
            0x86 => self.add_a_ptr_hl()?,
            0x00 => self.nop()?,
            0xc3 => self.jp_u16()?,
            0x2a => self.ld_a_ptr_hli()?,
            0x3a => self.ld_a_ptr_hld()?,
            0x02 => self.ld_ptr_bc_a()?,
            0x12 => self.ld_ptr_de_a()?,
            0xf3 => self.di()?,
            0xfb => self.ei()?,
            0x36 => self.ld_ptr_hl_u8()?,
            0x0b | 0x1b | 0x2b | 0x3b => self.dec_r16(opcode)?,
            0xb0..=0xb5 | 0xb7 => self.or_a_r8(opcode)?,
            0x2f => self.cpl()?,
            0xe6 => self.and_a_u8()?,
            0xfa => self.ld_a_ptr_u16()?,
            0xc4 | 0xd4 | 0xcc | 0xdc => self.call_cond_u16(opcode)?,
            0xc6 => self.add_a_u8()?,
            0xd6 => self.sub_a_u8()?,
            0x46 | 0x56 | 0x66 | 0x4e | 0x5e | 0x6e | 0x7e => self.ld_r8_ptr_hl(opcode)?,
            0xae => self.xor_a_ptr_hl()?,
            0x1f => self.rra()?,
            0xee => self.xor_a_u8()?,
            0xce => self.adc_a_u8()?,
            0xc0 | 0xd0 | 0xc8 | 0xd8 => self.ret_cond(opcode)?,
            0xb6 => self.or_a_ptr_hl()?,
            0x35 => self.dec_ptr_hl()?,
            0x34 => self.inc_ptr_hl()?,
            0x09 | 0x19 | 0x29 | 0x39 => self.add_hl_rr(opcode)?,
            0xe9 => self.jp_hl()?,
            0xa0..=0xa5 | 0xa7 => self.and_a_r8(opcode)?,
            0xc7 | 0xd7 | 0xe7 | 0xf7 | 0xcf | 0xdf | 0xef | 0xff => self.rst(opcode)?,
            0x80..=0x85 | 0x87 => self.add_a_r8(opcode)?,
            0xf6 => self.or_a_u8()?,
            0x08 => self.ld_ptr_u16_sp()?,
            0xf9 => self.ld_sp_hl()?,
            0xc2 | 0xd2 | 0xca | 0xda => self.jp_cond_u16(opcode)?,
            0xd9 => self.reti()?,
            0xe8 => self.add_sp_i8()?,
            0xf8 => self.ld_hl_sp_i8()?,
            0xb8..=0xbd | 0xbf => self.cp_a_r8(opcode)?,
            0xde => self.sbc_a_u8()?,
            0x37 => self.scf()?,
            0x3f => self.ccf()?,
            0x88..=0x8d | 0x8f => self.adc_a_r8(opcode)?,
            0x98..=0x9d | 0x9f => self.sbc_a_r8(opcode)?,
            0x07 => self.rlca()?,
            0x0f => self.rrca()?,
            0xf2 => self.ld_a_ptr_ff00_c()?,
            0x8e => self.adc_a_ptr_hl()?,
            0x96 => self.sub_a_ptr_hl()?,
            0x9e => self.sbc_a_ptr_hl()?,
            0xa6 => self.and_a_ptr_hl()?,
            0x27 => self.daa()?,
            0x76 => self.halt()?,
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
                    0x18..=0x1d | 0x1f => self.rr_r8(opcode),
                    0x30..=0x35 | 0x37 => self.swap_r8(opcode),
                    0x38..=0x3d | 0x3f => self.srl_r8(opcode),
                    0xc0..=0xc5
                    | 0xc7
                    | 0xd0..=0xd5
                    | 0xd7
                    | 0xe0..=0xe5
                    | 0xe7
                    | 0xf0..=0xf5
                    | 0xf7
                    | 0xc8..=0xcd
                    | 0xcf
                    | 0xd8..=0xdd
                    | 0xdf
                    | 0xe8..=0xed
                    | 0xef
                    | 0xf8..=0xfd
                    | 0xff => self.set_b_r8(opcode),
                    0xc6 | 0xd6 | 0xe6 | 0xf6 | 0xce | 0xde | 0xee | 0xfe => {
                        self.set_b_ptr_hl(opcode)
                    }
                    0x00..=0x05 | 0x07 => self.rlc_r8(opcode),
                    0x20..=0x25 | 0x27 => self.sla_r8(opcode),
                    0x08..=0x0d | 0x0f => self.rrc_r8(opcode),
                    0x28..=0x2d | 0x2f => self.sra_r8(opcode),
                    0x80..=0x85
                    | 0x87..=0x8d
                    | 0x8f
                    | 0x90..=0x95
                    | 0x97..=0x9d
                    | 0x9f
                    | 0xa0..=0xa5
                    | 0xa7..=0xad
                    | 0xaf
                    | 0xb0..=0xb5
                    | 0xb7..=0xbd
                    | 0xbf => self.res_b_r8(opcode),
                    0x06 => self.rlc_ptr_hl(),
                    0x0e => self.rrc_ptr_hl(),
                    0x16 => self.rl_ptr_hl(),
                    0x1e => self.rr_ptr_hl(),
                    0x26 => self.sla_ptr_hl(),
                    0x2e => self.sra_ptr_hl(),
                    0x36 => self.swap_ptr_hl(),
                    0x3e => self.srl_ptr_hl(),
                    0x46 | 0x4e | 0x56 | 0x5e | 0x66 | 0x6e | 0x76 | 0x7e => {
                        self.bit_b_ptr_hl(opcode)
                    }
                    0x86 | 0x8e | 0x96 | 0x9e | 0xa6 | 0xae | 0xb6 | 0xbe => {
                        self.res_b_ptr_hl(opcode)
                    }
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

    fn xor_a_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let a = self.registers.af.high.read();
        let target = self.registers.get_r8(reg).read();
        let val = a ^ target;
        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
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
            let (x, _) = (a & 0x0f).overflowing_sub(arg & 0x0f);
            (x & 0x10) > 0
        };

        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn cp_a_ptr_hl(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.hl.read())?;
        let a = self.registers.af.high.read();

        self.registers.af.low.z = a == arg;
        self.registers.af.low.n = true;
        self.registers.af.low.c = arg > a;
        self.registers.af.low.h = {
            let (x, _) = (a & 0x0f).overflowing_sub(arg & 0x0f);
            (x & 0x10) > 0
        };

        self.registers.pc += 1;
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

    fn sub_a_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let arg = self.registers.get_r8(reg).read();

        let a = self.registers.af.high.read();

        self.registers.af.low.z = a == arg;
        self.registers.af.low.n = true;
        self.registers.af.low.c = arg > a;
        self.registers.af.low.h = {
            let (x, _) = (a & 0x0f).overflowing_sub(arg & 0x0f);
            (x & 0x10) > 0
        };

        let (val, _) = a.overflowing_sub(arg);
        self.registers.af.high.write(val);

        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }

    fn add_a_ptr_hl(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.hl.read())?;
        let a = self.registers.af.high.read();

        self.registers.af.low.n = false;
        self.registers.af.low.h = {
            let (x, _) = (a & 0x0f).overflowing_add(arg & 0x0f);
            (x & 0x10) > 0
        };

        let (val, overflow) = a.overflowing_add(arg);
        self.registers.af.high.write(val);
        self.registers.af.low.c = overflow;
        self.registers.af.low.z = val == 0;
        self.registers.pc += 1;

        self.delay += 2;
        Ok(())
    }

    fn nop(&mut self) -> anyhow::Result<()> {
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }

    fn jp_u16(&mut self) -> anyhow::Result<()> {
        let low = self.mmu.read(self.registers.pc.read() + 1)?;
        let high = self.mmu.read(self.registers.pc.read() + 2)?;
        let addr = ((high as u16) << 8) | (low as u16);
        self.registers.pc.write(addr);
        self.delay += 4;
        Ok(())
    }

    fn ld_a_ptr_hli(&mut self) -> anyhow::Result<()> {
        let val = self.mmu.read(self.registers.hl.read())?;
        self.registers.af.high.write(val);
        self.registers.hl += 1;
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }
    fn ld_a_ptr_hld(&mut self) -> anyhow::Result<()> {
        let val = self.mmu.read(self.registers.hl.read())?;
        self.registers.af.high.write(val);
        self.registers.hl -= 1;
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }
    fn ld_ptr_de_a(&mut self) -> anyhow::Result<()> {
        self.mmu
            .write(self.registers.de.read(), self.registers.af.high.read())?;
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }
    fn ld_ptr_bc_a(&mut self) -> anyhow::Result<()> {
        self.mmu
            .write(self.registers.bc.read(), self.registers.af.high.read())?;
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn di(&mut self) -> anyhow::Result<()> {
        log::debug!("DI");
        self.ime = false;
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }
    fn ld_ptr_hl_u8(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.pc.read() + 1)?;
        self.mmu.write(self.registers.hl.read(), arg)?;
        self.registers.pc += 2;
        self.delay += 3;
        Ok(())
    }

    fn dec_r16(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = (opcode & 0b00110000) >> 4;
        let target = self.registers.get_r16_ss(reg);
        let val = target.read();
        let (val, _) = val.overflowing_sub(1);
        target.write(val);
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn or_a_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;

        let target = {
            let r = self.registers.get_r8(reg);
            r.read()
        };
        let a = self.registers.af.high.read();
        let val = target | a;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.af.high.write(val);
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }
    fn ei(&mut self) -> anyhow::Result<()> {
        log::debug!("EI");
        self.ime = true;
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }

    fn cpl(&mut self) -> anyhow::Result<()> {
        let val = self.registers.af.high.read();
        self.registers.af.high.write(!val);
        self.registers.af.low.h = true;
        self.registers.af.low.n = true;
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }
    fn and_a_u8(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.pc.read() + 1)?;
        let a = self.registers.af.high.read();
        let val = a & arg;
        self.registers.af.high.write(val);
        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = true;
        self.registers.af.low.c = false;
        self.registers.pc += 2;
        self.delay += 2;

        Ok(())
    }

    fn ld_a_ptr_u16(&mut self) -> anyhow::Result<()> {
        let low = self.mmu.read(self.registers.pc.read() + 1)?;
        let high = self.mmu.read(self.registers.pc.read() + 2)?;
        let addr = ((high as u16) << 8) | (low as u16);
        let val = self.mmu.read(addr)?;
        self.registers.af.high.write(val);
        self.registers.pc += 3;
        self.delay += 4;

        Ok(())
    }

    fn call_cond_u16(&mut self, opcode: u8) -> anyhow::Result<()> {
        let code = (opcode & 0b00011000) >> 3;
        let cond = self.registers.get_cond(code);

        if !cond {
            self.registers.pc += 3;
            self.delay += 3;
            return Ok(());
        }

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
        log::trace!("call_cond_u16: calling subroutine 0x{addr:x?}");
        Ok(())
    }

    fn add_a_u8(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.pc.read() + 1)?;
        let a = self.registers.af.high.read();
        let (val, c) = a.overflowing_add(arg);
        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = {
            let (x, _) = (a & 0x0f).overflowing_add(arg & 0x0f);
            (x & 0x10) > 0
        };
        self.registers.af.low.c = c;

        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn sub_a_u8(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.pc.read() + 1)?;
        let a = self.registers.af.high.read();
        let (val, c) = a.overflowing_sub(arg);
        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = true;
        self.registers.af.low.h = {
            let (x, _) = (a & 0x0f).overflowing_sub(arg & 0x0f);
            (x & 0x10) > 0
        };
        self.registers.af.low.c = c;

        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn ld_r8_ptr_hl(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = (opcode & 0b00111000) >> 3;
        let val = self.mmu.read(self.registers.hl.read())?;
        let target = self.registers.get_r8(reg);
        target.write(val);
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn xor_a_ptr_hl(&mut self) -> anyhow::Result<()> {
        let a = self.registers.af.high.read();
        let target = self.mmu.read(self.registers.hl.read())?;
        let val = a ^ target;
        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn srl_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let target = self.registers.get_r8(reg);
        let b0 = target.read() & 0b1;
        let val = target.read() >> 1;
        target.write(val);

        self.registers.af.low.z = target.read() == 0;
        self.registers.af.low.c = b0 > 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 2;
        self.delay += 2;

        Ok(())
    }

    fn rr_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let target = self.registers.get_r8(reg).read();
        let b0 = target & 0b1;
        let val = if self.registers.af.low.c {
            (target >> 1) | 0b10000000
        } else {
            target >> 1
        };
        self.registers.get_r8(reg).write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = b0 > 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 2;
        self.delay += 2;

        Ok(())
    }

    fn rra(&mut self) -> anyhow::Result<()> {
        let target = self.registers.af.high.read();
        let b0 = target & 0b1;
        let val = if self.registers.af.low.c {
            (target >> 1) | 0b10000000
        } else {
            target >> 1
        };
        self.registers.af.high.write(val);

        self.registers.af.low.z = false;
        self.registers.af.low.c = b0 > 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 1;
        self.delay += 1;

        Ok(())
    }
    fn xor_a_u8(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.pc.read() + 1)?;
        let a = self.registers.af.high.read();
        let val = a ^ arg;
        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }
    fn adc_a_u8(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.pc.read() + 1)?;
        let a = self.registers.af.high.read();
        let c = if self.registers.af.low.c { 1 } else { 0 };

        let (val, carry) = {
            let (val, c1) = a.overflowing_add(arg);
            let (val, c2) = val.overflowing_add(c);
            (val, c1 || c2)
        };

        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = carry;
        self.registers.af.low.h = {
            let (a, _) = (a & 0xf).overflowing_add(arg & 0xf);
            let (a, _) = a.overflowing_add(c);
            a >= 0x10
        };
        self.registers.af.low.n = false;

        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn ret_cond(&mut self, opcode: u8) -> anyhow::Result<()> {
        let code = (opcode & 0b00011000) >> 3;
        let cond = self.registers.get_cond(code);
        if !cond {
            self.registers.pc += 1;
            self.delay += 2;
            return Ok(());
        }

        let low = self.mmu.read(self.registers.sp.read())?;
        let high = self.mmu.read(self.registers.sp.read() + 1)?;
        self.registers.pc.high.write(high);
        self.registers.pc.low.write(low);
        self.registers.sp += 2;
        self.delay += 5;
        Ok(())
    }

    fn or_a_ptr_hl(&mut self) -> anyhow::Result<()> {
        let target = self.mmu.read(self.registers.hl.read())?;
        let a = self.registers.af.high.read();
        let val = target | a;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.af.high.write(val);
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn dec_ptr_hl(&mut self) -> anyhow::Result<()> {
        let target = self.mmu.read(self.registers.hl.read())?;
        let (val, _) = target.overflowing_sub(1);
        self.mmu.write(self.registers.hl.read(), val)?;

        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = {
            let (v, _) = (target & 0x0f).overflowing_sub(1);
            v >= 0x10
        };
        self.registers.af.low.n = true;
        self.registers.pc += 1;
        self.delay += 3;

        Ok(())
    }
    fn inc_ptr_hl(&mut self) -> anyhow::Result<()> {
        let target = self.mmu.read(self.registers.hl.read())?;
        let (val, _) = target.overflowing_add(1);
        self.mmu.write(self.registers.hl.read(), val)?;

        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = {
            let (v, _) = (target & 0x0f).overflowing_add(1);
            v >= 0x10
        };
        self.registers.af.low.n = false;
        self.registers.pc += 1;
        self.delay += 3;

        Ok(())
    }
    fn add_hl_rr(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = (opcode & 0b00110000) >> 4;
        let target = self.registers.get_r16_ss(reg).read();
        let hl = self.registers.hl.read();
        let (val, carry) = hl.overflowing_add(target);

        self.registers.af.low.n = false;
        self.registers.af.low.h = {
            let (v, _) = (hl & 0x0fff).overflowing_add(target & 0x0fff);
            (v & 0x1000) > 0
        };
        self.registers.af.low.c = carry;

        self.registers.hl.write(val);

        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn jp_hl(&mut self) -> anyhow::Result<()> {
        self.registers.pc.write(self.registers.hl.read());
        self.delay += 1;
        Ok(())
    }

    fn swap_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let target = self.registers.get_r8(reg);
        let r = target.read();
        let high = (r & 0xf0) >> 4;
        let low = r & 0x0f;
        let val = (low << 4) | high;

        target.write(val);
        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = false;
        self.registers.af.low.c = false;

        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }
    fn and_a_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let target = self.registers.get_r8(reg).read();
        let a = self.registers.af.high.read();
        let val = a & target;

        self.registers.af.high.write(val);
        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = true;
        self.registers.af.low.c = false;
        self.registers.pc += 1;
        self.delay += 1;

        Ok(())
    }

    fn rst(&mut self, opcode: u8) -> anyhow::Result<()> {
        let n = (opcode & 0b00111000) >> 3;
        let a = (n * 8) as u16;
        self.registers.pc += 1;
        self.mmu
            .write(self.registers.sp.read() - 1, self.registers.pc.high.read())?;
        self.mmu
            .write(self.registers.sp.read() - 2, self.registers.pc.low.read())?;
        self.registers.sp -= 2;
        self.registers.pc.write(a);

        self.delay += 4;

        Ok(())
    }

    fn add_a_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let arg = self.registers.get_r8(reg).read();
        let a = self.registers.af.high.read();
        let (val, c) = a.overflowing_add(arg);
        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = {
            let (x, _) = (a & 0x0f).overflowing_add(arg & 0x0f);
            (x & 0x10) > 0
        };
        self.registers.af.low.c = c;

        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }

    fn or_a_u8(&mut self) -> anyhow::Result<()> {
        let target = self.mmu.read(self.registers.pc.read() + 1)?;
        let a = self.registers.af.high.read();
        let val = target | a;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.af.high.write(val);
        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn set_b_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let bit = (opcode & 0b00111000) >> 3;
        let target = self.registers.get_r8(reg);
        let val = target.read() | (1 << bit);
        target.write(val);
        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn set_b_ptr_hl(&mut self, opcode: u8) -> anyhow::Result<()> {
        let bit = (opcode & 0b00111000) >> 3;
        let target = self.mmu.read(self.registers.hl.read())?;
        let val = target | (1 << bit);
        self.mmu.write(self.registers.hl.read(), val)?;
        self.registers.pc += 2;
        self.delay += 4;
        Ok(())
    }
    fn ld_ptr_u16_sp(&mut self) -> anyhow::Result<()> {
        let low = self.mmu.read(self.registers.pc.read() + 1)?;
        let high = self.mmu.read(self.registers.pc.read() + 2)?;
        let addr = ((high as u16) << 8) | (low as u16);
        self.mmu.write(addr, self.registers.sp.low.read())?;
        self.mmu.write(addr + 1, self.registers.sp.high.read())?;
        self.registers.pc += 3;
        self.delay += 5;
        Ok(())
    }
    fn ld_sp_hl(&mut self) -> anyhow::Result<()> {
        self.registers.sp.write(self.registers.hl.read());
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }
    fn jp_cond_u16(&mut self, opcode: u8) -> anyhow::Result<()> {
        let low = self.mmu.read(self.registers.pc.read() + 1)?;
        let high = self.mmu.read(self.registers.pc.read() + 2)?;
        let addr = ((high as u16) << 8) | (low as u16);
        let code = (opcode & 0b00011000) >> 3;
        let cond = self.registers.get_cond(code);
        if cond {
            self.registers.pc.write(addr);
            self.delay += 4;
        } else {
            self.registers.pc += 3;
            self.delay += 3;
        }

        Ok(())
    }
    fn reti(&mut self) -> anyhow::Result<()> {
        let low = self.mmu.read(self.registers.sp.read())?;
        let high = self.mmu.read(self.registers.sp.read() + 1)?;
        self.registers.pc.high.write(high);
        self.registers.pc.low.write(low);
        self.registers.sp += 2;
        self.delay += 4;
        self.ime = true;
        Ok(())
    }
    fn add_sp_i8(&mut self) -> anyhow::Result<()> {
        let offset: u8 = self.mmu.read(self.registers.pc.read() + 1)?;
        let offset = offset as i8 as i16;
        let sp = self.registers.sp.read();
        let (val, _) = sp.overflowing_add_signed(offset);

        self.registers.af.low.z = false;
        self.registers.af.low.n = false;
        self.registers.af.low.h = {
            let (x, _) = (sp & 0xf).overflowing_add_signed(offset & 0xf);
            (x & 0x10) > 0
        };
        self.registers.af.low.c = {
            let (x, _) = (sp & 0xff).overflowing_add_signed(offset & 0xff);
            (x & 0x100) > 0
        };
        self.registers.sp.write(val);
        self.registers.pc += 2;
        self.delay += 4;
        Ok(())
    }
    fn ld_hl_sp_i8(&mut self) -> anyhow::Result<()> {
        let offset: u8 = self.mmu.read(self.registers.pc.read() + 1)?;
        let offset = offset as i8 as i16;
        let sp = self.registers.sp.read();
        let (val, _) = sp.overflowing_add_signed(offset);

        self.registers.af.low.z = false;
        self.registers.af.low.n = false;
        self.registers.af.low.h = {
            let (x, _) = (sp & 0xf).overflowing_add_signed(offset & 0xf);
            (x & 0x10) > 0
        };
        self.registers.af.low.c = {
            let (x, _) = (sp & 0xff).overflowing_add_signed(offset & 0xff);
            (x & 0x100) > 0
        };
        self.registers.hl.write(val);
        self.registers.pc += 2;
        self.delay += 3;
        Ok(())
    }

    fn cp_a_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let arg = self.registers.get_r8(reg).read();
        let a = self.registers.af.high.read();

        self.registers.af.low.z = a == arg;
        self.registers.af.low.n = true;
        self.registers.af.low.c = arg > a;
        self.registers.af.low.h = {
            let (x, _) = (a & 0x0f).overflowing_sub(arg & 0x0f);
            (x & 0x10) > 0
        };

        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }

    fn sbc_a_u8(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.pc.read() + 1)?;
        let a = self.registers.af.high.read();
        let c = if self.registers.af.low.c { 1 } else { 0 };

        let (val, carry) = {
            let (val, c1) = a.overflowing_sub(arg);
            let (val, c2) = val.overflowing_sub(c);
            (val, c1 || c2)
        };

        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = carry;
        self.registers.af.low.h = {
            let (a, _) = (a & 0xf).overflowing_sub(arg & 0xf);
            let (a, _) = a.overflowing_sub(c);
            a >= 0x10
        };
        self.registers.af.low.n = true;

        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }
    fn scf(&mut self) -> anyhow::Result<()> {
        self.registers.af.low.n = false;
        self.registers.af.low.h = false;
        self.registers.af.low.c = true;
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }
    fn ccf(&mut self) -> anyhow::Result<()> {
        self.registers.af.low.n = false;
        self.registers.af.low.h = false;
        self.registers.af.low.c = !self.registers.af.low.c;
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }
    fn adc_a_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let arg = self.registers.get_r8(reg).read();
        let a = self.registers.af.high.read();
        let c = if self.registers.af.low.c { 1 } else { 0 };

        let (val, carry) = {
            let (val, c1) = a.overflowing_add(arg);
            let (val, c2) = val.overflowing_add(c);
            (val, c1 || c2)
        };

        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = carry;
        self.registers.af.low.h = {
            let (a, _) = (a & 0xf).overflowing_add(arg & 0xf);
            let (a, _) = a.overflowing_add(c);
            a >= 0x10
        };
        self.registers.af.low.n = false;

        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }

    fn sbc_a_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let arg = self.registers.get_r8(reg).read();
        let a = self.registers.af.high.read();
        let c = if self.registers.af.low.c { 1 } else { 0 };

        let (val, carry) = {
            let (val, c1) = a.overflowing_sub(arg);
            let (val, c2) = val.overflowing_sub(c);
            (val, c1 || c2)
        };

        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = carry;
        self.registers.af.low.h = {
            let (a, _) = (a & 0xf).overflowing_sub(arg & 0xf);
            let (a, _) = a.overflowing_sub(c);
            a >= 0x10
        };
        self.registers.af.low.n = true;

        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }
    pub fn rlca(&mut self) -> anyhow::Result<()> {
        let val = self.registers.af.high.read();
        let c = (val & 0b10000000) > 0;
        let val = if c { (val << 1) | 1 } else { val << 1 };
        self.registers.af.low.c = c;
        self.registers.af.low.z = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.af.high.write(val);
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }
    pub fn rrca(&mut self) -> anyhow::Result<()> {
        let val = self.registers.af.high.read();
        let c = (val & 0b00000001) > 0;
        let val = if c { (val >> 1) | 0b10000000 } else { val >> 1 };
        self.registers.af.low.c = c;
        self.registers.af.low.z = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.af.high.write(val);
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }
    pub fn rlc_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let code = opcode & 0b00000111;
        let target = self.registers.get_r8(code);
        let val = target.read();
        let c = (val & 0b10000000) > 0;
        let val = if c { (val << 1) | 1 } else { val << 1 };
        target.write(val);
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }
    fn rrc_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let code = opcode & 0b00000111;
        let target = self.registers.get_r8(code);
        let val = target.read();
        let c = (val & 0b00000001) > 0;
        let val = if c { (val >> 1) | 0b10000000 } else { val >> 1 };
        target.write(val);
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn sla_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let code = opcode & 0b00000111;
        let target = self.registers.get_r8(code);
        let val = target.read();
        let c = (val & 0b10000000) > 0;
        let val = val << 1;
        target.write(val);
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn sra_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let code = opcode & 0b00000111;
        let target = self.registers.get_r8(code);
        let val = target.read();
        let c = (val & 0b00000001) > 0;
        let b7 = (val & 0b10000000) > 0;
        let val = if b7 {
            (val >> 1) | 0b10000000
        } else {
            val >> 1
        };
        target.write(val);
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn res_b_r8(&mut self, opcode: u8) -> anyhow::Result<()> {
        let reg = opcode & 0b00000111;
        let bit = (opcode & 0b00111000) >> 3;
        let target = self.registers.get_r8(reg);
        let val = target.read() & !(1 << bit);
        target.write(val);
        self.registers.pc += 2;
        self.delay += 2;
        Ok(())
    }

    fn ld_a_ptr_ff00_c(&mut self) -> anyhow::Result<()> {
        let c = self.registers.bc.low.read() as u16;
        let addr = 0xff00 + c;
        let val = self.mmu.read(addr)?;
        self.registers.af.high.write(val);
        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }
    fn adc_a_ptr_hl(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.hl.read())?;
        let a = self.registers.af.high.read();
        let c = if self.registers.af.low.c { 1 } else { 0 };

        let (val, carry) = {
            let (val, c1) = a.overflowing_add(arg);
            let (val, c2) = val.overflowing_add(c);
            (val, c1 || c2)
        };

        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = carry;
        self.registers.af.low.h = {
            let (a, _) = (a & 0xf).overflowing_add(arg & 0xf);
            let (a, _) = a.overflowing_add(c);
            a >= 0x10
        };
        self.registers.af.low.n = false;

        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }

    fn sub_a_ptr_hl(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.hl.read())?;
        let a = self.registers.af.high.read();
        let (val, c) = a.overflowing_sub(arg);
        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = true;
        self.registers.af.low.h = {
            let (x, _) = (a & 0x0f).overflowing_sub(arg & 0x0f);
            (x & 0x10) > 0
        };
        self.registers.af.low.c = c;

        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }
    fn sbc_a_ptr_hl(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.hl.read())?;
        let a = self.registers.af.high.read();
        let c = if self.registers.af.low.c { 1 } else { 0 };

        let (val, carry) = {
            let (val, c1) = a.overflowing_sub(arg);
            let (val, c2) = val.overflowing_sub(c);
            (val, c1 || c2)
        };

        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = carry;
        self.registers.af.low.h = {
            let (a, _) = (a & 0xf).overflowing_sub(arg & 0xf);
            let (a, _) = a.overflowing_sub(c);
            a >= 0x10
        };
        self.registers.af.low.n = true;

        self.registers.pc += 1;
        self.delay += 2;
        Ok(())
    }
    fn and_a_ptr_hl(&mut self) -> anyhow::Result<()> {
        let arg = self.mmu.read(self.registers.hl.read())?;
        let a = self.registers.af.high.read();
        let val = a & arg;
        self.registers.af.high.write(val);
        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = true;
        self.registers.af.low.c = false;
        self.registers.pc += 1;
        self.delay += 2;

        Ok(())
    }
    pub fn rlc_ptr_hl(&mut self) -> anyhow::Result<()> {
        let val = self.mmu.read(self.registers.hl.read())?;
        let c = (val & 0b10000000) > 0;
        let val = if c { (val << 1) | 1 } else { val << 1 };
        self.mmu.write(self.registers.hl.read(), val)?;

        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 4;
        Ok(())
    }
    fn rrc_ptr_hl(&mut self) -> anyhow::Result<()> {
        let val = self.mmu.read(self.registers.hl.read())?;
        let c = (val & 0b00000001) > 0;

        let val = if c { (val >> 1) | 0b10000000 } else { val >> 1 };
        self.mmu.write(self.registers.hl.read(), val)?;
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 4;
        Ok(())
    }

    fn rl_ptr_hl(&mut self) -> anyhow::Result<()> {
        let val = self.mmu.read(self.registers.hl.read())?;

        let b7 = (val & 0b10000000) > 0;
        let val = {
            let val = val << 1;
            if self.registers.af.low.c {
                val | 0b00000001
            } else {
                val
            }
        };
        self.mmu.write(self.registers.hl.read(), val)?;

        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = false;
        self.registers.af.low.c = b7;

        self.registers.pc += 2;
        self.delay += 4;
        Ok(())
    }
    fn rr_ptr_hl(&mut self) -> anyhow::Result<()> {
        let target = self.mmu.read(self.registers.hl.read())?;
        let b0 = target & 0b1;
        let val = if self.registers.af.low.c {
            (target >> 1) | 0b10000000
        } else {
            target >> 1
        };
        self.mmu.write(self.registers.hl.read(), val)?;

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = b0 > 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 2;
        self.delay += 4;

        Ok(())
    }
    fn sla_ptr_hl(&mut self) -> anyhow::Result<()> {
        let val = self.mmu.read(self.registers.hl.read())?;
        let c = (val & 0b10000000) > 0;
        let val = val << 1;
        self.mmu.write(self.registers.hl.read(), val)?;
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 4;
        Ok(())
    }

    fn sra_ptr_hl(&mut self) -> anyhow::Result<()> {
        let val = self.mmu.read(self.registers.hl.read())?;
        let c = (val & 0b00000001) > 0;
        let b7 = (val & 0b10000000) > 0;
        let val = if b7 {
            (val >> 1) | 0b10000000
        } else {
            val >> 1
        };
        self.mmu.write(self.registers.hl.read(), val)?;
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 4;
        Ok(())
    }
    fn swap_ptr_hl(&mut self) -> anyhow::Result<()> {
        let r = self.mmu.read(self.registers.hl.read())?;
        let high = (r & 0xf0) >> 4;
        let low = r & 0x0f;
        let val = (low << 4) | high;

        self.mmu.write(self.registers.hl.read(), val)?;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = false;
        self.registers.af.low.c = false;

        self.registers.pc += 2;
        self.delay += 4;
        Ok(())
    }

    fn srl_ptr_hl(&mut self) -> anyhow::Result<()> {
        let r = self.mmu.read(self.registers.hl.read())?;
        let b0 = r & 0b1;
        let val = r >> 1;
        self.mmu.write(self.registers.hl.read(), val)?;

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = b0 > 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 2;
        self.delay += 4;

        Ok(())
    }

    fn bit_b_ptr_hl(&mut self, opcode: u8) -> anyhow::Result<()> {
        let bit = (opcode & 0b00111000) >> 3;
        let target = self.mmu.read(self.registers.hl.read())?;
        self.registers.af.low.z = (target & (1 << bit)) == 0;
        self.registers.af.low.h = true;
        self.registers.af.low.n = false;
        self.delay += 3;
        self.registers.pc += 2;
        Ok(())
    }

    fn res_b_ptr_hl(&mut self, opcode: u8) -> anyhow::Result<()> {
        let bit = (opcode & 0b00111000) >> 3;
        let target = self.mmu.read(self.registers.hl.read())?;
        let val = target & !(1 << bit);
        self.mmu.write(self.registers.hl.read(), val)?;
        self.registers.pc += 2;
        self.delay += 3;
        Ok(())
    }

    fn daa(&mut self) -> anyhow::Result<()> {
        // adapted from https://gbdev.gg8.se/wiki/articles/DAA
        if self.registers.af.low.n {
            if self.registers.af.low.c {
                self.registers.af.high -= 0x60;
            }
            if self.registers.af.low.h {
                self.registers.af.high -= 0x06;
            }
        } else {
            if self.registers.af.low.c || self.registers.af.high.read() > 0x99 {
                self.registers.af.high += 0x60;
                self.registers.af.low.c = true;
            }
            if self.registers.af.low.h || (self.registers.af.high.read() & 0x0f) > 0x09 {
                self.registers.af.high += 0x06;
            }
        }
        self.registers.af.low.z = self.registers.af.high.read() == 0;
        self.registers.af.low.h = false;

        self.registers.pc += 1;
        self.delay += 1;

        Ok(())
    }

    fn halt(&mut self) -> anyhow::Result<()> {
        log::debug!("halt!");
        self.halted = true;
        self.registers.pc += 1;
        self.delay += 1;
        Ok(())
    }
}
