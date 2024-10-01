use crate::memory::{Interrupt, Memory};

pub struct Timer {
    div: u8,
    tima: u8,
    tma: u8,
    tac: u8,
    counter: u8,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            counter: 255,
        }
    }
    
    pub fn cycle(&mut self, mem: &mut Memory) {
        if self.counter == 0 {
            self.counter = 255;
            self.div = self.div.wrapping_add(1);
        } else {
            self.counter -= 1;
        }
        let tac = mem.get(0xFF07);
        let increment = match tac & 0b11 {
            0b00 => 1024,
            0b01 => 16,
            0b10 => 64,
            0b11 => 256,
            _ => unreachable!(),
        };
        if tac & 0b100 == 0b100 {  // if timer is enabled
            self.tima = self.tima.wrapping_add(1);
            if self.tima == 0 {
                self.tima = mem.get(0xFF06);
                mem.request_interrupt(Interrupt::Timer);
            }
        }
    }
}