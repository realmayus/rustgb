use crate::{Flags, RegisterPair, RegisterPairMem, RegisterPairStk};
use crate::FrameData;
use crate::ControlMsg;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;
use eframe::egui::debug_text::print;
use log::{debug, info};
use crate::disassembler::Disassembler;
use crate::arithmetic::{op_adc, op_add, op_add16, op_and, op_bit, op_cp, op_dec, op_dec16, op_inc, op_inc16, op_or, op_res, op_rl, op_rlc, op_rr, op_rrc, op_sbc, op_set, op_sla, op_sra, op_srl, op_sub, op_swap, op_xor};
use crate::isa::{ArithmeticInstruction, BitInstruction, Condition, Instruction, JumpInstruction, LoadInstruction, MiscInstruction, StackInstruction};
use crate::memory::{Interrupt, Mbc, MappedMemory, RegisterPairValue, Memory};
use crate::ppu::Ppu;
use crate::Register;
use crate::timer::Timer;

pub struct Cpu<M: Memory> {
    af: RegisterPairValue,
    bc: RegisterPairValue,
    de: RegisterPairValue,
    hl: RegisterPairValue,
    pub sp: RegisterPairValue,
    pub pc: RegisterPairValue,
    pub mem: M,
    disassembler: Disassembler,
    pub ime: bool,  // interrupt master enable
    stall: usize,
    pub(crate) last_cycle: Instant,
    pub recv: Receiver<ControlMsg>,
    halted: bool,
    terminate: bool,
    di_ctr: u8, // delay di instruction
    ei_ctr: u8, // delay ei instruction
}

impl<M> Cpu<M> where M: Memory {
    pub fn new(mem: M, recv: Receiver<ControlMsg>) -> Self {
        Self {
            af: RegisterPairValue::from(0x0100 | Flags::ZERO.bits() as u16),
            bc: RegisterPairValue::from(0x0013),
            de: RegisterPairValue::from(0x00D8),
            hl: RegisterPairValue::from(0x014D),
            sp: RegisterPairValue::from(0xFFFE),
            pc: RegisterPairValue::from(0x0100),
            mem,
            disassembler: Disassembler::new(),
            ime: false,
            stall: 0,
            last_cycle: Instant::now(),
            recv,
            halted: false,
            terminate: false,
            di_ctr: 0,
            ei_ctr: 0,
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
            RegisterPairStk::AF => self.af.as_u16(),
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
            RegisterPairStk::AF => &mut self.af,
        }
    }
    
    pub fn run(&mut self) {
        let frame_time = 16.74 / 1000.0; // s
        let cycles_per_frame = 70224 / 4;
        while !self.terminate {
            puffin::profile_scope!("Cpu::cycle");
            puffin::GlobalProfiler::lock().new_frame();
            
            
           
            let before_frame = Instant::now();
            for _ in 0..cycles_per_frame {
                if let Ok(msg) = self.recv.try_recv() {
                    self.control_message(msg);
                }
                self.cycle();
            }
            let elapsed = before_frame.elapsed().as_secs_f64() * 1000.0;
            
            if elapsed < frame_time {
                // print!("delaying next cycle by {} ms", (cycle_time - elapsed) * 1000.0);
                std::thread::sleep(std::time::Duration::from_secs_f64(frame_time - elapsed));
            }
        }
    }
    
    pub fn cycle(&mut self) {
        puffin::profile_function!();
        self.last_cycle = Instant::now();
        if self.di_ctr == 1 {
            self.ime = false;
        }
        if self.ei_ctr == 1 {
            self.ime = true;
        }
        self.di_ctr = self.di_ctr.saturating_sub(1);
        self.ei_ctr = self.ei_ctr.saturating_sub(1);
        
        if self.stall > 0 {
            self.stall -= 1;
        } else if !self.halted {
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
        }
        self.handle_interrupt();

        
        self.mem.cycle();
    }
    
    fn handle_interrupt(&mut self) {
        if !self.ime && !self.halted {
            return;
        }
        let triggered = self.mem.enabled_interrupts() & self.mem.requested_interrupts();
        if triggered == 0 {
            return;
        }
        self.halted = false;
        if !self.ime {
            return;
        }
        let requested = self.mem.requested_interrupts();
        let enabled = self.mem.enabled_interrupts();
        let interrupt = Interrupt::from(requested & enabled); // todo priority?

        self.ime = false;
        self.push(self.pc.as_u16());
        match interrupt {
            Interrupt::VBlank => {
                self.mem.clear_requested_interrupt(Interrupt::VBlank);
                self.pc = RegisterPairValue::from(0x0040);
            }
            Interrupt::LcdStat => {
                debug!("Requested interrupts: {:#08b}, enabled: {:#08b}", requested, enabled);
                debug!("Handling LCD Stat interrupt");
                self.mem.clear_requested_interrupt(Interrupt::LcdStat);
                self.pc = RegisterPairValue::from(0x0048);
            }
            Interrupt::Timer => {
                debug!("Requested interrupts: {:#08b}, enabled: {:#08b}", requested, enabled);
                debug!("Handling Timer interrupt");
                self.mem.clear_requested_interrupt(Interrupt::Timer);
                self.pc = RegisterPairValue::from(0x0050);
            }
            Interrupt::Serial => {
                debug!("Requested interrupts: {:#08b}, enabled: {:#08b}", requested, enabled);
                debug!("Handling Serial interrupt");
                self.mem.clear_requested_interrupt(Interrupt::Serial);
                self.pc = RegisterPairValue::from(0x0058);
            }
            Interrupt::Joypad => {
                debug!("Requested interrupts: {:#08b}, enabled: {:#08b}", requested, enabled);
                debug!("Handling Joypad interrupt");
                self.mem.clear_requested_interrupt(Interrupt::Joypad);
                self.pc = RegisterPairValue::from(0x0060);
            }
        }
        self.stall += 4; // indeed 4 full cycles because we don't fetch an instruction
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
                self.stall = 1;
            }
            ArithmeticInstruction::AdcAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_adc(a, b, &mut flags));
                self.stall = 1;
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
                self.stall = 1;
            }
            ArithmeticInstruction::AddAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_add(a, b, &mut flags));
                self.stall = 1;
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
                self.stall = 1;
            }
            ArithmeticInstruction::AndAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_and(a, b, &mut flags));
                self.stall = 1;
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
                self.stall = 1;
            }
            ArithmeticInstruction::CpAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                op_cp(a, b, &mut flags);
                self.stall = 1;
            }
            ArithmeticInstruction::DecR8(reg) => {
                let a = self.register(reg);
                *self.register_mut(reg) = op_dec(a, &mut flags);
            }
            ArithmeticInstruction::DecMemHL => {
                let a = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_dec(a, &mut flags));
                self.stall = 2;
            }
            ArithmeticInstruction::IncR8(reg) => {
                let a = self.register(reg);
                *self.register_mut(reg) = op_inc(a, &mut flags);
            }
            ArithmeticInstruction::IncMemHL => {
                let a = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_inc(a, &mut flags));
                self.stall = 2;
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
                self.stall = 1;
            }
            ArithmeticInstruction::OrAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_or(a, b, &mut flags));
                self.stall = 1;
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
                self.stall = 1;
            }
            ArithmeticInstruction::SbcAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_sbc(a, b, &mut flags));
                self.stall = 1;
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
                self.stall = 1;
            }
            ArithmeticInstruction::SubAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_sub(a, b, &mut flags));
                self.stall = 1;
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
                self.stall = 1;
            }
            ArithmeticInstruction::XorAN8(imm) => {
                let a = self.af.high();
                let b = imm;
                self.af.set_high(op_xor(a, b, &mut flags));
                self.stall = 1;
            }
            ArithmeticInstruction::AddHLR16(reg) => {
                let a = self.hl.as_u16();
                let b = self.register_pair(reg);
                self.hl = RegisterPairValue::from(op_add16(a, b, &mut flags));
                self.stall = 1;
            }
            ArithmeticInstruction::DecR16(reg) => {
                let a = self.register_pair(reg);
                *self.register_pair_mut(reg) = RegisterPairValue::from(op_dec16(a));
                self.stall = 1;
            }
            ArithmeticInstruction::IncR16(reg) => {
                let a = self.register_pair(reg);
                *self.register_pair_mut(reg) = RegisterPairValue::from(op_inc16(a));
                self.stall = 1;
            }
        }
        self.af.set_low(flags.bits());
    }

    fn eval_bit(&mut self, instruction: BitInstruction) {
        let mut flags = self.af.flags();
        self.stall = 1;
        match instruction {
            BitInstruction::Bit(a, reg) => {
                op_bit(a, self.register(reg), &mut flags);
            }
            BitInstruction::BitMemHL(a) => {
                op_bit(a, self.mem.get(self.hl.as_u16()), &mut flags);
                self.stall = 2;
            }
            BitInstruction::Res(a, reg) => {
                *self.register_mut(reg) = op_res(a, self.register(reg));
            }
            BitInstruction::ResMemHL(a) => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_res(a, prev));
                self.stall = 3;
            }
            BitInstruction::Set(a, reg) => {
                *self.register_mut(reg) = op_set(a, self.register(reg));
            }
            BitInstruction::SetMemHL(a) => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_set(a, prev));
                self.stall = 3;
            }
            BitInstruction::Swap(reg) => {
                *self.register_mut(reg) = op_swap(self.register(reg), &mut flags);
                self.stall = 1;
            }
            BitInstruction::SwapMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_swap(prev, &mut flags));
                self.stall = 3;
            }
            BitInstruction::Rl(reg) => {
                *self.register_mut(reg) = op_rl(self.register(reg), &mut flags, false);
            }
            BitInstruction::RlMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_rl(prev, &mut flags, false));
                self.stall = 3;
            }
            BitInstruction::Rla => {
                *self.register_mut(Register::A) = op_rl(self.register(Register::A), &mut flags, true);
            }
            BitInstruction::Rlc(reg) => {
                *self.register_mut(reg) = op_rlc(self.register(reg), &mut flags, false);
            }
            BitInstruction::RlcMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_rlc(prev, &mut flags, false));
                self.stall = 3;
            }
            BitInstruction::Rlca => {
                *self.register_mut(Register::A) = op_rlc(self.register(Register::A), &mut flags, true);
            }
            BitInstruction::Rr(reg) => {
                *self.register_mut(reg) = op_rr(self.register(reg), &mut flags);
            }
            BitInstruction::RrMemHL => {
                println!("Working with value {:#04X}", self.mem.get(self.hl.as_u16()));
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_rr(prev, &mut flags));
                self.stall = 3;
            }
            BitInstruction::Rra => {
                *self.register_mut(Register::A) = op_rr(self.register(Register::A), &mut flags);
            }
            BitInstruction::Rrc(reg) => {
                *self.register_mut(reg) = op_rrc(self.register(reg), &mut flags, false);
            }
            BitInstruction::RrcMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_rrc(prev, &mut flags, false));
                self.stall = 3;
            }
            BitInstruction::Rrca => {
                *self.register_mut(Register::A) = op_rrc(self.register(Register::A), &mut flags, true);
            }
            BitInstruction::Sla(reg) => {
                *self.register_mut(reg) = op_sla(self.register(reg), &mut flags);
            }
            BitInstruction::SlaMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_sla(prev, &mut flags));
                self.stall = 3;
            }
            BitInstruction::Sra(reg) => {
                *self.register_mut(reg) = op_sra(self.register(reg), &mut flags);
            }
            BitInstruction::SraMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_sra(prev, &mut flags));
                self.stall = 3;
            }
            BitInstruction::Srl(reg) => {
                *self.register_mut(reg) = op_srl(self.register(reg), &mut flags);
            }
            BitInstruction::SrlMemHL => {
                let prev = self.mem.get(self.hl.as_u16());
                self.mem.update(self.hl.as_u16(), || op_srl(prev, &mut flags));
                self.stall = 3;
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
                self.stall = 1;
            }
            LoadInstruction::LdR16N16(reg, imm) => {
                *self.register_pair_mut(reg) = RegisterPairValue::from(imm);
                self.stall = 2;
            }
            LoadInstruction::LdMemHLR8(reg) => {
                let val = self.register(reg);
                self.mem.update(self.hl.as_u16(), || val);
                self.stall = 1;
            }
            LoadInstruction::LdMemHLN8(imm) => {
                self.mem.update(self.hl.as_u16(), || imm);
                self.stall = 2;
            }
            LoadInstruction::LdR8MemHL(reg) => {
                *self.register_mut(reg) = self.mem.get(self.hl.as_u16());
                self.stall = 1;
            }
            LoadInstruction::LdMemR16A(reg) => {
                let addr = self.register_pair_mem(reg);
                self.mem.update(addr, || self.af.high());
                self.stall = 1;
            }
            LoadInstruction::LdMemN16A(addr) => {
                self.mem.update(addr, || self.af.high());
                self.stall = 4;
            }
            LoadInstruction::LdhMemN16A(addr) => {
                self.mem.update(0xFF00 + addr, || self.af.high());
                self.stall = 2;
            }
            LoadInstruction::LdhMemCA => {
                self.mem.update(0xFF00 + self.bc.low() as u16, || self.af.high());
                self.stall = 1;
            }
            LoadInstruction::LdAMemR16(reg) => {
                let addr = self.register_pair_mem(reg);
                self.af.set_high(self.mem.get(addr));
                self.stall = 1;
            }
            LoadInstruction::LdAMemN16(addr) => {
                self.af.set_high(self.mem.get(addr));
                self.stall = 3;
            }
            LoadInstruction::LdhAMemN16(addr) => {
                self.af.set_high(self.mem.get(0xFF00 + addr));
                self.stall = 1;
            }
            LoadInstruction::LdhAMemC => {
                self.af.set_high(self.mem.get(0xFF00 + self.bc.low() as u16));
                self.stall = 1;
            }
            LoadInstruction::LdMemHLIA => {
                self.mem.update(self.hl.as_u16(), || self.af.high());
                self.hl = RegisterPairValue::from(self.hl.as_u16().wrapping_add(1));
                self.stall = 1;
            }
            LoadInstruction::LdMemHLDA => {
                self.mem.update(self.hl.as_u16(), || self.af.high());
                self.hl = RegisterPairValue::from(self.hl.as_u16().wrapping_sub(1));
                self.stall = 1;
            }
            LoadInstruction::LdAMemHLI => {
                self.af.set_high(self.mem.get(self.hl.as_u16()));
                self.hl = RegisterPairValue::from(self.hl.as_u16().wrapping_add(1));
                self.stall = 1;
            }
            LoadInstruction::LdAMemHLD => {
                self.af.set_high(self.mem.get(self.hl.as_u16()));
                self.hl = RegisterPairValue::from(self.hl.as_u16().wrapping_sub(1));
                self.stall = 1;
            }
            LoadInstruction::LdhAMemN8(addr) => {
                let val = self.mem.get(0xFF00 + addr as u16);
                self.af.set_high(val);
                self.stall = 2;
            }
            LoadInstruction::LdhMemN8A(addr) => {
                self.mem.update(0xFF00 + addr as u16, || self.af.high());
                self.stall = 2;
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

    fn push(&mut self, value: u16) {
        self.sp = RegisterPairValue::from(self.sp.as_u16().wrapping_sub(2));
        self.mem.write(self.sp.as_u16(), value as u8);
        self.mem.write(self.sp.as_u16().wrapping_add(1), (value >> 8) as u8);
    }

    fn pop(&mut self) -> u16 {
        let lo = self.mem.get(self.sp.as_u16());
        let hi = self.mem.get(self.sp.as_u16().wrapping_add(1));
        self.sp = RegisterPairValue::from(self.sp.as_u16().wrapping_add(2));
        (hi as u16) << 8 | lo as u16
    }

    fn eval_jump(&mut self, instruction: JumpInstruction) {
        match instruction {
            JumpInstruction::CallN16(imm) => {
                self.push(self.pc.as_u16());
                self.pc = RegisterPairValue::from(imm);
                self.stall = 5;
            }
            JumpInstruction::CallCCN16(cond, imm) => {
                if self.eval_cond(cond) {
                    self.push(self.pc.as_u16());
                    self.pc = RegisterPairValue::from(imm);
                    self.stall = 5;
                } else {
                    self.stall = 2;
                }
            }
            JumpInstruction::JpHL => {
                self.pc = self.hl;
            }
            JumpInstruction::JpN16(imm) => {
                self.pc = RegisterPairValue::from(imm);
                self.stall = 3;
            }
            JumpInstruction::JpCCN16(cond, imm) => {
                if self.eval_cond(cond) {
                    self.pc = RegisterPairValue::from(imm);
                    self.stall = 3;
                } else {
                    self.stall = 2;
                }
            }
            JumpInstruction::JrN8(imm) => {
                self.pc = RegisterPairValue::from(self.pc.as_u16().wrapping_add(imm as u16));
                self.stall = 2;
            }
            JumpInstruction::JrCCN8(cond, imm) => {
                if self.eval_cond(cond) {
                    self.pc = RegisterPairValue::from(self.pc.as_u16().wrapping_add(imm as u16));
                    self.stall = 2;
                } else {
                    self.stall = 1;
                }
            }
            JumpInstruction::RetCC(cond) => {
                if self.eval_cond(cond) {
                    self.pc = RegisterPairValue::from(self.pop());
                    self.stall = 4;
                } else {
                    self.stall = 1;
                }
            }
            JumpInstruction::Ret => {
                self.pc = RegisterPairValue::from(self.pop());
                self.stall = 3;
            }
            JumpInstruction::Reti => {
                self.pc = RegisterPairValue::from(self.pop());
                self.ime = true;
                self.stall = 3;
            }
            JumpInstruction::Rst(vec) => {
                self.push(self.pc.as_u16());
                self.pc = RegisterPairValue::from(vec);
                self.stall = 3;
            }
        }
    }

    fn eval_stack(&mut self, instruction: StackInstruction) {
        match instruction {
            StackInstruction::AddHLSP => {
                let mut flags = self.af.flags();
                self.hl = RegisterPairValue::from(op_add16(self.hl.as_u16(), self.sp.as_u16(), &mut flags));
                self.af.set_low(flags.bits());
                self.stall = 1;
            }
            StackInstruction::AddSPE8(imm) => {
                let imm = imm as i16 as u16;
                let mut flags = Flags::empty();
                flags.set(Flags::HALF_CARRY, (self.sp.as_u16() & 0x000F) + (imm & 0x000F) > 0x000F);
                flags.set(Flags::CARRY, (self.sp.as_u16() & 0x00FF) + (imm & 0x00FF) > 0x00FF);
                self.af.set_low(flags.bits());
                self.sp = RegisterPairValue::from(self.sp.as_u16().wrapping_add(imm));
                self.stall = 3;
            }
            StackInstruction::DecSP => {
                self.sp = RegisterPairValue::from(self.sp.as_u16().wrapping_sub(1));
                self.stall = 1;
            }
            StackInstruction::IncSP => {
                self.sp = RegisterPairValue::from(self.sp.as_u16().wrapping_add(1));
                self.stall = 1;
            }
            StackInstruction::LdSPN16(imm) => {
                self.sp = RegisterPairValue::from(imm);
                self.stall = 2;
            }
            StackInstruction::LdMemN16SP(imm) => {
                let addr = imm;
                self.mem.update(addr, || self.sp.low());
                self.mem.update(addr + 1, || self.sp.high());
                self.stall = 4;
            }
            StackInstruction::LdHLSPPlusE8(imm) => {
                self.hl = RegisterPairValue::from(self.sp.as_u16().wrapping_add(imm as u16));
                self.stall = 2;
                let mut flags = Flags::empty();
                flags.set(Flags::ZERO, false);
                flags.set(Flags::SUBTRACT, false);
                flags.set(Flags::HALF_CARRY, (self.sp.low() & 0xF) + (imm as u8 & 0xF) > 0xF);
                flags.set(Flags::CARRY, (self.sp.low() as u16) + ((imm as u8) as u16) > 0x00FF);
                self.af.set_low(flags.bits());
            }
            StackInstruction::LdSPHL => {
                self.sp = self.hl;
                self.stall = 1;
            }
            StackInstruction::PopAF => {
                self.af = RegisterPairValue::from(self.pop());
                let mut flags = Flags::empty();
                flags.set(Flags::ZERO, self.af.low() & Flags::ZERO.bits() != 0);
                flags.set(Flags::SUBTRACT, self.af.low() & Flags::SUBTRACT.bits() != 0);
                flags.set(Flags::HALF_CARRY, self.af.low() & Flags::HALF_CARRY.bits() != 0);
                flags.set(Flags::CARRY, self.af.low() & Flags::CARRY.bits() != 0);
                self.af.set_low(flags.bits());
                self.stall = 2;
            }
            StackInstruction::PopR16(reg) => {
                *self.register_pair_stk_mut(reg) = RegisterPairValue::from(self.pop());
                self.stall = 2;
            }
            StackInstruction::PushAF => { // todo: why is this variant required?
                self.push(self.af.as_u16());
                self.stall = 3;
            }
            StackInstruction::PushR16(reg) => {
                match reg {
                    RegisterPairStk::BC => {
                        self.push(self.bc.as_u16());
                    }
                    RegisterPairStk::DE => {
                        self.push(self.de.as_u16());
                    }
                    RegisterPairStk::HL => {
                        self.push(self.hl.as_u16());
                    }
                    RegisterPairStk::AF => {
                        self.push(self.af.as_u16());
                    }
                }
                self.stall = 3;
            }
        }
    }

    fn eval_misc(&mut self, instruction: MiscInstruction) {
        match instruction {
            MiscInstruction::Ccf => {
                let mut flags = self.af.flags();
                flags.toggle(Flags::CARRY);
                flags.set(Flags::SUBTRACT, false);
                flags.set(Flags::HALF_CARRY, false);
                self.af.set_low(flags.bits());
            }
            MiscInstruction::Cpl => {
                let a = self.af.high();
                self.af.set_high(!a);
                let mut flags = self.af.flags();
                flags.set(Flags::SUBTRACT, true);
                flags.set(Flags::HALF_CARRY, true);
                self.af.set_low(flags.bits());
            }
            MiscInstruction::DaA => {
                let mut flags = self.af.flags();
                let mut a = self.af.high();
                let mut correction = if self.af.flags().contains(Flags::CARRY) { 0x60 } else { 0x00 };
                if self.af.flags().contains(Flags::HALF_CARRY) {
                    correction |= 0x06;
                }
                if !self.af.flags().contains(Flags::SUBTRACT) {
                    if a & 0x0F > 0x09 {
                        correction |= 0x06;
                    }
                    if a > 0x99 {
                        correction |= 0x60;
                    }
                    a = a.wrapping_add(correction);
                } else {
                    a = a.wrapping_sub(correction);
                }
                flags.set(Flags::CARRY, correction >= 0x60);
                flags.set(Flags::HALF_CARRY, false);
                flags.set(Flags::ZERO, a == 0);
                self.af.set_high(a);
                self.af.set_low(flags.bits());
            }
            MiscInstruction::Di => {
                self.di_ctr = 2;
                info!("Disabling interrupts...")
            }
            MiscInstruction::Ei => {
                self.ei_ctr = 2;
                info!("Enabling interrupts...")
            }
            MiscInstruction::Halt => {
                self.halted = true;
                info!("Halting CPU...")
            }
            MiscInstruction::Nop => {}
            MiscInstruction::Scf => {
                let mut flags = self.af.flags();
                flags.insert(Flags::CARRY);
                flags.set(Flags::SUBTRACT, false);
                flags.set(Flags::HALF_CARRY, false);
                self.af.set_low(flags.bits());
            }
            MiscInstruction::Stop => {
                self.halted = true; // TODO: not sure if this is correct...
                info!("Stopping CPU...")
            }
        }
    }
    
    pub fn control_message(&mut self, msg: ControlMsg) {
        match msg {
            ControlMsg::Terminate => self.terminate = true,
            _ => self.mem.control_msg(msg),
        }
    }
}
