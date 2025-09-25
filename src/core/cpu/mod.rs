pub mod opcodes;
pub mod register;
use anyhow::anyhow;

use crate::core::{
    cpu::register::{CpuRegisters, Register},
    mmu::Mmu,
    ppu::Ppu,
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
    pub timer_overflow: bool,
}

impl Cpu {
    pub fn new(rom: &[u8], sample_rate: u32) -> anyhow::Result<Self> {
        let cpu = Cpu {
            registers: CpuRegisters::default(),
            mmu: Mmu::new(rom, sample_rate)?,
            delay: 0,
            cycles: 0,
            ppu: Ppu::new(),
            ime: false,
            logging: false,
            halted: false,
            dma_idx: 0,
            timer_overflow: true,
        };
        Ok(cpu)
    }

    pub fn new_fastboot(rom: &[u8], sample_rate: u32) -> anyhow::Result<Self> {
        let mut cpu = Cpu::new(rom, sample_rate)?;
        cpu.registers.af.write(0x01b0);
        cpu.registers.bc.write(0x0013);
        cpu.registers.de.write(0x00d8);
        cpu.registers.hl.write(0x014d);
        cpu.registers.pc.write(0x0100);
        cpu.registers.sp.write(0xfffe);
        cpu.mmu.io.bank = 0xff;
        cpu.mmu.sys = 0xabcc;
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
        self.delay += 5;
        Ok(())
    }

    pub fn cycle(&mut self) -> anyhow::Result<()> {
        if self.mmu.sys % 4 != 0 {
            // only clock the cpu on an m-cycle
            return Ok(());
        }
        if self.logging {
            println!(
                "SYS: {} IME: {} IE: {:x?} IF: {:x?} TIMA: {:x?} TAC: {:x?} TMA: {:x?} DIV: {:x?}",
                self.mmu.sys,
                self.ime,
                self.mmu.ie,
                self.mmu.io.interrupt,
                self.mmu.io.tima,
                self.mmu.io.tac,
                self.mmu.io.tma,
                self.mmu.read(0xff04)?,
            );
        }
        if self.timer_overflow {
            // should be delayed by one m-cycle
            self.mmu.io.interrupt |= 0b00000100;
            self.mmu.io.tima = self.mmu.io.tma;
            self.timer_overflow = false;
        }
        if (self.mmu.io.tac & 0b00000100) > 0 {
            // timer is enabled, tick it
            let interval = match self.mmu.io.tac & 0b0000_0011 {
                0b00 => 1024,
                0b01 => 16,
                0b10 => 64,
                0b11 => 256,
                _ => unreachable!(),
            };

            if self.mmu.sys % interval == 0 {
                let (val, overflow) = self.mmu.io.tima.overflowing_add(1);
                self.mmu.io.tima = val;
                if overflow {
                    log::debug!("timer overflow");
                    self.timer_overflow = true;
                }
            }
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
                "A: {:02X?} F: {:02X?} B: {:02X?} C: {:02X?} D: {:02X?} E: {:02X?} H: {:02X?} L: {:02X?} SP: {:04X?} PC: 00:{:04X?} ({:02X?} {:02X?} {:02X?} {:02X?}) (HL): {:02X?}",
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
                self.mmu.read(self.registers.hl.read())?
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
}
