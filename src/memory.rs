use crate::joypad::Joypad;
use crate::ppu::Ppu;
use crate::serial::Serial;
use crate::timer::Timer;
use crate::{ControlMsg, Flags};
use bitflags::bitflags;
use log::{debug, info, warn};
use std::sync::mpsc::Sender;

#[derive(Default, Copy, Clone, Debug)]
pub struct RegisterPairValue {
    high: u8,
    low: u8,
}

impl RegisterPairValue {
    pub(crate) fn high(&self) -> u8 {
        self.high
    }
    pub(crate) fn low(&self) -> u8 {
        self.low
    }

    pub fn high_mut(&mut self) -> &mut u8 {
        &mut self.high
    }

    pub fn low_mut(&mut self) -> &mut u8 {
        &mut self.low
    }

    pub fn as_u16(&self) -> u16 {
        u16::from_be_bytes([self.high, self.low])
    }

    pub(crate) fn flags(&self) -> Flags {
        Flags::from_bits(self.low()).unwrap()
    }

    pub(crate) fn set_high(&mut self, value: u8) {
        self.high = value;
    }
    pub(crate) fn set_low(&mut self, value: u8) {
        self.low = value;
    }
}

impl From<u16> for RegisterPairValue {
    // RegisterPairValue is in little-endian representation.
    fn from(value: u16) -> Self {
        RegisterPairValue {
            high: (value >> 8) as u8,
            low: value as u8,
        }
    }
}

pub trait Mbc {
    fn new(rom: Vec<u8>) -> Self;
    fn read_rom(&self, addr: u16) -> u8;
    fn read_ram(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}

pub struct RomOnlyMbc {
    rom: Vec<u8>,
}
impl Mbc for RomOnlyMbc {
    fn new(rom: Vec<u8>) -> Self {
        Self { rom }
    }
    fn read_rom(&self, addr: u16) -> u8 {
        self.rom[addr as usize]
    }
    fn read_ram(&self, addr: u16) -> u8 {
        warn!(
            "RomOnlyMbc doesn't have any RAM banks, reading from 0x{:x}",
            addr
        );
        0xff
    }
    fn write(&mut self, _addr: u16, _value: u8) {
        // Do nothing
    }
}

pub struct Mbc1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    enable_ram: bool,
    rom_bank: usize,
    ram_bank: usize,
    num_rambanks: usize,
    num_rombanks: usize,
    banking_mode: bool,
}

impl Mbc for Mbc1 {
    fn new(rom: Vec<u8>) -> Self {
        todo!()
    }

    fn read_rom(&self, addr: u16) -> u8 {
        todo!()
    }

    fn read_ram(&self, addr: u16) -> u8 {
        if !self.enable_ram {
            warn!("RAM is not enabled, reading from 0x{:x}", addr);
            return 0xff;
        }
        if self.banking_mode {
            self.ram[(self.ram_bank * 0x2000) | (addr as usize & 0x1fff)]
        } else {
            self.ram[addr as usize & 0x1fff]
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => {
                self.enable_ram = value & 0x0f == 0x0a;
            }
            0x2000..=0x3fff => {
                self.rom_bank = (value & 0x1f) as usize;
            }
            0x4000..=0x5fff => {
                if self.num_rombanks > 0x20 {
                    panic!("Only at most 0x20 rom banks is supported");
                }
                self.ram_bank = (value & 0x03) as usize;
            }
            0x6000..=0x7fff => {
                self.banking_mode = value & 0x01 == 0x01;
            }
            _ => warn!("[Mbc1] Write to unsupported address 0x{:04X}", addr),
        }
    }
}

pub trait Memory {
    fn get(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);

    fn update<F>(&mut self, addr: u16, closure: F)
    where
        F: FnOnce() -> u8;

    fn cycle(&mut self);

    fn enable_interrupt(&mut self, interrupt: Interrupt, enable: bool);

    fn enabled_interrupts(&self) -> u8;

    fn request_interrupt(&mut self, interrupt: u8);

    fn requested_interrupts(&self) -> u8;
    fn set_requested_interrupts(&mut self, value: u8);

    fn clear_requested_interrupt(&mut self, interrupt: Interrupt);

    fn control_msg(&mut self, msg: ControlMsg) {
        panic!("This memory implementation does not support control messages.")
    }
}

pub struct MappedMemory<MBC: Mbc> {
    mbc: MBC,
    work_ram: [u8; 0x2000],
    high_ram: [u8; 0x7F],
    wram_bank: u8, // 1-7
    joypad: Joypad,
    pub ppu: Ppu,
    pub timer: Timer,
    serial: Serial,
    int_enable: u8,
    int_request: u8,
}

impl<MBC> MappedMemory<MBC>
where
    MBC: Mbc,
{
    pub fn new(mbc: MBC, ppu: Ppu, timer: Timer) -> Self {
        let mut mmu = Self {
            mbc,
            work_ram: [0; 0x2000],
            high_ram: [0; 0x7F],
            wram_bank: 1,
            joypad: Joypad::new(),
            ppu,
            timer,
            serial: Serial::default(),
            int_enable: 0,
            int_request: 0,
        };

        mmu.write(0xFF00, 0xCF); // P1
        mmu.write(0xFF01, 0x00); // SB
        mmu.write(0xFF02, 0x7E); // SC
        mmu.write(0xFF04, 0xAB); // DIV
        mmu.write(0xFF05, 0x00); // TIMA
        mmu.write(0xFF06, 0x00); // TMA
        mmu.write(0xFF07, 0xF8); // TAC
        mmu.write(0xFF0F, 0xE1); // IF
        mmu.write(0xFF10, 0x80); // NR10
        mmu.write(0xFF11, 0xBF); // NR11
        mmu.write(0xFF12, 0xF3); // NR12
        mmu.write(0xFF13, 0xFF); // NR13
        mmu.write(0xFF14, 0xBF); // NR14
        mmu.write(0xFF16, 0x3F); // NR21
        mmu.write(0xFF17, 0x00); // NR22
        mmu.write(0xFF18, 0xFF); // NR23
        mmu.write(0xFF19, 0xBF); // NR24
        mmu.write(0xFF1A, 0x7F); // NR30
        mmu.write(0xFF1B, 0xFF); // NR31
        mmu.write(0xFF1C, 0x9F); // NR32
        mmu.write(0xFF1D, 0xFF); // NR33
        mmu.write(0xFF1E, 0xBF); // NR34
        mmu.write(0xFF20, 0xFF); // NR41
        mmu.write(0xFF21, 0x00); // NR42
        mmu.write(0xFF22, 0x00); // NR43
        mmu.write(0xFF23, 0xBF); // NR44
        mmu.write(0xFF24, 0x77); // NR50
        mmu.write(0xFF25, 0xF3); // NR51
        mmu.write(0xFF26, 0xF1); // NR52
        mmu.write(0xFF40, 0x91); // LCDC
        mmu.write(0xFF41, 0x85); // STAT
        mmu.write(0xFF42, 0x00); // SCY
        mmu.write(0xFF43, 0x00); // SCX
        mmu.write(0xFF45, 0x00); // LYC
        mmu.write(0xFF47, 0xFC); // BGP
        mmu.write(0xFF48, 0xFF); // OBP0
        mmu.write(0xFF49, 0xFF); // OBP1
        mmu.write(0xFF4A, 0x00); // WY
        mmu.write(0xFF4B, 0x00); // WX

        mmu
    }

    fn dma_transfer(&mut self, value: u8) {
        assert!(value <= 0xDF);
        let start = (value as u16) << 8;
        for i in 0..0xa0 {
            let copied = self.get(start + i as u16);
            self.ppu.oam[i] = copied;
        }
    }
}

impl<MBC> Memory for MappedMemory<MBC>
where
    MBC: Mbc,
{
    fn get(&self, addr: u16) -> u8 {
        // debug!("First tile: {:02X?}", &self.mem[0x8000..0x8016]);
        let addr = if addr > 0xE000 && addr < 0xFE00 {
            addr - 0x2000
        } else {
            addr
        };
        match addr {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.mbc.read_rom(addr),
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B | 0xFF68..=0xFF6B => {
                self.ppu.read(addr)
            }
            0xC000..=0xCFFF | 0xE000..=0xEFFF => self.work_ram[(addr - 0xC000) as usize],
            0xD000..=0xDFFF | 0xF000..=0xFDFF => {
                self.work_ram[(self.wram_bank as usize * 0x1000) | addr as usize & 0x0FFF]
            }
            0xFF00 => self.joypad.read(),
            0xFF01..=0xFF02 => self.serial.read(addr),
            0xFF0F => self.requested_interrupts(),
            0xFF04..=0xFF07 => self.timer.read(addr),
            0xFF80..=0xFFFE => self.high_ram[(addr - 0xFF80) as usize],
            0xFFFF => self.enabled_interrupts(),
            _ => panic!("Read from unimplemented memory address: {:02X?}", addr),
        }
    }

    fn write(&mut self, mut addr: u16, value: u8) {
        // debug!("Updating memory at {:02X?} to {:02X?}", addr, result);
        if addr > 0xE000 && addr < 0xFE00 {
            addr -= 0x2000;
        }
        match addr {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.mbc.write(addr, value),
            0xFF46 => self.dma_transfer(value),
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B | 0xFF68..=0xFF6B => {
                self.ppu.write(addr, value)
            }
            0xC000..=0xCFFF | 0xE000..=0xEFFF => self.work_ram[(addr - 0xC000) as usize] = value,
            0xD000..=0xDFFF | 0xF000..=0xFDFF => {
                self.work_ram[(self.wram_bank as usize * 0x1000) | addr as usize & 0x0FFF] = value
            }
            0xFF00 => self.joypad.write(value),
            0xFF01..=0xFF02 => self.serial.write(addr, value),
            0xFF04..=0xFF07 => self.timer.write(addr, value),
            0xFF0F => self.int_request = value,
            0xFF10..=0xFF3F => { /* audio */ }
            0xFF80..=0xFFFE => self.high_ram[(addr - 0xFF80) as usize] = value,
            0xFFFF => {
                println!("Setting interrupt enable to {:08b}", value);
                self.int_enable = value
            }
            0xFEA0..=0xFEFF => { /* Unusable memory */ }
            _ => warn!("Write to unimplemented memory address: {:02X?}", addr),
        }
    }

    fn update<F>(&mut self, addr: u16, closure: F)
    where
        F: FnOnce() -> u8,
    {
        let result = closure();
        self.write(addr, result);
    }

    fn cycle(&mut self) {
        let interrupt1 = self.timer.cycle();
        if let Some(interrupt) = interrupt1 {
            self.request_interrupt(u8::from(interrupt));
        }
        self.ppu.cycle();
        if self.ppu.interrupt != 0 {
            self.request_interrupt(self.ppu.interrupt);
            self.ppu.interrupt = 0;
        }
        if self.joypad.interrupt != 0 {
            // println!("Requesting joypad interrupt");
            self.request_interrupt(self.joypad.interrupt);
            self.joypad.interrupt = 0;
        }
    }

    fn enable_interrupt(&mut self, interrupt: Interrupt, enable: bool) {
        let mask = u8::from(interrupt);
        if enable {
            self.int_enable |= mask;
        } else {
            self.int_enable &= !mask;
        }
    }

    fn enabled_interrupts(&self) -> u8 {
        self.int_enable
    }

    fn request_interrupt(&mut self, interrupt: u8) {
        self.int_request |= interrupt;
    }

    fn requested_interrupts(&self) -> u8 {
        self.int_request
    }

    fn set_requested_interrupts(&mut self, value: u8) {
        self.int_request = value;
    }

    fn clear_requested_interrupt(&mut self, interrupt: Interrupt) {
        self.int_request &= !u8::from(interrupt);
    }

    fn control_msg(&mut self, msg: ControlMsg) {
        println!("Received control message: {:?}", msg);
        match msg {
            ControlMsg::Debug => {
                println!("{:08b}", self.enabled_interrupts())
            }
            ControlMsg::ShowVRam(show) => {
                self.ppu.show_vram = show;
            }
            ControlMsg::KeyDown(key) => self.joypad.keydown(key),
            ControlMsg::KeyUp(key) => self.joypad.keyup(key),
            _ => panic!("Unhandled control message: {:?}", msg),
        }
    }
}

pub struct LinearMemory<const SIZE: usize> {
    mem: [u8; SIZE],
    int_enable: u8,
    int_request: u8,
}

impl<const SIZE: usize> LinearMemory<SIZE> {
    pub fn new() -> Self {
        Self {
            mem: [0; SIZE],
            int_enable: 0,
            int_request: 0,
        }
    }
}

impl<const SIZE: usize> Memory for LinearMemory<SIZE> {
    fn get(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.mem[addr as usize] = value;
    }

    fn update<F>(&mut self, addr: u16, closure: F)
    where
        F: FnOnce() -> u8,
    {
        let result = closure();
        self.write(addr, result);
    }

    fn cycle(&mut self) {}

    fn enable_interrupt(&mut self, interrupt: Interrupt, enable: bool) {
        let mask = u8::from(interrupt);
        if enable {
            self.int_enable |= mask;
        } else {
            self.int_enable &= !mask;
        }
    }
    fn set_requested_interrupts(&mut self, value: u8) {
        self.int_request = value;
    }
    fn enabled_interrupts(&self) -> u8 {
        self.int_enable
    }

    fn request_interrupt(&mut self, interrupt: u8) {
        self.int_request |= interrupt;
    }

    fn requested_interrupts(&self) -> u8 {
        self.int_request
    }

    fn clear_requested_interrupt(&mut self, interrupt: Interrupt) {
        self.int_request &= !u8::from(interrupt);
    }
}

#[derive(Debug)]
pub enum Interrupt {
    VBlank,
    LcdStat,
    Timer,
    Serial,
    Joypad,
}

impl From<Interrupt> for u8 {
    fn from(interrupt: Interrupt) -> u8 {
        match interrupt {
            Interrupt::VBlank => 0b0001,
            Interrupt::LcdStat => 0b0010,
            Interrupt::Timer => 0b0100,
            Interrupt::Serial => 0b1000,
            Interrupt::Joypad => 0b10000,
        }
    }
}

impl From<u8> for Interrupt {
    fn from(value: u8) -> Interrupt {
        match value {
            0b0001 => Interrupt::VBlank,
            0b0010 => Interrupt::LcdStat,
            0b0100 => Interrupt::Timer,
            0b1000 => Interrupt::Serial,
            0b10000 => Interrupt::Joypad,
            _ => panic!("Invalid interrupt value: {:02X?}", value),
        }
    }
}
