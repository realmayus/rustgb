mod cpu;
mod disassembler;
mod isa;

use bitflags::bitflags;
use std::fs;

bitflags! {
    struct Flags: u8 {
        const CARRY = 0b00000001;
        const ADD_SUBTRACT = 0b00000010;
        const PARITY_OVERFLOW = 0b00000100;
        // Bit 4 is unused
        const HALF_CARRY = 0b00010000;
        // Bit 5 is unused
        const ZERO = 0b01000000;
        const SIGN = 0b10000000;
    }
}

bitflags! {
    struct InterruptFlags: u8 {
        const IFF_1 = 0b00000001;
        const IFF_2 = 0b00000010;
    }
}

#[derive(Debug)]
enum RegisterPair {
    BC,
    DE,
    HL,
    SP,
    AF,
}

impl RegisterPair {
    pub const fn from_bits(a: u8, b: u8) -> RegisterPair {
        match (a, b) {
            (0, 0) => RegisterPair::BC,
            (0, 1) => RegisterPair::DE,
            (1, 0) => RegisterPair::HL,
            (1, 1) => RegisterPair::SP,
            _ => panic!("Invalid register pair bits"),
        }
    }
}

#[derive(Debug)]
enum Register {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

impl Register {
    pub fn from_bits(a: u8, b: u8, c: u8) -> Register {
        match (a, b, c) {
            (0, 0, 0) => Register::B,
            (0, 0, 1) => Register::C,
            (0, 1, 0) => Register::D,
            (0, 1, 1) => Register::E,
            (1, 0, 0) => Register::H,
            (1, 0, 1) => Register::L,
            (1, 1, 1) => Register::A,
            _ => panic!("Invalid register bits {a}{b}{c}"),
        }
    }
}

pub fn main() {
    let rom = fs::read("test.gb").expect("Unable to read file");

    let title = rom[0x134..0x143]
        .iter()
        .map(|&c| c as char)
        .collect::<String>();
    println!("Loading {title}...");

    let cpu = Cpu::new();
}
