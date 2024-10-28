use crate::{Register, RegisterPair, RegisterPairMem, RegisterPairStk};
use std::fmt::{Debug, Formatter};

/*
8-bit Arithmetic and Logic Instructions
ADC A,r8
ADC A,[HL]
ADC A,n8
ADD A,r8
ADD A,[HL]
ADD A,n8
AND A,r8
AND A,[HL]
AND A,n8
CP A,r8
CP A,[HL]
CP A,n8
DEC r8
DEC [HL]
INC r8
INC [HL]
OR A,r8
OR A,[HL]
OR A,n8
SBC A,r8
SBC A,[HL]
SBC A,n8
SUB A,r8
SUB A,[HL]
SUB A,n8
XOR A,r8
XOR A,[HL]
XOR A,n8
16-bit Arithmetic Instructions
ADD HL,r16
DEC r16
INC r16
*/

#[derive(Debug)]
pub enum ArithmeticInstruction {
    AdcAR8(Register),       // Add with carry, from register to A
    AdcAMemHL,              // Add with carry, from memory at HL to A
    AdcAN8(u8),             // Add with carry, from immediate value to A
    AddAR8(Register),       // Add, from register to A
    AddAMemHL,              // Add, from memory at HL to A
    AddAN8(u8),             // Add, from immediate value to A
    AndAR8(Register),       // And, register AND A -> A
    AndAMemHL,              // And, memory at HL AND A -> A
    AndAN8(u8),             // And, immediate value AND A -> A
    CpAR8(Register),        // Compare, register with A
    CpAMemHL,               // Compare, memory at HL with A
    CpAN8(u8),              // Compare, immediate value with A
    DecR8(Register),        // Decrement register
    DecMemHL,               // Decrement memory at HL
    IncR8(Register),        // Increment register
    IncMemHL,               // Increment memory at HL
    OrAR8(Register),        // Or, register OR A -> A
    OrAMemHL,               // Or, memory at HL OR A -> A
    OrAN8(u8),              // Or, immediate value OR A -> A
    SbcAR8(Register),       // Subtract with carry, register from A
    SbcAMemHL,              // Subtract with carry, memory at HL from A
    SbcAN8(u8),             // Subtract with carry, immediate value from A
    SubAR8(Register),       // Subtract, register from A
    SubAMemHL,              // Subtract, memory at HL from A
    SubAN8(u8),             // Subtract, immediate value from A
    XorAR8(Register),       // Xor, register XOR A -> A
    XorAMemHL,              // Xor, memory at HL XOR A -> A
    XorAN8(u8),             // Xor, immediate value XOR A -> A
    AddHLR16(RegisterPair), // Add, register pair to HL
    DecR16(RegisterPair),   // Decrement register pair
    IncR16(RegisterPair),   // Increment register pair
}

/*
Bit Operations Instructions
BIT u3,r8
BIT u3,[HL]
RES u3,r8
RES u3,[HL]
SET u3,r8
SET u3,[HL]
SWAP r8
SWAP [HL]
Bit Shift Instructions
RL r8
RL [HL]
RLA
RLC r8
RLC [HL]
RLCA
RR r8
RR [HL]
RRA
RRC r8
RRC [HL]
RRCA
SLA r8
SLA [HL]
SRA r8
SRA [HL]
SRL r8
SRL [HL]
*/

#[derive(Debug)]
pub enum BitInstruction {
    Bit(u8, Register), // Test u'th bit in register, set zero flag if not set
    BitMemHL(u8),      // Test u'th bit in memory at HL, set zero flag if not set
    Res(u8, Register), // Reset u'th bit in register to 0
    ResMemHL(u8),      // Reset u'th bit in memory at HL to 0
    Set(u8, Register), // Set u'th bit in register to 1
    SetMemHL(u8),      // Set u'th bit in memory at HL to 1
    Swap(Register),    // Swap upper and lower nibbles in register
    SwapMemHL,         // Swap upper and lower nibbles in memory at HL
    Rl(Register),      // Rotate bits in register r8 left, through the carry flag.
    RlMemHL,           // Rotate bits in memory at HL left, through the carry flag.
    Rla,               // Rotate bits in register A left, through the carry flag.
    Rlc(Register),     // Rotate bits in register r8 left, through the carry flag.
    RlcMemHL,          // Rotate bits in memory at HL left, setting carry flag to MSB.
    Rlca,              // Rotate bits in register A left, setting carry flag to MSB.
    Rr(Register),      // Rotate bits in register r8 right, through the carry flag.
    RrMemHL,           // Rotate bits in memory at HL right, through the carry flag.
    Rra,               // Rotate bits in register A right, through the carry flag.
    Rrc(Register),     // Rotate bits in register r8 right, setting carry flag to LSB.
    RrcMemHL,          // Rotate bits in memory at HL right, setting carry flag to LSB.
    Rrca,              // Rotate bits in register A right, setting carry flag to LSB.
    Sla(Register),     // Shift bits in register r8 left, setting carry flag to MSB.
    SlaMemHL,          // Shift bits in memory at HL left, setting carry flag to MSB.
    Sra(Register),     // Shift bits in register r8 right, setting carry flag to MSB.
    SraMemHL,          // Shift bits in memory at HL right, setting carry flag to MSB.
    Srl(Register),     // Shift bits in register r8 right, setting carry flag to LSB.
    SrlMemHL,          // Shift bits in memory at HL right, setting carry flag to LSB.
}

/*
Load Instructions
LD r8,r8
LD r8,n8
LD r16,n16
LD [HL],r8
LD [HL],n8
LD r8,[HL]
LD [r16],A
LD [n16],A
LDH [n16],A
LDH [C],A
LD A,[r16]
LD A,[n16]
LDH A,[n16]
LDH A,[C]
LD [HLI],A
LD [HLD],A
LD A,[HLI]
LD A,[HLD]
*/

#[derive(Debug)]
pub enum LoadInstruction {
    LdR8R8(Register, Register), // Load (copy) value in register on the right into register on the left.
    LdR8N8(Register, u8),       // Load immediate value into register.
    LdR16N16(RegisterPair, u16), // Load immediate value into register pair.
    LdMemHLR8(Register),        // Store value in register r8 into memory pointed to by register HL.
    LdMemHLN8(u8),              // Store immediate value into memory pointed to by register HL.
    LdR8MemHL(Register),        // Load value in memory pointed to by register HL into register.
    LdMemR16A(RegisterPairMem), // Store value in register A into memory pointed to by register pair.
    LdMemN16A(u16), // Store value in register A into memory pointed to by immediate value.
    LdhMemN16A(u16), // Store value in register A into memory pointed to by immediate value, high.
    LdhMemCA,       // Store value in register A into memory pointed to by register C, high.
    LdAMemR16(RegisterPairMem), // Load value in memory pointed to by register pair into register A.
    LdAMemN16(u16), // Load value in memory pointed to by immediate value into register A.
    LdhAMemN16(u16), // Load value in memory pointed to by immediate value, high into register A.
    LdhAMemC,       // Load value in memory pointed to by register C, high into register A.
    LdMemHLIA, // Store value in register A into memory pointed to by register HL, then increment HL.
    LdMemHLDA, // Store value in register A into memory pointed to by register HL, then decrement HL.
    LdAMemHLI, // Load value in memory pointed to by register HL into register A, then increment HL.
    LdAMemHLD, // Load value in memory pointed to by register HL into register A, then decrement HL.
    LdhAMemN8(u8),
    LdhMemN8A(u8),
}

/*
Jumps and Subroutines
CALL n16
CALL cc,n16
JP HL
JP n16
JP cc,n16
JR n16
JR cc,n16
RET cc
RET
RETI
RST vec
*/

#[derive(Debug)]
pub enum Condition {
    NotZero,  // Z flag is not set.
    Zero,     // Z flag is set.
    NotCarry, // C flag is not set.
    Carry,    // C flag is set.
}
impl Condition {
    pub fn from_bits(a: u8, b: u8) -> Condition {
        match (a, b) {
            (0, 0) => Condition::NotZero,
            (0, 1) => Condition::Zero,
            (1, 0) => Condition::NotCarry,
            (1, 1) => Condition::Carry,
            _ => panic!("Invalid condition bits: {}{}", a, b),
        }
    }
}

pub enum JumpInstruction {
    CallN16(u16),              // Call subroutine at immediate value.
    CallCCN16(Condition, u16), // Call subroutine at immediate value if condition is met.
    JpHL,                      // Jump to address in register pair HL.
    JpN16(u16),                // Jump to immediate value.
    JpCCN16(Condition, u16),   // Jump to immediate value if condition is met.
    JrN8(i8),                  // Jump relative to immediate value.
    JrCCN8(Condition, i8),     // Jump relative to immediate value if condition is met.
    RetCC(Condition),          // Return from subroutine if condition is met.
    Ret,                       // Return from subroutine.
    Reti,                      // Return from subroutine and enable interrupts.
    Rst(u16),                  // Call subroutine at vector.
}

// same as derived Debug impl but print u16, u8, i8 as hex
impl Debug for JumpInstruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            JumpInstruction::CallN16(n) => write!(f, "CallN16(${:04X})", n),
            JumpInstruction::CallCCN16(c, n) => write!(f, "CallCCN16({:?}, ${:04X})", c, n),
            JumpInstruction::JpHL => write!(f, "JpHL"),
            JumpInstruction::JpN16(n) => write!(f, "JpN16(${:04X})", n),
            JumpInstruction::JpCCN16(c, n) => write!(f, "JpCCN16({:?}, ${:04X})", c, n),
            JumpInstruction::JrN8(n) => write!(f, "JrN8(${:02X})", n),
            JumpInstruction::JrCCN8(c, n) => write!(f, "JrCCN8({:?}, ${:02X})", c, n),
            JumpInstruction::RetCC(c) => write!(f, "RetCC({:?})", c),
            JumpInstruction::Ret => write!(f, "Ret"),
            JumpInstruction::Reti => write!(f, "Reti"),
            JumpInstruction::Rst(n) => write!(f, "Rst(${:02X})", n),
        }
    }
}

/*
Stack Operations Instructions
ADD HL,SP
ADD SP,e8
DEC SP
INC SP
LD SP,n16
LD [n16],SP
LD HL,SP+e8
LD SP,HL
POP AF
POP r16
PUSH AF
PUSH r16
*/

#[derive(Debug)]
pub enum StackInstruction {
    AddHLSP,                  // Add SP to HL.  TODO: why are there unused variants?
    AddSPE8(i8),              // Add immediate value to SP.
    DecSP,                    // Decrement SP.
    IncSP,                    // Increment SP.
    LdSPN16(u16),             // Load immediate value into SP.
    LdMemN16SP(u16),          // Store SP & $FF at address n16 and SP >> 8 at address n16 + 1.
    LdHLSPPlusE8(i8),         // Load SP plus immediate value into HL.
    LdSPHL,                   // Load HL into SP.
    PopAF,                    // Pop value from stack into AF.
    PopR16(RegisterPairStk),  // Pop value from stack into register pair.
    PushAF,                   // Push value in AF onto stack.
    PushR16(RegisterPairStk), // Push value in register pair onto stack.
}

/*
Miscellaneous Instructions
CCF
CPL
DAA
DI
EI
HALT
NOP
SCF
STOP
 */

#[derive(Debug)]
pub enum MiscInstruction {
    Ccf,  // Complement carry flag.
    Cpl,  // Complement A.
    DaA,  // Decimal adjust A.
    Di,   // Disable interrupts.
    Ei,   // Enable interrupts.
    Halt, // Halt CPU.
    Nop,  // No operation.
    Scf,  // Set carry flag.
    Stop, // Stop CPU.
}

#[derive(Debug)]
pub enum Instruction {
    Arithmetic(ArithmeticInstruction),
    Bit(BitInstruction),
    Load(LoadInstruction),
    Jump(JumpInstruction),
    Stack(StackInstruction),
    Misc(MiscInstruction),
}
