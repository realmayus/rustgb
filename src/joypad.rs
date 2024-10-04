

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
        if self.data & 0x20 == 0 {
            self.buttons
        } else if self.data & 0x10 == 0 {
            self.dpad
        } else {
            0xFF
        }
    }

    pub fn write(&mut self, value: u8) {
        self.data = (self.data & 0xCF) | (value & 0x30);
        self.update();
    }

    fn update(&mut self) {
        let mut new_interrupt = 0;
        if self.data & 0x20 == 0 {
            if self.buttons & 0x0F != 0x0F {
                new_interrupt = 1;
            }
        } else if self.data & 0x10 == 0 {
            if self.dpad & 0x0F != 0x0F {
                new_interrupt = 1;
            }
        }
        self.interrupt = new_interrupt;
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