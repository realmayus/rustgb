
// pixel processing unit

use std::sync::mpsc::Sender;
use std::thread::sleep;
use std::time::Duration;

enum PpuMode {
    OamScan = 2,
    DrawingPixels = 3,
    HBlank = 0,
    VBlank = 1,
}
impl PpuMode {
    fn next(&self) -> PpuMode {
        match self {
            PpuMode::OamScan => PpuMode::DrawingPixels,
            PpuMode::DrawingPixels => PpuMode::HBlank,
            PpuMode::HBlank => PpuMode::VBlank,
            PpuMode::VBlank => PpuMode::OamScan,
        }
    }
}

pub struct Ppu {
    mode: PpuMode,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            mode: PpuMode::OamScan,
        }
    }

    pub fn start(&mut self) {
        loop {
            match self.mode {
                PpuMode::OamScan => {
                    // OAM scan
                }
                PpuMode::DrawingPixels => {
                    // Drawing pixels
                }
                PpuMode::HBlank => {
                    // HBlank
                }
                PpuMode::VBlank => {
                    // VBlank
                }
            }
            sleep(Duration::from_millis(10));
            self.mode = self.mode.next();
        }
    }
}