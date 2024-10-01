use log::{debug, info};
use crate::disassembler::Disassembler;
use crate::{Flags, Register, RegisterPair, RegisterPairMem, RegisterPairStk};
use crate::arithmetic::{op_adc, op_add, op_add16, op_and, op_bit, op_cp, op_dec, op_dec16, op_inc, op_inc16, op_or, op_res, op_rl, op_rlc, op_rr, op_rrc, op_sbc, op_set, op_sla, op_sra, op_srl, op_sub, op_swap, op_xor};
use crate::isa::{ArithmeticInstruction, BitInstruction, Condition, Instruction, JumpInstruction, LoadInstruction, MiscInstruction, StackInstruction};
use crate::memory::{Interrupt, Memory, RegisterPairValue};
use crate::timer::Timer;

pub struct Cpu {
    af: RegisterPairValue,
    bc: RegisterPairValue,
    de: RegisterPairValue,
    hl: RegisterPairValue,
    sp: RegisterPairValue,
    pc: RegisterPairValue,
    mem: Memory,
    disassembler: Disassembler,
    ime: bool,  // interrupt master enable
    timer: Timer,
}

impl Cpu {
    pub fn new(boot_rom: Vec<u8>, rom: Vec<u8>) -> Self {
        let mem = Memory::new(&boot_rom, &rom[0..0x3fff]);
        Self {
            af: RegisterPairValue::from(Flags::ZERO.bits() as u16),
            bc: RegisterPairValue::from(0x0013),
            de: RegisterPairValue::from(0x00D8),
            hl: RegisterPairValue::from(0x014D),
            sp: RegisterPairValue::from(0xFFFE),
            pc: RegisterPairValue::from(0x0000),
            mem,
            disassembler: Disassembler::new(),
            ime: true,
            timer: Timer::new(),
        }
    }

    pub fn register(&self, reg_id: Register) -> u8 {
        match reg_id {
            Register::A => self.af.high(),
            Register::B => self.bc.high(),
            Register::C => self.bc.low(),
            Register::D => self.de.high(),
            Register::E => self.de.low(),
            Register::H => self.hl.high(),
            Register::L => self.hl.low(),
        }
    }

    pub fn register_mut(&mut self, reg_id: Register) -> &mut u8 {
        match reg_id {
            Register::A => self.af.high_mut(),
            Register::B => self.bc.high_mut(),
            Register::C => self.bc.low_mut(),
            Register::D => self.de.high_mut(),
            Register::E => self.de.low_mut(),
            Register::H => self.hl.high_mut(),
            Register::L => self.hl.low_mut(),
        }
    }

    pub fn register_pair(&self, reg_pair_id: RegisterPair) -> u16 {
        match reg_pair_id {
            RegisterPair::BC => self.bc.as_u16(),
            RegisterPair::DE => self.de.as_u16(),
            RegisterPair::HL => self.hl.as_u16(),
            RegisterPair::SP => self.sp.as_u16(),
        }
    }

    pub fn register_pair_mem(&mut self, reg_pair_id: RegisterPairMem) -> u16 {
        match reg_pair_id {
            RegisterPairMem::BC => self.bc.as_u16(),
            RegisterPairMem::DE => self.de.as_u16(),
            RegisterPairMem::HLI => { 
                let res = self.hl.as_u16();
                self.hl = RegisterPairValue::from(res.wrapping_add(1));
                res
            },
            RegisterPairMem::HLD => {
                let res = self.hl.as_u16();
                self.hl = RegisterPairValue::from(res.wrapping_sub(1));
                res
            },
        }
    }

    pub fn register_pair_stk(&self, reg_pair_id: RegisterPairStk) -> u16 {
        match reg_pair_id {
            RegisterPairStk::BC => self.bc.as_u16(),
            RegisterPairStk::DE => self.de.as_u16(),
            RegisterPairStk::HL => self.hl.as_u16(),
            RegisterPairStk::AF => self.sp.as_u16(),
        }
    }

    pub fn register_pair_mut(&mut self, reg_pair_id: RegisterPair) -> &mut RegisterPairValue {
        match reg_pair_id {
            RegisterPair::BC => &mut self.bc,
            RegisterPair::DE => &mut self.de,
            RegisterPair::HL => &mut self.hl,
            RegisterPair::SP => &mut self.sp,
        }
    }
    
    pub fn register_pair_stk_mut(&mut self, reg_pair_id: RegisterPairStk) -> &mut RegisterPairValue {
        match reg_pair_id {
            RegisterPairStk::BC => &mut self.bc,
            RegisterPairStk::DE => &mut self.de,
            RegisterPairStk::HL => &mut self.hl,
            RegisterPairStk::AF => &mut self.sp,
        }
    }
    const VBLANK_RATE: f32 = 59.73;  // Hz

    pub fn run(&mut self) {
        self.mem.enable_interrupt(Interrupt::VBlank, true);
        let mut instant = std::time::Instant::now();
        loop {
            let elapsed = instant.elapsed();
            if elapsed.as_secs_f32() >= 1.0 / Self::VBLANK_RATE {
                instant = std::time::Instant::now();
                self.vblank();
            }
            // println!("PC: {:#06X}", self.pc.as_u16());
            let (instruction, new_pc) = self.disassembler.disassemble(&self.mem, self.pc.as_u16());
            self.pc = RegisterPairValue::from(new_pc);
            match instruction {
                Instruction::Arithmetic(x) => self.eval_arithmetic(x),
                Instruction::Bit(x) => self.eval_bit(x),
                Instruction::Load(x) => self.eval_load(x),
                Instruction::Jump(x) => self.eval_jump(x),
                Instruction::Stack(x) => self.eval_stack(x),
                Instruction::Misc(x) => self.eval_misc(x),
            }
            if self.ime && self.mem.requested_interrupts() != 0 {
                self.handle_interrupt();
            }
            self.timer.cycle(&mut self.mem);
        }
    }
    
    fn vblank(&mut self) {
        debug!("=== VBlank ===");
        // request interrupt
        self.mem.request_interrupt(Interrupt::VBlank);
        // draw screen
        
    }
    
    fn handle_interrupt(&mut self) {
        debug!("Handling interrupt");
        let requested = self.mem.requested_interrupts();
        debug!("Requested interrupts: {:#08b}", requested);
        let enabled = self.mem.enabled_interrupts();
        debug!("Enabled interrupts: {:#08b}", enabled);
        let interrupt = requested & enabled; // todo priority?
        
        match Interrupt::from(interrupt) {
            Interrupt::VBlank => {
                debug!("Handling VBlank interrupt");
                self.mem.clear_requested_interrupt(Interrupt::VBlank);
                self.ime = false;
                self.push(self.pc.high());
                self.push(self.pc.low());
                self.pc = RegisterPairValue::from(0x0040);
            }
            Interrupt::LcdStat => {}
            Interrupt::Timer => {}
            Interrupt::Serial => {}
            Interrupt::Joypad => {}
        }
    }

    fn eval_arithmetic(&mut self, instruction: ArithmeticInstruction) {
        let mut flags = self.af.flags();
        match instruction {
            ArithmeticInstruction::AdcAR8(reg) => {
                let a = self.af.high();
                let b = self.register(reg);
                self.af.set_high(op_adc(a, b, &mut flags));
            }
            ArithmeticInstruction::AdcAMemHL => {
                let a = self.af.high();
                let b = self.mem.get(self.hl.as_u16());
                self.af.set_high(op_adc(a, b, &mut flags));
            }
            ArithmeticInstruction::AdcAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_adc(a, b, &mut flags));
            }
            ArithmeticInstruction::AddAR8(reg) => {
                let a = self.af.high();
                let b = self.register(reg);
                self.af.set_high(op_add(a, b, &mut flags));
            }
            ArithmeticInstruction::AddAMemHL => {
                let a = self.af.high();
                let b = self.mem.get(self.hl.as_u16());
                self.af.set_high(op_add(a, b, &mut flags));
            }
            ArithmeticInstruction::AddAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_add(a, b, &mut flags));
            }
            ArithmeticInstruction::AndAR8(reg) => {
                let a = self.af.high();
                let b = self.register(reg);
                self.af.set_high(op_and(a, b, &mut flags));
            }
            ArithmeticInstruction::AndAMemHL => {
                let a = self.af.high();
                let b = self.mem.get(self.hl.as_u16());
                self.af.set_high(op_and(a, b, &mut flags));
            }
            ArithmeticInstruction::AndAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_and(a, b, &mut flags));
            }
            ArithmeticInstruction::CpAR8(reg) => {
                let a = self.af.high();
                let b = self.register(reg);
                op_cp(a, b, &mut flags);
            }
            ArithmeticInstruction::CpAMemHL => {
                let a = self.af.high();
                let b = self.mem.get(self.hl.as_u16());
                op_cp(a, b, &mut flags);
            }
            ArithmeticInstruction::CpAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                op_cp(a, b, &mut flags);
            }
            ArithmeticInstruction::DecR8(reg) => {
                let a = self.register(reg);
                *self.register_mut(reg) = op_dec(a, &mut flags);
            }
            ArithmeticInstruction::DecMemHL => {
                let a = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_dec(a, &mut flags));
            }
            ArithmeticInstruction::IncR8(reg) => {
                let a = self.register(reg);
                *self.register_mut(reg) = op_inc(a, &mut flags);
            }
            ArithmeticInstruction::IncMemHL => {
                let a = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_inc(a, &mut flags));
            }
            ArithmeticInstruction::OrAR8(reg) => {
                let a = self.af.high();
                let b = self.register(reg);
                self.af.set_high(op_or(a, b, &mut flags));
            }
            ArithmeticInstruction::OrAMemHL => {
                let a = self.af.high();
                let b = self.mem.get(self.hl.as_u16());
                self.af.set_high(op_or(a, b, &mut flags));
            }
            ArithmeticInstruction::OrAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_or(a, b, &mut flags));
            }
            ArithmeticInstruction::SbcAR8(reg) => {
                let a = self.af.high();
                let b = self.register(reg);
                self.af.set_high(op_sbc(a, b, &mut flags));
            }
            ArithmeticInstruction::SbcAMemHL => {
                let a = self.af.high();
                let b = self.mem.get(self.hl.as_u16());
                self.af.set_high(op_sbc(a, b, &mut flags));
            }
            ArithmeticInstruction::SbcAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_sbc(a, b, &mut flags));
            }
            ArithmeticInstruction::SubAR8(reg) => {
                let a = self.af.high();
                let b = self.register(reg);
                self.af.set_high(op_sub(a, b, &mut flags));
            }
            ArithmeticInstruction::SubAMemHL => {
                let a = self.af.high();
                let b = self.mem.get(self.hl.as_u16());
                self.af.set_high(op_sub(a, b, &mut flags));
            }
            ArithmeticInstruction::SubAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_sub(a, b, &mut flags));
            }
            ArithmeticInstruction::XorAR8(reg) => {
                let a = self.af.high();
                let b = self.register(reg);
                self.af.set_high(op_xor(a, b, &mut flags));
            }
            ArithmeticInstruction::XorAMemHL => {
                let a = self.af.high();
                let b = self.mem.get(self.hl.as_u16());
                self.af.set_high(op_xor(a, b, &mut flags));
            }
            ArithmeticInstruction::XorAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_xor(a, b, &mut flags));
            }
            ArithmeticInstruction::AddHLR16(reg) => {
                let a = self.hl.as_u16();
                let b = self.register_pair(reg);
                self.hl = RegisterPairValue::from(op_add16(a, b, &mut flags));
            }
            ArithmeticInstruction::DecR16(reg) => {
                let a = self.register_pair(reg);
                *self.register_pair_mut(reg) = RegisterPairValue::from(op_dec16(a));
            }
            ArithmeticInstruction::IncR16(reg) => {
                let a = self.register_pair(reg);
                *self.register_pair_mut(reg) = RegisterPairValue::from(op_inc16(a));
            }
        }
        self.af.set_low(flags.bits());
    }

    fn eval_bit(&mut self, instruction: BitInstruction) {
        let mut flags = self.af.flags();
        match instruction {
            BitInstruction::Bit(a, reg) => {
                op_bit(a, self.register(reg), &mut flags);
            }
            BitInstruction::BitMemHL(a) => {
                op_bit(a, self.mem.get(self.hl.as_u16()), &mut flags);
            }
            BitInstruction::Res(a, reg) => {
                *self.register_mut(reg) = op_res(a, self.register(reg));
            }
            BitInstruction::ResMemHL(a) => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_res(a, prev));
            }
            BitInstruction::Set(a, reg) => {
                *self.register_mut(reg) = op_set(a, self.register(reg));
            }
            BitInstruction::SetMemHL(a) => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_set(a, prev));
            }
            BitInstruction::Swap(reg) => {
                *self.register_mut(reg) = op_swap(self.register(reg), &mut flags);
            }
            BitInstruction::SwapMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_swap(prev, &mut flags));
            }
            BitInstruction::Rl(reg) => {
                *self.register_mut(reg) = op_rl(self.register(reg), &mut flags);
            }
            BitInstruction::RlMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_rl(prev, &mut flags));
            }
            BitInstruction::Rla => {
                *self.register_mut(Register::A) = op_rl(self.register(Register::A), &mut flags);
            }
            BitInstruction::Rlc(reg) => {
                *self.register_mut(reg) = op_rlc(self.register(reg), &mut flags);
            }
            BitInstruction::RlcMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_rlc(prev, &mut flags));
            }
            BitInstruction::Rlca => {
                // TODO: modify zero flag? 
                *self.register_mut(Register::A) = op_rlc(self.register(Register::A), &mut flags);
            }
            BitInstruction::Rr(reg) => {
                *self.register_mut(reg) = op_rr(self.register(reg), &mut flags);
            }
            BitInstruction::RrMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_rr(prev, &mut flags));
            }
            BitInstruction::Rra => {
                *self.register_mut(Register::A) = op_rr(self.register(Register::A), &mut flags);
            }
            BitInstruction::Rrc(reg) => {
                *self.register_mut(reg) = op_rrc(self.register(reg), &mut flags);
            }
            BitInstruction::RrcMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_rrc(prev, &mut flags));
            }
            BitInstruction::Rrca => {
                *self.register_mut(Register::A) = op_rrc(self.register(Register::A), &mut flags);
            }
            BitInstruction::Sla(reg) => {
                *self.register_mut(reg) = op_sla(self.register(reg), &mut flags);
            }
            BitInstruction::SlaMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_sla(prev, &mut flags));
            }
            BitInstruction::Sra(reg) => {
                *self.register_mut(reg) = op_sra(self.register(reg), &mut flags);
            }
            BitInstruction::SraMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_sra(prev, &mut flags));
            }
            BitInstruction::Srl(reg) => {
                *self.register_mut(reg) = op_srl(self.register(reg), &mut flags);
            }
            BitInstruction::SrlMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_srl(prev, &mut flags));
            }
        }
        self.af.set_low(flags.bits());
    }

    fn eval_load(&mut self, instruction: LoadInstruction) {
        match instruction {
            LoadInstruction::LdR8R8(reg1, reg2) => {
                *self.register_mut(reg1) = self.register(reg2);
            }
            LoadInstruction::LdR8N8(reg, imm) => {
                *self.register_mut(reg) = imm;
            }
            LoadInstruction::LdR16N16(reg, imm) => {
                *self.register_pair_mut(reg) = RegisterPairValue::from(imm);
            }
            LoadInstruction::LdMemHLR8(reg) => {
                let val = self.register(reg);
                self.mem.update(self.hl.as_u16(), || val);
            }
            LoadInstruction::LdMemHLN8(imm) => {
                self.mem.update(self.hl.as_u16(), || imm);
            }
            LoadInstruction::LdR8MemHL(reg) => {
                *self.register_mut(reg) = self.mem.get(self.hl.as_u16());
            }
            LoadInstruction::LdMemR16A(reg) => {
                let addr = self.register_pair_mem(reg);
                self.mem.update(addr, || self.af.high());
            }
            LoadInstruction::LdMemN16A(addr) => {
                self.mem.update(addr, || self.af.high());
            }
            LoadInstruction::LdhMemN16A(addr) => {
                self.mem.update(0xFF00 + addr, || self.af.high());
            }
            LoadInstruction::LdhMemCA => {
                self.mem.update(0xFF00 + self.bc.low() as u16, || self.af.high());
            }
            LoadInstruction::LdAMemR16(reg) => {
                let addr = self.register_pair_mem(reg);
                self.af.set_high(self.mem.get(addr));
            }
            LoadInstruction::LdAMemN16(addr) => {
                self.af.set_high(self.mem.get(addr));
            }
            LoadInstruction::LdhAMemN16(addr) => {
                self.af.set_high(self.mem.get(0xFF00 + addr));
            }
            LoadInstruction::LdhAMemC => {
                self.af.set_high(self.mem.get(0xFF00 + self.bc.low() as u16));
            }
            LoadInstruction::LdMemHLIA => {
                self.mem.update(self.hl.as_u16(), || self.af.high());
                self.hl = RegisterPairValue::from(self.hl.as_u16().wrapping_add(1));
            }
            LoadInstruction::LdMemHLDA => {
                self.mem.update(self.hl.as_u16(), || self.af.high());
                self.hl = RegisterPairValue::from(self.hl.as_u16().wrapping_sub(1));
            }
            LoadInstruction::LdAMemHLI => {
                self.af.set_high(self.mem.get(self.hl.as_u16()));
                self.hl = RegisterPairValue::from(self.hl.as_u16().wrapping_add(1));
            }
            LoadInstruction::LdAMemHLD => {
                self.af.set_high(self.mem.get(self.hl.as_u16()));
                self.hl = RegisterPairValue::from(self.hl.as_u16().wrapping_sub(1));
            }
            LoadInstruction::LdhAMemN8(addr) => {
                let val = self.mem.get(0xFF00 + addr as u16);
                println!("Loading value {:#04X} from address {:#06X}", val, 0xFF00 + addr as u16);
                self.af.set_high(val);
            }
            LoadInstruction::LdhMemN8A(addr) => {
                self.mem.update(0xFF00 + addr as u16, || self.af.high());
            }
        }
    }

    fn eval_cond(&self, condition: Condition) -> bool{
        match condition {
            Condition::NotZero => !self.af.flags().contains(Flags::ZERO),
            Condition::Zero => self.af.flags().contains(Flags::ZERO),
            Condition::NotCarry => !self.af.flags().contains(Flags::CARRY),
            Condition::Carry => self.af.flags().contains(Flags::CARRY),
        }
    }

    fn push(&mut self, value: u8) {
        self.mem.update(self.sp.as_u16(), || value);
        self.sp = RegisterPairValue::from(self.sp.as_u16().wrapping_sub(1));
    }

    fn pop(&mut self) -> Option<u8> {
        self.sp = RegisterPairValue::from(self.sp.as_u16().wrapping_add(1));
        Some(self.mem.get(self.sp.as_u16()))
    }

    fn eval_jump(&mut self, instruction: JumpInstruction) {
        match instruction {
            JumpInstruction::CallN16(imm) => {
                self.push(self.pc.high());
                self.push(self.pc.low());
                self.pc = RegisterPairValue::from(imm);
            }
            JumpInstruction::CallCCN16(cond, imm) => {
                if self.eval_cond(cond) {
                    self.push(self.pc.high());
                    self.push(self.pc.low());
                    self.pc = RegisterPairValue::from(imm);
                }
            }
            JumpInstruction::JpHL => {
                self.pc = self.hl;
            }
            JumpInstruction::JpN16(imm) => {
                self.pc = RegisterPairValue::from(imm);
            }
            JumpInstruction::JpCCN16(cond, imm) => {
                if self.eval_cond(cond) {
                    self.pc = RegisterPairValue::from(imm);
                }
            }
            JumpInstruction::JrN8(imm) => {
                self.pc = RegisterPairValue::from(self.pc.as_u16().wrapping_add(imm as u16));
            }
            JumpInstruction::JrCCN8(cond, imm) => {
                if self.eval_cond(cond) {
                    self.pc = RegisterPairValue::from(self.pc.as_u16().wrapping_add(imm as u16));
                }
            }
            JumpInstruction::RetCC(cond) => {
                if self.eval_cond(cond) {
                    let lo = self.pop().unwrap();
                    let hi = self.pop().unwrap();
                    self.pc = RegisterPairValue::from((hi as u16) << 8 | lo as u16);
                }
            }
            JumpInstruction::Ret => {
                let lo = self.pop().unwrap();
                let hi = self.pop().unwrap();
                self.pc = RegisterPairValue::from((hi as u16) << 8 | lo as u16);
            }
            JumpInstruction::Reti => {
                let lo = self.pop().unwrap();
                let hi = self.pop().unwrap();
                self.pc = RegisterPairValue::from((hi as u16) << 8 | lo as u16);
                self.ime = true;
            }
            JumpInstruction::Rst(vec) => {
                self.push(self.pc.high());
                self.push(self.pc.low());
                self.pc = RegisterPairValue::from(vec as u16);
            }
        }
    }

    fn eval_stack(&mut self, instruction: StackInstruction) {
        match instruction {
            StackInstruction::AddHLSP => {
                self.hl = RegisterPairValue::from(self.hl.as_u16().wrapping_add(self.sp.as_u16()));
            }
            StackInstruction::AddSPE8(imm) => {
                self.sp = RegisterPairValue::from(self.sp.as_u16().wrapping_add_signed(imm as i16));
            }
            StackInstruction::DecSP => {
                self.sp = RegisterPairValue::from(self.sp.as_u16().wrapping_sub(1));
            }
            StackInstruction::IncSP => {
                self.sp = RegisterPairValue::from(self.sp.as_u16().wrapping_add(1));
            }
            StackInstruction::LdSPN16(imm) => {
                self.sp = RegisterPairValue::from(imm);
            }
            StackInstruction::LdMemN16SP(imm) => {
                let addr = imm;
                self.mem.update(addr, || self.sp.low());
                self.mem.update(addr + 1, || self.sp.high());
            }
            StackInstruction::LdHLSPPlusE8(imm) => {
                self.hl = RegisterPairValue::from(self.sp.as_u16().wrapping_add_signed(imm as i16));
            }
            StackInstruction::LdSPHL => {
                self.sp = self.hl;
            }
            StackInstruction::PopAF => {
                let hi = self.pop().unwrap();
                let lo = self.pop().unwrap();
                self.af = RegisterPairValue::from((hi as u16) << 8 | lo as u16);
            }
            StackInstruction::PopR16(reg) => {
                let hi = self.pop().unwrap();
                let lo = self.pop().unwrap();
                *self.register_pair_stk_mut(reg) = RegisterPairValue::from((hi as u16) << 8 | lo as u16);
            }
            StackInstruction::PushAF => {
                self.push(self.af.low());
                self.push(self.af.high());
            }
            StackInstruction::PushR16(reg) => {
                match reg {
                    RegisterPairStk::BC => {
                        self.push(self.bc.low());
                        self.push(self.bc.high());
                    }
                    RegisterPairStk::DE => {
                        self.push(self.de.low());
                        self.push(self.de.high());
                    }
                    RegisterPairStk::HL => {
                        self.push(self.hl.low());
                        self.push(self.hl.high());
                    }
                    RegisterPairStk::AF => {
                        self.push(self.af.low());
                        self.push(self.af.high());
                    }
                }
            }
        }
    }

    fn eval_misc(&mut self, instruction: MiscInstruction) {
        match instruction {
            MiscInstruction::Ccf => {todo!()}
            MiscInstruction::Cpl => {todo!()}
            MiscInstruction::DaA => {todo!()}
            MiscInstruction::Di => {
                unreachable!();
                self.ime = false;
                info!("Disabling interrupts...")
            }
            MiscInstruction::Ei => {
                self.ime = true;
                info!("Enabling interrupts...")
            }
            MiscInstruction::Halt => {todo!()}
            MiscInstruction::Nop => {}
            MiscInstruction::Scf => {todo!()}
            MiscInstruction::Stop => {
                info!("Stopping CPU...")
            }
        }
    }
}
