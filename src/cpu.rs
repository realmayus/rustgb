use crate::disassembler::Disassembler;
use crate::isa::RegisterPairValue;

struct Cpu {
    af: RegisterPairValue,
    bc: RegisterPairValue,
    de: RegisterPairValue,
    hl: RegisterPairValue,
    sp: RegisterPairValue,
    pc: RegisterPairValue,
    disassembler: Disassembler,
}

impl Cpu {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            af: Default::default(),
            bc: Default::default(),
            de: Default::default(),
            hl: Default::default(),
            sp: Default::default(),
            pc: RegisterPairValue(0x100),
            disassembler: Disassembler::new(rom),
        }
    }

    pub fn run(&mut self) {
        loop {
            self.disassembler.goto(self.pc.0);
        }
    }
}
