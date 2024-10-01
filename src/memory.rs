use log::debug;
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
    pub fn new(boot_rom: &[u8], rom_bank0: &[u8]) -> Self {
        let mut mem = [0; 65536];
        mem[0..boot_rom.len()].copy_from_slice(boot_rom);
        // mem[0..rom_bank0.len()].copy_from_slice(rom_bank0);
        Self {
            mem,
            joypad: Joypad::default(),
        }
    }
    
    pub fn get(&self, addr: u16) -> u8 {
        // debug!("First tile: {:02X?}", &self.mem[0x8000..0x8016]);
        if addr > 0xE000 && addr < 0xFE00 {
            self.mem[addr as usize - 0x2000]
        } else if addr == 0xFF00 {
            self.get_io(addr);
            0x0
        } else {
            self.mem[addr as usize]
        }
    }
    
    fn get_io(&self, addr: u16) {
        debug!("Requested I/O: {:02X?}", addr);
    }
        
    

    pub fn update<F>(&mut self, mut addr: u16, closure: F) where F: FnOnce() -> u8 {
        let result = closure();

        debug!("Updating memory at {:02X?} to {:02X?}", addr, result);
        if addr > 0xE000 && addr < 0xFE00 {
            addr -= 0x2000;
        }
        self.mem[addr as usize] = result;
        if (0xFF00..0xFF80).contains(&addr) {
            self.update_io(addr, result);
        }
    }
    
    fn update_io(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF00 => self.joypad = Joypad::from(value),
            0xFF0F => {
                debug!("Interrupt request: {:02X?}", value);
                if value & 0b0001 != 0 {
                    debug!("V-Blank interrupt requested");
                }
                if value & 0b0010 != 0 {
                    debug!("LCD STAT interrupt requested");
                }
                if value & 0b0100 != 0 {
                    debug!("Timer interrupt requested");
                }
                if value & 0b1000 != 0 {
                    debug!("Serial interrupt requested");
                }
                if value & 0b10000 != 0 {
                    debug!("Joypad interrupt requested");
                }
            }
            0xFFFF => {
                debug!("Interrupt enable: {:02X?}", value);
                if value & 0b0001 != 0 {
                    debug!("V-Blank interrupt enabled");
                }
                if value & 0b0010 != 0 {
                    debug!("LCD STAT interrupt enabled");
                }
                if value & 0b0100 != 0 {
                    debug!("Timer interrupt enabled");
                }
                if value & 0b1000 != 0 {
                    debug!("Serial interrupt enabled");
                }
                if value & 0b10000 != 0 {
                    debug!("Joypad interrupt enabled");
                }
            }
            0xFF40 => {
                debug!("LCDC:");
                if value & 0b10000000 != 0 {
                    debug!("- LCD enabled");
                }
                if value & 0b01000000 != 0 {
                    debug!("- Window tile map display select: 0x9C00-0x9FFF");
                } else {
                    debug!("- Window tile map display select: 0x9800-0x9BFF");
                }
                if value & 0b00100000 != 0 {
                    debug!("- Window enabled");
                }
                if value & 0b00010000 != 0 {
                    debug!("- Tile data select: 0x8000-0x8FFF");
                } else {
                    debug!("- Tile data select: 0x8800-0x97FF");
                }
                if value & 0b00001000 != 0 {
                    debug!("- Background tile map display select: 0x9C00-0x9FFF");
                } else {
                    debug!("- Background tile map display select: 0x9800-0x9BFF");
                }
                if value & 0b00000100 != 0 {
                    debug!("- Sprite size: 8x16");
                } else {
                    debug!("- Sprite size: 8x8");
                }
                if value & 0b00000010 != 0 {
                    debug!("- Sprites enabled");
                }
                if value & 0b00000001 != 0 {
                    debug!("- Background enabled");
                }
            }
            0xFF43 => debug!("Scroll Y: {:02X?}", value),
            0xFF42 => debug!("Scroll X: {:02X?}", value),
            0xFF47 => debug!("Background palette: {:02X?}", value),
            0xFF48 => debug!("Object palette 0: {:02X?}", value),
            0xFF49 => debug!("Object palette 1: {:02X?}", value),
            0xFF4B => debug!("Window X: {:02X?}", value),
            0xFF4A => debug!("Window Y: {:02X?}", value),
            0xFF04 => debug!("Timer divider: {:02X?}", value),
            0xFF05 => debug!("Timer counter: {:02X?}", value),
            0xFF06 => debug!("Timer modulo: {:02X?}", value),
            0xFF07 => debug!("Timer control: {:02X?}", value),
            _ => debug!("Updated I/O: {:02X?} with {:02X?}", addr, value),
        }
    }
    
    fn iter_tiles(&self) -> impl Iterator<Item = &[u8]> {
        (0..384).map(move |i| {
            let addr = 0x8000 + i * 16;
            &self.mem[addr..addr + 16]
        })
    }
    
    pub fn enable_interrupt(&mut self, interrupt: Interrupt, enable: bool) {
        let mask = u8::from(interrupt);
        if enable {
            self.mem[0xFFFF] |= mask;
        } else {
            self.mem[0xFFFF] &= !mask;
        }
    }
    
    pub fn enabled_interrupts(&self) -> u8 {
        self.mem[0xFFFF]
    }
    
    pub fn request_interrupt(&mut self, interrupt: Interrupt) {
        println!("Requesting interrupt: {:?}", interrupt);
        self.mem[0xFF0F] |= u8::from(interrupt);
        println!("Requested interrupts: {:02X?}", self.mem[0xFF0F]);
    }
    
    pub fn requested_interrupts(&self) -> u8 {
        self.mem[0xFF0F]
    }
    
    pub fn clear_requested_interrupt(&mut self, interrupt: Interrupt) {
        self.mem[0xFF0F] &= !u8::from(interrupt);
    }
    
    pub fn increment_divider(&mut self) {
        let mut divider = self.mem[0xFF04];
        divider = divider.wrapping_add(1);
        self.mem[0xFF04] = divider;
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
