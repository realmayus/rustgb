use crate::Flags;

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

pub struct Memory {
    mem: [u8; 65536],
    joypad: Joypad,
}

impl Memory {
    pub fn new(rom_bank0: &[u8]) -> Self {
        let mut mem = [0; 65536];
        mem[0..rom_bank0.len()].copy_from_slice(rom_bank0);
        Self {
            mem,
            joypad: Joypad::default(),
        }
    }
    
    pub fn get(&self, addr: u16) -> u8 {
        if addr > 0xE000 && addr < 0xFE00 {
            self.mem[addr as usize - 0x2000]
        } else if addr == 0xFF00 {
            self.joypad.into()
        } else {
            self.mem[addr as usize]
        }
    }

    pub fn update<F>(&mut self, mut addr: u16, closure: F) where F: FnOnce() -> u8 {
        if addr > 0xE000 && addr < 0xFE00 {
            addr -= 0x2000;
        }
        let result = closure();
        self.mem[addr as usize] = result;
        self.update_io(addr, result);
    }
    
    fn update_io(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF00 => self.joypad = Joypad::from(value),
            _ => {},
        }
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Joypad {
    a_right: bool,
    b_left: bool,
    select_up: bool,
    start_down: bool,
    select_direction: bool,
    select_buttons: bool,
}

impl From<u8> for Joypad {
    fn from(value: u8) -> Self {
        Self {
            // Note that, rather unconventionally for the Game Boy, a button being pressed is seen as the corresponding bit being 0, not 1.
            a_right: value & 0b0001 == 0,
            b_left: value & 0b0010 == 0,
            select_up: value & 0b0100 == 0,
            start_down: value & 0b1000 == 0,
            select_direction: value & 0b10000 == 0,
            select_buttons: value & 0b100000 == 0,
        }
    }
}

impl From<Joypad> for u8 {
    fn from(joypad: Joypad) -> Self {
        let mut value = 0b0000;
        if joypad.a_right {
            value |= 0b0001;
        }
        if joypad.b_left {
            value |= 0b0010;
        }
        if joypad.select_up {
            value |= 0b0100;
        }
        if joypad.start_down {
            value |= 0b1000;
        }
        if joypad.select_direction {
            value |= 0b10000;
        }
        if joypad.select_buttons {
            value |= 0b100000;
        }
        value
    }
}
