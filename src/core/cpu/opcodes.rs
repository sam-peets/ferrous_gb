use crate::core::{
    cpu::{Cpu, register::Register},
    util::extract,
};

impl Cpu {
    pub fn ld_r16_u16(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0011_0000);
        let arg_high = u16::from(self.mmu.read(self.registers.pc.read() + 2));
        let arg_low = u16::from(self.mmu.read(self.registers.pc.read() + 1));
        let arg = (arg_high << 8) | arg_low;
        log::trace!("ld_r16_u16: {arg:x?}");
        self.registers.get_r16_ss(reg).write(arg);
        self.registers.pc += 3;
        self.delay += 3;
    }

    pub fn xor_a_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
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
    }

    pub fn ld_ptr_hli_a(&mut self) {
        let addr: u16 = self.registers.hl.into();
        self.mmu.write(addr, self.registers.af.high.into());
        self.registers.hl += 1;
        self.registers.pc += 1;
        self.delay += 2;
    }
    pub fn ld_ptr_hld_a(&mut self) {
        let addr: u16 = self.registers.hl.into();
        self.mmu.write(addr, self.registers.af.high.into());
        self.registers.hl -= 1;
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn bit_b_r8(&mut self, opcode: u8) {
        let register = extract(opcode, 0b0000_0111);
        let bit = extract(opcode, 0b0011_1000);
        let target = self.registers.get_r8(register);
        self.registers.af.low.z = (target.read() & (1 << bit)) == 0;
        self.registers.af.low.h = true;
        self.registers.af.low.n = false;
        self.delay += 2;
        self.registers.pc += 2;
    }

    #[allow(clippy::cast_possible_wrap)]
    pub fn jr_cond_i8(&mut self, opcode: u8) {
        let offset: u8 = self.mmu.read(self.registers.pc.read() + 1);
        let offset = i16::from(offset as i8);
        let code = extract(opcode, 0b0001_1000);
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
    }

    pub fn ld_r8_u8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0011_1000);
        let arg = self.mmu.read(self.registers.pc.read() + 1);
        self.registers.get_r8(reg).write(arg);
        self.registers.pc += 2;
        self.delay += 2;
    }

    pub fn ld_ptr_ff00_c_a(&mut self) {
        let addr = 0xff00 | u16::from(self.registers.bc.low.read());
        log::trace!("ld_ff00_c_a: writing {addr:x?}");
        self.mmu.write(addr, self.registers.af.high.read());
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn inc_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0011_1000);
        let target = self.registers.get_r8(reg);
        let val = target.read();
        let (new_val, _) = val.overflowing_add(1);
        target.write(new_val);
        self.registers.af.low.z = new_val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = (val & 0x0f) + 1 > 0x0f;
        self.registers.pc += 1;
        self.delay += 1;
    }

    pub fn ld_ptr_hl_r(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
        let addr = self.registers.hl.read();
        self.mmu.write(addr, self.registers.get_r8(reg).read());
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn ld_ptr_ff00_u8_a(&mut self) {
        let offset = u16::from(self.mmu.read(self.registers.pc.read() + 1));
        let addr = 0xff00 | offset;
        log::trace!("ld_ff00_u8_a: writing {addr:x?}");
        self.mmu.write(addr, self.registers.af.high.read());
        self.registers.pc += 2;
        self.delay += 3;
    }

    pub fn ld_a_ptr_bc(&mut self) {
        let addr = self.registers.bc.read();
        self.registers.af.high.write(self.mmu.read(addr));
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn ld_a_ptr_de(&mut self) {
        let addr = self.registers.de.read();
        self.registers.af.high.write(self.mmu.read(addr));
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn call_u16(&mut self) {
        let addr_high = u16::from(self.mmu.read(self.registers.pc.read() + 2));
        let addr_low = u16::from(self.mmu.read(self.registers.pc.read() + 1));
        let addr = (addr_high << 8) | addr_low;
        self.registers.pc += 3;
        self.mmu
            .write(self.registers.sp.read() - 1, self.registers.pc.high.read());
        self.mmu
            .write(self.registers.sp.read() - 2, self.registers.pc.low.read());
        self.registers.pc.write(addr);
        self.registers.sp -= 2;
        self.delay += 6;
        log::trace!("call_u16: calling subroutine 0x{addr:x?}");
    }

    pub fn ld_r8_r8(&mut self, opcode: u8) {
        let dest = extract(opcode, 0b0011_1000);
        let src = extract(opcode, 0b0000_0111);

        let src_value = {
            let r = self.registers.get_r8(src);
            r.read()
        };
        let dest = self.registers.get_r8(dest);
        dest.write(src_value);

        self.registers.pc += 1;
        self.delay += 1;
    }

    pub fn push_r16(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0011_0000);
        let val = self.registers.get_r16_qq(reg).read();
        let high = ((val & 0xff00) >> 8) as u8;
        let low = (val & 0x00ff) as u8;
        self.mmu.write(self.registers.sp.read() - 1, high);
        self.mmu.write(self.registers.sp.read() - 2, low);
        self.registers.sp -= 2;
        self.registers.pc += 1;
        self.delay += 4;
    }

    pub fn rl_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
        let val = self.registers.get_r8(reg).read();
        let b7 = (val & 0b1000_0000) > 0;
        let val = {
            let val = val << 1;
            if self.registers.af.low.c {
                val | 0b0000_0001
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
    }

    pub fn rla(&mut self) {
        let val = self.registers.af.high.read();
        let b7 = (val & 0b1000_0000) > 0;
        let val = {
            let val = val << 1;
            if self.registers.af.low.c {
                val | 0b0000_0001
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
    }

    pub fn pop_r16(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0011_0000);
        let low = self.mmu.read(self.registers.sp.read());
        let high = self.mmu.read(self.registers.sp.read() + 1);
        let val = (u16::from(high) << 8) | u16::from(low);
        self.registers.get_r16_qq(reg).write(val);
        self.registers.sp += 2;
        self.registers.pc += 1;
        self.delay += 3;
    }

    pub fn dec_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0011_1000);
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
    }

    pub fn inc_r16(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0011_0000);
        let target = self.registers.get_r16_ss(reg);
        let val = target.read();
        let (new_val, _) = val.overflowing_add(1);
        target.write(new_val);
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn ret(&mut self) {
        let low = self.mmu.read(self.registers.sp.read());
        let high = self.mmu.read(self.registers.sp.read() + 1);
        self.registers.pc.high.write(high);
        self.registers.pc.low.write(low);
        self.registers.sp += 2;
        self.delay += 4;
    }

    pub fn cp_a_u8(&mut self) {
        let arg = self.mmu.read(self.registers.pc.read() + 1);
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
    }

    pub fn cp_a_ptr_hl(&mut self) {
        let arg = self.mmu.read(self.registers.hl.read());
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
    }

    pub fn ld_ptr_u16_a(&mut self) {
        let low = self.mmu.read(self.registers.pc.read() + 1);
        let high = self.mmu.read(self.registers.pc.read() + 2);
        let addr = (u16::from(high) << 8) | u16::from(low);
        self.mmu.write(addr, self.registers.af.high.read());
        self.registers.pc += 3;
        self.delay += 4;
    }

    #[allow(clippy::cast_possible_wrap)]
    pub fn jr_i8(&mut self) {
        let offset: u8 = self.mmu.read(self.registers.pc.read() + 1);
        let offset = i16::from(offset as i8);
        log::trace!("jr_i8: unconditional jump to {offset}");
        self.registers.pc += 2;

        let pc = self.registers.pc.read();
        let (pc, _) = pc.overflowing_add_signed(offset);
        self.registers.pc.write(pc);

        self.delay += 3;
    }

    pub fn ld_a_ptr_ff00_u8(&mut self) {
        let arg = u16::from(self.mmu.read(self.registers.pc.read() + 1));
        let addr = 0xff00 | arg;
        let val = self.mmu.read(addr);
        self.registers.af.high.write(val);

        self.registers.pc += 2;
        self.delay += 3;
    }

    pub fn sub_a_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
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
    }

    pub fn add_a_ptr_hl(&mut self) {
        let arg = self.mmu.read(self.registers.hl.read());
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
    }

    pub fn nop(&mut self) {
        self.registers.pc += 1;
        self.delay += 1;
    }

    pub fn jp_u16(&mut self) {
        let low = self.mmu.read(self.registers.pc.read() + 1);
        let high = self.mmu.read(self.registers.pc.read() + 2);
        let addr = (u16::from(high) << 8) | u16::from(low);
        self.registers.pc.write(addr);
        self.delay += 4;
    }

    pub fn ld_a_ptr_hli(&mut self) {
        let val = self.mmu.read(self.registers.hl.read());
        self.registers.af.high.write(val);
        self.registers.hl += 1;
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn ld_a_ptr_hld(&mut self) {
        let val = self.mmu.read(self.registers.hl.read());
        self.registers.af.high.write(val);
        self.registers.hl -= 1;
        self.registers.pc += 1;
        self.delay += 2;
    }
    pub fn ld_ptr_de_a(&mut self) {
        self.mmu
            .write(self.registers.de.read(), self.registers.af.high.read());
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn ld_ptr_bc_a(&mut self) {
        self.mmu
            .write(self.registers.bc.read(), self.registers.af.high.read());
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn di(&mut self) {
        log::debug!("DI");
        self.ime = false;
        self.registers.pc += 1;
        self.delay += 1;
    }

    pub fn ld_ptr_hl_u8(&mut self) {
        let arg = self.mmu.read(self.registers.pc.read() + 1);
        self.mmu.write(self.registers.hl.read(), arg);
        self.registers.pc += 2;
        self.delay += 3;
    }

    pub fn dec_r16(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0011_0000);
        let target = self.registers.get_r16_ss(reg);
        let val = target.read();
        let (val, _) = val.overflowing_sub(1);
        target.write(val);
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn or_a_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);

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
    }

    pub fn ei(&mut self) {
        log::debug!("EI");
        self.ime = true;
        self.registers.pc += 1;
        self.delay += 1;
    }

    pub fn cpl(&mut self) {
        let val = self.registers.af.high.read();
        self.registers.af.high.write(!val);
        self.registers.af.low.h = true;
        self.registers.af.low.n = true;
        self.registers.pc += 1;
        self.delay += 1;
    }
    pub fn and_a_u8(&mut self) {
        let arg = self.mmu.read(self.registers.pc.read() + 1);
        let a = self.registers.af.high.read();
        let val = a & arg;
        self.registers.af.high.write(val);
        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = true;
        self.registers.af.low.c = false;
        self.registers.pc += 2;
        self.delay += 2;
    }

    pub fn ld_a_ptr_u16(&mut self) {
        let low = self.mmu.read(self.registers.pc.read() + 1);
        let high = self.mmu.read(self.registers.pc.read() + 2);
        let addr = (u16::from(high) << 8) | u16::from(low);
        let val = self.mmu.read(addr);
        self.registers.af.high.write(val);
        self.registers.pc += 3;
        self.delay += 4;
    }

    pub fn call_cond_u16(&mut self, opcode: u8) {
        let code = extract(opcode, 0b0001_1000);
        let cond = self.registers.get_cond(code);

        if !cond {
            self.registers.pc += 3;
            self.delay += 3;
            return;
        }

        let addr_high = u16::from(self.mmu.read(self.registers.pc.read() + 2));
        let addr_low = u16::from(self.mmu.read(self.registers.pc.read() + 1));
        let addr = (addr_high << 8) | addr_low;
        self.registers.pc += 3;
        self.mmu
            .write(self.registers.sp.read() - 1, self.registers.pc.high.read());
        self.mmu
            .write(self.registers.sp.read() - 2, self.registers.pc.low.read());
        self.registers.pc.write(addr);
        self.registers.sp -= 2;
        self.delay += 6;
        log::trace!("call_cond_u16: calling subroutine 0x{addr:x?}");
    }

    pub fn add_a_u8(&mut self) {
        let arg = self.mmu.read(self.registers.pc.read() + 1);
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
    }

    pub fn sub_a_u8(&mut self) {
        let arg = self.mmu.read(self.registers.pc.read() + 1);
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
    }

    pub fn ld_r8_ptr_hl(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0011_1000);
        let val = self.mmu.read(self.registers.hl.read());
        let target = self.registers.get_r8(reg);
        target.write(val);
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn xor_a_ptr_hl(&mut self) {
        let a = self.registers.af.high.read();
        let target = self.mmu.read(self.registers.hl.read());
        let val = a ^ target;
        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn srl_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
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
    }

    pub fn rr_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
        let target = self.registers.get_r8(reg).read();
        let b0 = target & 0b1;
        let val = if self.registers.af.low.c {
            (target >> 1) | 0b1000_0000
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
    }

    pub fn rra(&mut self) {
        let target = self.registers.af.high.read();
        let b0 = target & 0b1;
        let val = if self.registers.af.low.c {
            (target >> 1) | 0b1000_0000
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
    }
    pub fn xor_a_u8(&mut self) {
        let arg = self.mmu.read(self.registers.pc.read() + 1);
        let a = self.registers.af.high.read();
        let val = a ^ arg;
        self.registers.af.high.write(val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 2;
        self.delay += 2;
    }
    pub fn adc_a_u8(&mut self) {
        let arg = self.mmu.read(self.registers.pc.read() + 1);
        let a = self.registers.af.high.read();
        let c = u8::from(self.registers.af.low.c);

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
    }

    pub fn ret_cond(&mut self, opcode: u8) {
        let code = extract(opcode, 0b0001_1000);
        let cond = self.registers.get_cond(code);
        if !cond {
            self.registers.pc += 1;
            self.delay += 2;
            return;
        }

        let low = self.mmu.read(self.registers.sp.read());
        let high = self.mmu.read(self.registers.sp.read() + 1);
        self.registers.pc.high.write(high);
        self.registers.pc.low.write(low);
        self.registers.sp += 2;
        self.delay += 5;
    }

    pub fn or_a_ptr_hl(&mut self) {
        let target = self.mmu.read(self.registers.hl.read());
        let a = self.registers.af.high.read();
        let val = target | a;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.af.high.write(val);
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn dec_ptr_hl(&mut self) {
        let target = self.mmu.read(self.registers.hl.read());
        let (val, _) = target.overflowing_sub(1);
        self.mmu.write(self.registers.hl.read(), val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = {
            let (v, _) = (target & 0x0f).overflowing_sub(1);
            v >= 0x10
        };
        self.registers.af.low.n = true;
        self.registers.pc += 1;
        self.delay += 3;
    }

    pub fn inc_ptr_hl(&mut self) {
        let target = self.mmu.read(self.registers.hl.read());
        let (val, _) = target.overflowing_add(1);
        self.mmu.write(self.registers.hl.read(), val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = {
            let (v, _) = (target & 0x0f).overflowing_add(1);
            v >= 0x10
        };
        self.registers.af.low.n = false;
        self.registers.pc += 1;
        self.delay += 3;
    }

    pub fn add_hl_rr(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0011_0000);
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
    }

    pub fn jp_hl(&mut self) {
        self.registers.pc.write(self.registers.hl.read());
        self.delay += 1;
    }

    pub fn swap_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
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
    }

    pub fn and_a_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
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
    }

    pub fn rst(&mut self, opcode: u8) {
        let n = extract(opcode, 0b0011_1000);
        let a = u16::from(n * 8);
        self.registers.pc += 1;
        self.mmu
            .write(self.registers.sp.read() - 1, self.registers.pc.high.read());
        self.mmu
            .write(self.registers.sp.read() - 2, self.registers.pc.low.read());
        self.registers.sp -= 2;
        self.registers.pc.write(a);
        self.delay += 4;
    }

    pub fn add_a_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
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
    }

    pub fn or_a_u8(&mut self) {
        let target = self.mmu.read(self.registers.pc.read() + 1);
        let a = self.registers.af.high.read();
        let val = target | a;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.af.high.write(val);
        self.registers.pc += 2;
        self.delay += 2;
    }

    pub fn set_b_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
        let bit = extract(opcode, 0b0011_1000);
        let target = self.registers.get_r8(reg);
        let val = target.read() | (1 << bit);
        target.write(val);
        self.registers.pc += 2;
        self.delay += 2;
    }

    pub fn set_b_ptr_hl(&mut self, opcode: u8) {
        let bit = extract(opcode, 0b0011_1000);
        let target = self.mmu.read(self.registers.hl.read());
        let val = target | (1 << bit);
        self.mmu.write(self.registers.hl.read(), val);
        self.registers.pc += 2;
        self.delay += 4;
    }

    pub fn ld_ptr_u16_sp(&mut self) {
        let low = self.mmu.read(self.registers.pc.read() + 1);
        let high = self.mmu.read(self.registers.pc.read() + 2);
        let addr = (u16::from(high) << 8) | u16::from(low);
        self.mmu.write(addr, self.registers.sp.low.read());
        self.mmu.write(addr + 1, self.registers.sp.high.read());
        self.registers.pc += 3;
        self.delay += 5;
    }

    pub fn ld_sp_hl(&mut self) {
        self.registers.sp.write(self.registers.hl.read());
        self.registers.pc += 1;
        self.delay += 2;
    }

    pub fn jp_cond_u16(&mut self, opcode: u8) {
        let low = self.mmu.read(self.registers.pc.read() + 1);
        let high = self.mmu.read(self.registers.pc.read() + 2);
        let addr = (u16::from(high) << 8) | u16::from(low);
        let code = extract(opcode, 0b0001_1000);
        let cond = self.registers.get_cond(code);
        if cond {
            self.registers.pc.write(addr);
            self.delay += 4;
        } else {
            self.registers.pc += 3;
            self.delay += 3;
        }
    }

    pub fn reti(&mut self) {
        let low = self.mmu.read(self.registers.sp.read());
        let high = self.mmu.read(self.registers.sp.read() + 1);
        self.registers.pc.high.write(high);
        self.registers.pc.low.write(low);
        self.registers.sp += 2;
        self.delay += 4;
        self.ime = true;
    }

    #[allow(clippy::cast_possible_wrap)]
    pub fn add_sp_i8(&mut self) {
        let offset: u8 = self.mmu.read(self.registers.pc.read() + 1);
        let offset = i16::from(offset as i8);
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
    }

    #[allow(clippy::cast_possible_wrap)]
    pub fn ld_hl_sp_i8(&mut self) {
        let offset: u8 = self.mmu.read(self.registers.pc.read() + 1);
        let offset = i16::from(offset as i8);
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
    }

    pub fn cp_a_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
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
    }

    pub fn sbc_a_u8(&mut self) {
        let arg = self.mmu.read(self.registers.pc.read() + 1);
        let a = self.registers.af.high.read();
        let c = u8::from(self.registers.af.low.c);

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
    }

    pub fn scf(&mut self) {
        self.registers.af.low.n = false;
        self.registers.af.low.h = false;
        self.registers.af.low.c = true;
        self.registers.pc += 1;
        self.delay += 1;
    }

    pub fn ccf(&mut self) {
        self.registers.af.low.n = false;
        self.registers.af.low.h = false;
        self.registers.af.low.c = !self.registers.af.low.c;
        self.registers.pc += 1;
        self.delay += 1;
    }
    pub fn adc_a_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
        let arg = self.registers.get_r8(reg).read();
        let a = self.registers.af.high.read();
        let c = u8::from(self.registers.af.low.c);

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
    }

    pub fn sbc_a_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
        let arg = self.registers.get_r8(reg).read();
        let a = self.registers.af.high.read();
        let c = u8::from(self.registers.af.low.c);

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
    }

    pub fn rlca(&mut self) {
        let val = self.registers.af.high.read();
        let c = (val & 0b1000_0000) > 0;
        let val = if c { (val << 1) | 1 } else { val << 1 };
        self.registers.af.low.c = c;
        self.registers.af.low.z = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.af.high.write(val);
        self.registers.pc += 1;
        self.delay += 1;
    }
    pub fn rrca(&mut self) {
        let val = self.registers.af.high.read();
        let c = (val & 0b0000_0001) > 0;
        let val = if c {
            (val >> 1) | 0b1000_0000
        } else {
            val >> 1
        };
        self.registers.af.low.c = c;
        self.registers.af.low.z = false;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.af.high.write(val);
        self.registers.pc += 1;
        self.delay += 1;
    }
    pub fn rlc_r8(&mut self, opcode: u8) {
        let code = extract(opcode, 0b0000_0111);
        let target = self.registers.get_r8(code);
        let val = target.read();
        let c = (val & 0b1000_0000) > 0;
        let val = if c { (val << 1) | 1 } else { val << 1 };
        target.write(val);
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 2;
    }

    pub fn rrc_r8(&mut self, opcode: u8) {
        let code = extract(opcode, 0b0000_0111);
        let target = self.registers.get_r8(code);
        let val = target.read();
        let c = (val & 0b0000_0001) > 0;
        let val = if c {
            (val >> 1) | 0b1000_0000
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
    }

    pub fn sla_r8(&mut self, opcode: u8) {
        let code = extract(opcode, 0b0000_0111);
        let target = self.registers.get_r8(code);
        let val = target.read();
        let c = (val & 0b1000_0000) > 0;
        let val = val << 1;
        target.write(val);
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 2;
    }

    pub fn sra_r8(&mut self, opcode: u8) {
        let code = extract(opcode, 0b000_0111);
        let target = self.registers.get_r8(code);
        let val = target.read();
        let c = (val & 0b0000_0001) > 0;
        let b7 = (val & 0b1000_0000) > 0;
        let val = if b7 {
            (val >> 1) | 0b1000_0000
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
    }

    pub fn res_b_r8(&mut self, opcode: u8) {
        let reg = extract(opcode, 0b0000_0111);
        let bit = extract(opcode, 0b0011_1000);
        let target = self.registers.get_r8(reg);
        let val = target.read() & !(1 << bit);
        target.write(val);
        self.registers.pc += 2;
        self.delay += 2;
    }

    pub fn ld_a_ptr_ff00_c(&mut self) {
        let c = u16::from(self.registers.bc.low.read());
        let addr = 0xff00 + c;
        let val = self.mmu.read(addr);
        self.registers.af.high.write(val);
        self.registers.pc += 1;
        self.delay += 2;
    }
    pub fn adc_a_ptr_hl(&mut self) {
        let arg = self.mmu.read(self.registers.hl.read());
        let a = self.registers.af.high.read();
        let c = u8::from(self.registers.af.low.c);

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
    }

    pub fn sub_a_ptr_hl(&mut self) {
        let arg = self.mmu.read(self.registers.hl.read());
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
    }
    pub fn sbc_a_ptr_hl(&mut self) {
        let arg = self.mmu.read(self.registers.hl.read());
        let a = self.registers.af.high.read();
        let c = u8::from(self.registers.af.low.c);

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
    }
    pub fn and_a_ptr_hl(&mut self) {
        let arg = self.mmu.read(self.registers.hl.read());
        let a = self.registers.af.high.read();
        let val = a & arg;
        self.registers.af.high.write(val);
        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = true;
        self.registers.af.low.c = false;
        self.registers.pc += 1;
        self.delay += 2;
    }
    pub fn rlc_ptr_hl(&mut self) {
        let val = self.mmu.read(self.registers.hl.read());
        let c = (val & 0b1000_0000) > 0;
        let val = if c { (val << 1) | 1 } else { val << 1 };
        self.mmu.write(self.registers.hl.read(), val);

        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 4;
    }

    pub fn rrc_ptr_hl(&mut self) {
        let val = self.mmu.read(self.registers.hl.read());
        let c = (val & 0b0000_0001) > 0;

        let val = if c {
            (val >> 1) | 0b1000_0000
        } else {
            val >> 1
        };
        self.mmu.write(self.registers.hl.read(), val);
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 4;
    }

    pub fn rl_ptr_hl(&mut self) {
        let val = self.mmu.read(self.registers.hl.read());

        let b7 = (val & 0b1000_0000) > 0;
        let val = {
            let val = val << 1;
            if self.registers.af.low.c {
                val | 0b0000_0001
            } else {
                val
            }
        };
        self.mmu.write(self.registers.hl.read(), val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = false;
        self.registers.af.low.c = b7;

        self.registers.pc += 2;
        self.delay += 4;
    }

    pub fn rr_ptr_hl(&mut self) {
        let target = self.mmu.read(self.registers.hl.read());
        let b0 = target & 0b1;
        let val = if self.registers.af.low.c {
            (target >> 1) | 0b1000_0000
        } else {
            target >> 1
        };
        self.mmu.write(self.registers.hl.read(), val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = b0 > 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 2;
        self.delay += 4;
    }

    pub fn sla_ptr_hl(&mut self) {
        let val = self.mmu.read(self.registers.hl.read());
        let c = (val & 0b1000_0000) > 0;
        let val = val << 1;
        self.mmu.write(self.registers.hl.read(), val);
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 4;
    }

    pub fn sra_ptr_hl(&mut self) {
        let val = self.mmu.read(self.registers.hl.read());
        let c = (val & 0b0000_0001) > 0;
        let b7 = (val & 0b1000_0000) > 0;
        let val = if b7 {
            (val >> 1) | 0b1000_0000
        } else {
            val >> 1
        };
        self.mmu.write(self.registers.hl.read(), val);
        self.registers.af.low.c = c;
        self.registers.af.low.z = val == 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;
        self.registers.pc += 2;
        self.delay += 4;
    }

    pub fn swap_ptr_hl(&mut self) {
        let r = self.mmu.read(self.registers.hl.read());
        let high = extract(r, 0b1111_0000);
        let low = extract(r, 0b0000_1111);
        let val = (low << 4) | high;

        self.mmu.write(self.registers.hl.read(), val);
        self.registers.af.low.z = val == 0;
        self.registers.af.low.n = false;
        self.registers.af.low.h = false;
        self.registers.af.low.c = false;

        self.registers.pc += 2;
        self.delay += 4;
    }

    pub fn srl_ptr_hl(&mut self) {
        let r = self.mmu.read(self.registers.hl.read());
        let b0 = r & 0b1;
        let val = r >> 1;
        self.mmu.write(self.registers.hl.read(), val);

        self.registers.af.low.z = val == 0;
        self.registers.af.low.c = b0 > 0;
        self.registers.af.low.h = false;
        self.registers.af.low.n = false;

        self.registers.pc += 2;
        self.delay += 4;
    }

    pub fn bit_b_ptr_hl(&mut self, opcode: u8) {
        let bit = extract(opcode, 0b0011_1000);
        let target = self.mmu.read(self.registers.hl.read());
        self.registers.af.low.z = (target & (1 << bit)) == 0;
        self.registers.af.low.h = true;
        self.registers.af.low.n = false;
        self.delay += 3;
        self.registers.pc += 2;
    }

    pub fn res_b_ptr_hl(&mut self, opcode: u8) {
        let bit = extract(opcode, 0b0011_1000);
        let target = self.mmu.read(self.registers.hl.read());
        let val = target & !(1 << bit);
        self.mmu.write(self.registers.hl.read(), val);
        self.registers.pc += 2;
        self.delay += 4;
    }

    pub fn daa(&mut self) {
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
    }

    pub fn halt(&mut self) {
        log::debug!("halt!");
        self.halted = true;
        self.registers.pc += 1;
        self.delay += 1;
    }
}
