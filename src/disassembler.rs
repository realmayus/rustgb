use log::debug;
use crate::isa::{ArithmeticInstruction, BitInstruction, Condition, Instruction, JumpInstruction, LoadInstruction, MiscInstruction, StackInstruction};
use crate::{Register, RegisterPair, RegisterPairMem, RegisterPairStk};
use crate::memory::{Mbc, MappedMemory, Memory};


pub struct Disassembler {
    cursor: usize,
}

impl Disassembler {
    pub fn new() -> Disassembler {
        Disassembler {
            cursor: 0,
        }
    }

    pub fn disassemble<M>(&mut self, mem: &M, pc: u16) -> (Instruction, u16) where M: crate::memory::Memory {
        self.cursor = pc as usize;
        let byte = self.nom(mem);

        let instruction = match Self::bits_tup(byte) {
            // Block 0
            (0, 0, 0, 0, 0, 0, 0, 0) => { Instruction::Misc(MiscInstruction::Nop) },
            (0, 0, a, b, 0, 0, 0, 1) => Instruction::Load(LoadInstruction::LdR16N16(RegisterPair::from_bits(a,b), self.nomnom(mem))),
            (0, 0, a, b, 0, 0, 1, 0) => Instruction::Load(LoadInstruction::LdMemR16A(RegisterPairMem::from_bits(a,b))),
            (0, 0, a, b, 1, 0, 1, 0) => Instruction::Load(LoadInstruction::LdAMemR16(RegisterPairMem::from_bits(a,b))),
            (0, 0, 0, 0, 1, 0, 0, 0) => Instruction::Stack(StackInstruction::LdMemN16SP(self.nomnom(mem))),

            (0, 0, a, b, 0, 0, 1, 1) => Instruction::Arithmetic(ArithmeticInstruction::IncR16(RegisterPair::from_bits(a,b))),
            (0, 0, a, b, 1, 0, 1, 1) => Instruction::Arithmetic(ArithmeticInstruction::DecR16(RegisterPair::from_bits(a,b))),
            (0, 0, a, b, 1, 0, 0, 1) => Instruction::Arithmetic(ArithmeticInstruction::AddHLR16(RegisterPair::from_bits(a,b))),

            (0, 0, 1, 1, 0, 1, 0, 0) => Instruction::Arithmetic(ArithmeticInstruction::IncMemHL),
            (0, 0, a, b, c, 1, 0, 0) => Instruction::Arithmetic(ArithmeticInstruction::IncR8(Register::from_bits(a,b,c))),
            (0, 0, 1, 1, 0, 1, 0, 1) => Instruction::Arithmetic(ArithmeticInstruction::DecMemHL),
            (0, 0, a, b, c, 1, 0, 1) => Instruction::Arithmetic(ArithmeticInstruction::DecR8(Register::from_bits(a,b,c))),

            (0, 0, 1, 1, 0, 1, 1, 0) => Instruction::Load(LoadInstruction::LdMemHLN8(self.nom(mem))),
            (0, 0, a, b, c, 1, 1, 0) => Instruction::Load(LoadInstruction::LdR8N8(Register::from_bits(a,b,c), self.nom(mem))),

            (0, 0, 0, 0, 0, 1, 1, 1) => Instruction::Bit(BitInstruction::Rlca),
            (0, 0, 0, 0, 1, 1, 1, 1) => Instruction::Bit(BitInstruction::Rrca),
            (0, 0, 0, 1, 0, 1, 1, 1) => Instruction::Bit(BitInstruction::Rla),
            (0, 0, 0, 1, 1, 1, 1, 1) => Instruction::Bit(BitInstruction::Rra),
            (0, 0, 1, 0, 0, 1, 1, 1) => Instruction::Misc(MiscInstruction::DaA),
            (0, 0, 1, 0, 1, 1, 1, 1) => Instruction::Misc(MiscInstruction::Cpl),
            (0, 0, 1, 1, 0, 1, 1, 1) => Instruction::Misc(MiscInstruction::Scf),
            (0, 0, 1, 1, 1, 1, 1, 1) => Instruction::Misc(MiscInstruction::Ccf),

            (0, 0, 0, 1, 1, 0, 0, 0) => Instruction::Jump(JumpInstruction::JrN8(self.nom(mem) as i8)),
            (0, 0, 1, a, b, 0, 0, 0) => Instruction::Jump(JumpInstruction::JrCCN8(Condition::from_bits(a,b), self.nom(mem) as i8)),

            (0, 0, 0, 1, 0, 0, 0, 0) => { Instruction::Misc(MiscInstruction::Stop) },


            // Block 1
            (0, 1, 1, 1, 0, 1, 1, 0) => Instruction::Misc(MiscInstruction::Halt),

            (0, 1, 1, 1, 0, a, b, c) => Instruction::Load(LoadInstruction::LdMemHLR8(Register::from_bits(a, b, c))),
            (0, 1, a, b, c, 1, 1, 0) => Instruction::Load(LoadInstruction::LdR8MemHL(Register::from_bits(a, b, c))),
            (0, 1, a, b, c, x, y, z) => Instruction::Load(LoadInstruction::LdR8R8(Register::from_bits(a, b, c), Register::from_bits(x, y, z))),

            // Block 2
            (1, 0, 0, 0, 0, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::AddAMemHL),
            (1, 0, 0, 0, 0, a, b, c) => Instruction::Arithmetic(ArithmeticInstruction::AddAR8(Register::from_bits(a, b, c))),
            (1, 0, 0, 0, 1, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::AdcAMemHL),
            (1, 0, 0, 0, 1, a, b, c) => Instruction::Arithmetic(ArithmeticInstruction::AdcAR8(Register::from_bits(a, b, c))),
            (1, 0, 0, 1, 0, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::SubAMemHL),
            (1, 0, 0, 1, 0, a, b, c) => Instruction::Arithmetic(ArithmeticInstruction::SubAR8(Register::from_bits(a, b, c))),
            (1, 0, 0, 1, 1, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::SbcAMemHL),
            (1, 0, 0, 1, 1, a, b, c) => Instruction::Arithmetic(ArithmeticInstruction::SbcAR8(Register::from_bits(a, b, c))),
            (1, 0, 1, 0, 0, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::AndAMemHL),
            (1, 0, 1, 0, 0, a, b, c) => Instruction::Arithmetic(ArithmeticInstruction::AndAR8(Register::from_bits(a, b, c))),
            (1, 0, 1, 0, 1, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::XorAMemHL),
            (1, 0, 1, 0, 1, a, b, c) => Instruction::Arithmetic(ArithmeticInstruction::XorAR8(Register::from_bits(a, b, c))),
            (1, 0, 1, 1, 0, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::OrAMemHL),
            (1, 0, 1, 1, 0, a, b, c) => Instruction::Arithmetic(ArithmeticInstruction::OrAR8(Register::from_bits(a, b, c))),
            (1, 0, 1, 1, 1, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::CpAMemHL),
            (1, 0, 1, 1, 1, a, b, c) => Instruction::Arithmetic(ArithmeticInstruction::CpAR8(Register::from_bits(a, b, c))),

            // Block 3
            (1, 1, 0, 0, 0, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::AddAN8(self.nom(mem))),
            (1, 1, 0, 0, 1, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::AdcAN8(self.nom(mem))),
            (1, 1, 0, 1, 0, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::SubAN8(self.nom(mem))),
            (1, 1, 0, 1, 1, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::SbcAN8(self.nom(mem))),
            (1, 1, 1, 0, 0, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::AndAN8(self.nom(mem))),
            (1, 1, 1, 0, 1, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::XorAN8(self.nom(mem))),
            (1, 1, 1, 1, 0, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::OrAN8(self.nom(mem))),
            (1, 1, 1, 1, 1, 1, 1, 0) => Instruction::Arithmetic(ArithmeticInstruction::CpAN8(self.nom(mem))),

            (1, 1, 0, a, b, 0, 0, 0) => Instruction::Jump(JumpInstruction::RetCC(Condition::from_bits(a,b))),
            (1, 1, 0, 0, 1, 0, 0, 1) => Instruction::Jump(JumpInstruction::Ret),
            (1, 1, 0, 1, 1, 0, 0, 1) => Instruction::Jump(JumpInstruction::Reti),
            (1, 1, 0, a, b, 0, 1, 0) => Instruction::Jump(JumpInstruction::JpCCN16(Condition::from_bits(a,b), self.nomnom(mem))),
            (1, 1, 0, 0, 0, 0, 1, 1) => Instruction::Jump(JumpInstruction::JpN16(self.nomnom(mem))),
            (1, 1, 1, 0, 1, 0, 0, 1) => Instruction::Jump(JumpInstruction::JpHL),
            (1, 1, 0, a, b, 1, 0, 0) => Instruction::Jump(JumpInstruction::CallCCN16(Condition::from_bits(a,b), self.nomnom(mem))),
            (1, 1, 0, 0, 1, 1, 0, 1) => Instruction::Jump(JumpInstruction::CallN16(self.nomnom(mem))),
            (1, 1, a, b, c, 1, 1, 1) => Instruction::Jump(JumpInstruction::Rst((a << 2 | b << 1 | c) as u16 * 8)),

            (1, 1, 1, 1, 0, 0, 0, 1) => Instruction::Stack(StackInstruction::PopAF),
            (1, 1, a, b, 0, 0, 0, 1) => Instruction::Stack(StackInstruction::PopR16(RegisterPairStk::from_bits(a,b))),
            (1, 1, 1, 1, 0, 1, 0, 1) => Instruction::Stack(StackInstruction::PushAF),
            (1, 1, a, b, 0, 1, 0, 1) => Instruction::Stack(StackInstruction::PushR16(RegisterPairStk::from_bits(a,b))),

            (1, 1, 0, 0, 1, 0, 1, 1) => self.parse_prefix(mem),

            (1, 1, 1, 0, 0, 0, 1, 0) => Instruction::Load(LoadInstruction::LdhMemCA),
            (1, 1, 1, 0, 0, 0, 0, 0) => Instruction::Load(LoadInstruction::LdhMemN8A(self.nom(mem))),
            (1, 1, 1, 0, 1, 0, 1, 0) => Instruction::Load(LoadInstruction::LdMemN16A(self.nomnom(mem))),
            (1, 1, 1, 1, 0, 0, 1, 0) => Instruction::Load(LoadInstruction::LdhAMemC),
            (1, 1, 1, 1, 0, 0, 0, 0) => Instruction::Load(LoadInstruction::LdhAMemN8(self.nom(mem))),
            (1, 1, 1, 1, 1, 0, 1, 0) => Instruction::Load(LoadInstruction::LdAMemN16(self.nomnom(mem))),

            (1, 1, 1, 0, 1, 0, 0, 0) => Instruction::Stack(StackInstruction::AddSPE8(self.nom(mem) as i8)),
            (1, 1, 1, 1, 1, 0, 0, 0) => Instruction::Stack(StackInstruction::LdHLSPPlusE8(self.nom(mem) as i8)),
            (1, 1, 1, 1, 1, 0, 0, 1) => Instruction::Stack(StackInstruction::LdSPHL),

            (1, 1, 1, 1, 0, 0, 1, 1) => Instruction::Misc(MiscInstruction::Di),
            (1, 1, 1, 1, 1, 0, 1, 1) => Instruction::Misc(MiscInstruction::Ei),

            _ => panic!("Invalid instruction: {:08b}", byte),
        };
        debug!("{:?}", instruction);
        (instruction, self.cursor as u16)
    
    }

    fn parse_prefix<M>(&mut self, mem: &M) -> Instruction where M: Memory {
        match Self::bits_tup(self.nom(mem)) {
            (0, 0, 0, 0, 0, 1, 1, 0) => Instruction::Bit(BitInstruction::RlcMemHL),
            (0, 0, 0, 0, 0, a, b, c) => Instruction::Bit(BitInstruction::Rlc(Register::from_bits(a,b,c))),
            (0, 0, 0, 0, 1, 1, 1, 0) => Instruction::Bit(BitInstruction::RrcMemHL),
            (0, 0, 0, 0, 1, a, b, c) => Instruction::Bit(BitInstruction::Rrc(Register::from_bits(a,b,c))),
            (0, 0, 0, 1, 0, 1, 1, 0) => Instruction::Bit(BitInstruction::RlMemHL),
            (0, 0, 0, 1, 0, a, b, c) => Instruction::Bit(BitInstruction::Rl(Register::from_bits(a,b,c))),
            (0, 0, 0, 1, 1, 1, 1, 0) => Instruction::Bit(BitInstruction::RrMemHL),
            (0, 0, 0, 1, 1, a, b, c) => Instruction::Bit(BitInstruction::Rr(Register::from_bits(a,b,c))),
            (0, 0, 1, 0, 0, 1, 1, 0) => Instruction::Bit(BitInstruction::SlaMemHL),
            (0, 0, 1, 0, 0, a, b, c) => Instruction::Bit(BitInstruction::Sla(Register::from_bits(a,b,c))),
            (0, 0, 1, 0, 1, 1, 1, 0) => Instruction::Bit(BitInstruction::SraMemHL),
            (0, 0, 1, 0, 1, a, b, c) => Instruction::Bit(BitInstruction::Sra(Register::from_bits(a,b,c))),
            (0, 0, 1, 1, 0, 1, 1, 0) => Instruction::Bit(BitInstruction::SwapMemHL),
            (0, 0, 1, 1, 0, a, b, c) => Instruction::Bit(BitInstruction::Swap(Register::from_bits(a,b,c))),
            (0, 0, 1, 1, 1, 1, 1, 0) => Instruction::Bit(BitInstruction::SrlMemHL),
            (0, 0, 1, 1, 1, a, b, c) => Instruction::Bit(BitInstruction::Srl(Register::from_bits(a,b,c))),

            (0, 1, x, y, z, 1, 1, 0) => Instruction::Bit(BitInstruction::BitMemHL(x << 2 | y << 1 | z)),
            (0, 1, x, y, z, a, b, c) => Instruction::Bit(BitInstruction::Bit(x << 2 | y << 1 | z, Register::from_bits(a,b,c))),
            (1, 0, x, y, z, 1, 1, 0) => Instruction::Bit(BitInstruction::ResMemHL(x << 2 | y << 1 | z)),
            (1, 0, x, y, z, a, b, c) => Instruction::Bit(BitInstruction::Res(x << 2 | y << 1 | z, Register::from_bits(a,b,c))),
            (1, 1, x, y, z, 1, 1, 0) => Instruction::Bit(BitInstruction::SetMemHL(x << 2 | y << 1 | z)),
            (1, 1, x, y, z, a, b, c) => Instruction::Bit(BitInstruction::Set(x << 2 | y << 1 | z, Register::from_bits(a,b,c))),
            x => panic!("Invalid prefix instruction: {:?}", x),
        }
    }

    pub const fn bits_tup(byte: u8) -> (u8, u8, u8, u8, u8, u8, u8, u8) {
        (byte >> 7 & 1,
         byte >> 6 & 1,
         byte >> 5 & 1,
         byte >> 4 & 1,
         byte >> 3 & 1,
         byte >> 2 & 1,
         byte >> 1 & 1,
         byte & 1)
    }

    const fn u16_from_bytes(high: u8, low: u8) -> u16 {
        ((high as u16) << 8) | low as u16
    }

    fn nom<M>(&mut self, memory: &M) -> u8 where M: Memory {
        self.cursor += 1;
        memory.get((self.cursor - 1) as u16)
    }

    fn nomnom<M>(&mut self, memory: &M) -> u16 where M: Memory {
        self.cursor += 2;
        Self::u16_from_bytes(memory.get((self.cursor - 1) as u16), memory.get((self.cursor - 2) as u16))
    }
}