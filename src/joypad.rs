use log::debug;

pub struct Joypad {
    data: u8,
    buttons: u8,
    dpad: u8,
    pub interrupt: u8,
}

#[derive(Copy, Clone, Debug)]
pub enum JoypadKey {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

impl Default for Joypad {
    fn default() -> Self {
        Self::new()
    }
}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad {
            data: 0xFF,
            buttons: 0x0F,
            dpad: 0x0F,
            interrupt: 0,
        }
    }

    pub fn read(&self) -> u8 {
        let out = if self.data & 0x20 == 0 {
            self.buttons
        } else if self.data & 0x10 == 0 {
            self.dpad
        } else {
            0xFF
        };
        debug!("Joypad read out: {:#X}", out);
        out
    }

    pub fn write(&mut self, value: u8) {
        debug!("Joypad write: {:#X}", value);
        self.data = (self.data & 0xCF) | (value & 0x30);
        self.update();
    }

    fn update(&mut self) {
        let old_data = self.data & 0xF;
        let mut new_data = 0xF;

        if self.data & 0x10 == 0 {
            new_data &= self.dpad;
        }
        if self.data & 0x20 == 0 {
            new_data &= self.buttons;
        }
        if old_data == 0xF && new_data != 0xF {
            self.interrupt = 1;
        }

        self.data = (self.data & 0xF0) | new_data;
    }

    pub fn keydown(&mut self, key: JoypadKey) {
        match key {
            JoypadKey::Right => self.dpad &= !(1 << 0),
            JoypadKey::Left => self.dpad &= !(1 << 1),
            JoypadKey::Up => self.dpad &= !(1 << 2),
            JoypadKey::Down => self.dpad &= !(1 << 3),
            JoypadKey::A => self.buttons &= !(1 << 0),
            JoypadKey::B => self.buttons &= !(1 << 1),
            JoypadKey::Select => self.buttons &= !(1 << 2),
            JoypadKey::Start => self.buttons &= !(1 << 3),
        }
        self.update();
    }

    pub fn keyup(&mut self, key: JoypadKey) {
        match key {
            JoypadKey::Right => self.dpad |= 1 << 0,
            JoypadKey::Left => self.dpad |= 1 << 1,
            JoypadKey::Up => self.dpad |= 1 << 2,
            JoypadKey::Down => self.dpad |= 1 << 3,
            JoypadKey::A => self.buttons |= 1 << 0,
            JoypadKey::B => self.buttons |= 1 << 1,
            JoypadKey::Select => self.buttons |= 1 << 2,
            JoypadKey::Start => self.buttons |= 1 << 3,
        }
        self.update();
    }
}
