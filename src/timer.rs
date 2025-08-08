use log::debug;
use crate::memory::Interrupt;

pub struct Timer {
    div: u8,
    tima: u8,
    tma: u8,
    tac: u8,
    div_countdown: u16,
    timer_countdown: i32,
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            div_countdown: 64 * 4,
            timer_countdown: 0,
        }
    }

    pub fn cycle(&mut self) -> Option<Interrupt> {
        let mut interrupt = None;
        if self.div_countdown == 0 {
            self.div_countdown = 64 * 4;
            self.div = self.div.wrapping_add(1);
        } else {
            self.div_countdown -= 1;
        }
        let timer_enabled = self.tac & 0b100 == 0b100;
        if self.timer_countdown == 0 && timer_enabled {
            // if timer is enabled
            self.tima = self.tima.wrapping_add(1);
            if self.tima == 0 {
                self.tima = self.tma;
                interrupt = Some(Interrupt::Timer);
            }
            let duration = match self.tac & 0b11 {
                0b00 => 256,
                0b01 => 4,
                0b10 => 16,
                0b11 => 64,
                _ => unreachable!(),
            };
            self.timer_countdown = duration * 4;
        }
        if timer_enabled {
            self.timer_countdown -= 1;
        }
        interrupt
    }

    pub fn read(&self, addr: u16) -> u8 {
        debug!("Timer read: {:#X}", addr);
        match addr {
            0xFF04 => self.div,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        debug!("Timer write: {:#X} {:#X}", addr, value);
        match addr {
            0xFF04 => self.div = 0,
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac = value,
            _ => unreachable!(),
        }
    }
}
