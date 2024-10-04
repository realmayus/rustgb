use crate::memory::{Interrupt, MappedMemory};

pub struct Timer {
    div: u8,
    tima: u8,
    tma: u8,
    tac: u8,
    div_countdown: u8,
    timer_countdown: i32,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            div_countdown: 255,
            timer_countdown: 0,
        }
    }
    
    pub fn cycle(&mut self) -> Option<Interrupt> {
        let mut interrupt = None;
        if self.div_countdown == 0 {
            self.div_countdown = 255;
            self.div = self.div.wrapping_add(1);
        } else {
            self.div_countdown -= 1;
        }
        let timer_enabled = self.tac & 0b100 == 0b100;
        if self.timer_countdown == 0 && timer_enabled {  // if timer is enabled
            self.tima = self.tima.wrapping_add(1);
            if self.tima == 0 {
                self.tima = self.tma;
                interrupt = Some(Interrupt::Timer);
            }
        }
        let duration = match self.tac & 0b11 {
            0b00 => 1024,
            0b01 => 16,
            0b10 => 64,
            0b11 => 256,
            _ => unreachable!(),
        };
        self.timer_countdown = duration;
        interrupt
    }
    
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF04 => self.div,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,
            _ => unreachable!(),
        }
    }
    
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF04 => self.div = value,
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac = value,
            _ => unreachable!(),
        }
    }
}