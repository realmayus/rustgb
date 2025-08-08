use log::debug;

#[derive(Default)]
pub struct Serial {
    data: u8,
    control: u8,
}

impl Serial {
    pub fn write(&mut self, addr: u16, value: u8) {
        debug!("Serial write: addr=0x{:X}, value=0x{:X}", addr, value);
        match addr {
            0xFF01 => self.data = value,
            0xFF02 => self.control = value,
            _ => panic!("Invalid serial address: 0x{:X}", addr),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        debug!("Serial read: addr=0x{:X}", addr);
        match addr {
            0xFF01 => self.data,
            0xFF02 => self.control | 0b01111110,
            _ => panic!("Invalid serial address: 0x{:X}", addr),
        }
    }
}
