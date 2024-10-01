mod cpu;
mod disassembler;
mod isa;
mod arithmetic;
mod memory;
mod ppu;
mod timer;

use bitflags::bitflags;
use std::{fs, thread};
use std::sync::mpsc;
use crate::cpu::Cpu;
use crate::ppu::Ppu;

bitflags! {
    struct Flags: u8 {
        const CARRY = 0b00000001;
        const SUBTRACT = 0b00000010;
        const PARITY_OVERFLOW = 0b00000100;
        // Bit 4 is unused
        const HALF_CARRY = 0b00010000;
        // Bit 5 is unused
        const ZERO = 0b01000000;
        const SIGN = 0b10000000;
    }
}

#[derive(Debug, Copy, Clone)]
enum RegisterPair {
    BC,
    DE,
    HL,
    SP,
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

#[derive(Debug, Copy, Clone)]
enum RegisterPairStk {
    BC,
    DE,
    HL,
    AF,
}

impl RegisterPairStk {
    pub const fn from_bits(a: u8, b: u8) -> RegisterPairStk {
        match (a, b) {
            (0, 0) => RegisterPairStk::BC,
            (0, 1) => RegisterPairStk::DE,
            (1, 0) => RegisterPairStk::HL,
            (1, 1) => RegisterPairStk::AF,
            _ => panic!("Invalid register pair bits"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum RegisterPairMem {
    BC,
    DE,
    HLI,
    HLD,
}

impl RegisterPairMem {
    pub const fn from_bits(a: u8, b: u8) -> RegisterPairMem {
        match (a, b) {
            (0, 0) => RegisterPairMem::BC,
            (0, 1) => RegisterPairMem::DE,
            (1, 0) => RegisterPairMem::HLI,
            (1, 1) => RegisterPairMem::HLD,
            _ => panic!("Invalid register pair bits"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Copy, Clone)]
enum CartridgeType {
    RomOnly,
    Mbc1,
    Mbc1Ram,
    Mbc1RamBattery,
    Mbc2,
    Mbc2Battery,
    RomRam,
    RomRamBattery,
    Mmm01,
    Mmm01Sram,
    Mmm01SramBattery,
    Mbc3TimerBattery,
    Mbc3TimerRamBattery,
    Mbc3,
    Mbc3Ram,
    Mbc3RamBattery,
    Mbc5,
    Mbc5Ram,
    Mbc5RamBattery,
    Mbc5Rumble,
    Mbc5RumbleSram,
    Mbc5RumbleSramBattery,
    Mbc6,
    Mbc7SensorRumbleRamBattery,
    PocketCamera,
    BandaiTama5,
    HuC3,
    HuC1RamBattery,
}

impl From<u8> for CartridgeType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => CartridgeType::RomOnly,
            0x01 => CartridgeType::Mbc1,
            0x02 => CartridgeType::Mbc1Ram,
            0x03 => CartridgeType::Mbc1RamBattery,
            0x05 => CartridgeType::Mbc2,
            0x06 => CartridgeType::Mbc2Battery,
            0x08 => CartridgeType::RomRam,
            0x09 => CartridgeType::RomRamBattery,
            0x0B => CartridgeType::Mmm01,
            0x0C => CartridgeType::Mmm01Sram,
            0x0D => CartridgeType::Mmm01SramBattery,
            0x0F => CartridgeType::Mbc3TimerBattery,
            0x10 => CartridgeType::Mbc3TimerRamBattery,
            0x11 => CartridgeType::Mbc3,
            0x12 => CartridgeType::Mbc3Ram,
            0x13 => CartridgeType::Mbc3RamBattery,
            0x19 => CartridgeType::Mbc5,
            0x1A => CartridgeType::Mbc5Ram,
            0x1B => CartridgeType::Mbc5RamBattery,
            0x1C => CartridgeType::Mbc5Rumble,
            0x1D => CartridgeType::Mbc5RumbleSram,
            0x1E => CartridgeType::Mbc5RumbleSramBattery,
            0x20 => CartridgeType::Mbc6,
            0x22 => CartridgeType::Mbc7SensorRumbleRamBattery,
            0xFC => CartridgeType::PocketCamera,
            0xFD => CartridgeType::BandaiTama5,
            0xFE => CartridgeType::HuC3,
            0xFF => CartridgeType::HuC1RamBattery,
            _ => panic!("Invalid cartridge type {value}"),
        }
    }
}


pub fn main() {
    env_logger::init();
    let boot_rom = fs::read("boot.gb").expect("Unable to read boot rom");
    let rom = fs::read("test.gb").expect("Unable to read rom");

    let title = rom[0x134..0x143]
        .iter()
        .map(|&c| c as char)
        .collect::<String>();
    println!("Loading {title}...");

    let mbc = rom[0x147];
    let type_ = CartridgeType::from(mbc);
    println!("Memory Bank Controller: {type_:?}");

    
    Cpu::new(boot_rom, rom).run();
}
