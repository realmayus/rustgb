
// pixel processing unit

use std::cmp::PartialEq;
use std::sync::mpsc::Sender;
use bitflags::bitflags;
use eframe::egui::Color32;
use log::{debug, error, info};
use crate::FrameData;
use crate::memory::{Interrupt, Mbc, MappedMemory};

bitflags! {
    pub struct LcdStat: u8 {
        const LYC_INT_SEL = 1 << 6;
        const MODE_2_INT_SEL = 1 << 5;
        const MODE_1_INT_SEL = 1 << 4;
        const MODE_0_INT_SEL = 1 << 3;
        const LYC_EQ_LY = 1 << 2;
        const MODE_2 = 0b10;
        const MODE_1 = 0b01;
        const MODE_0 = 0b00;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq,)]
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
            PpuMode::HBlank => PpuMode::OamScan,
            PpuMode::VBlank => PpuMode::VBlank,
        }
    }
}

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

struct Palette {
    id_0: u8,
    id_1: u8,
    id_2: u8,
    id_3: u8,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            id_0: 0,
            id_1: 1,
            id_2: 2,
            id_3: 3,
        }
    }
}

pub struct Ppu {
    mode: PpuMode,
    send: Sender<FrameData>,
    pub mode_counter: usize,
    framebuffer: [u8; 160 * 144 * 4],
    vram: [u8; 0x2000],
    pub(crate) oam: [u8; 0xa0],
    line: u8,
    lyc: u8,

    bg_win_enable: bool,
    obj_enable: bool,
    obj_size: bool,
    bg_tile_map: bool,
    bg_win_tile_data: bool,
    win_enable: bool,
    win_tile_map: bool,
    lcd_enable: bool,

    mode_0_int: bool,
    mode_1_int: bool,
    mode_2_int: bool,
    lyc_int: bool,

    viewport_x: u8,
    viewport_y: u8,

    bg_palette: Palette,
    obj_palette_0: Palette,
    obj_palette_1: Palette,

    window_x: u8,  // window x position plus 7
    window_y: u8,
    tiles: [Tile; 3 * 128],
    tile_map_0: [u8; 0x400],
    tile_map_1: [u8; 0x400],
    hblank: bool,
    vblank: bool,
    win_y_trigger: bool,
    pub(crate) interrupt: u8,
}

// makes it easier to work with tiles, directly converts weird 16 byte format to 8x8 pixel format
struct Tile {
    raw: [u8; 16],
    pixels: [u8; 8*8],
}

impl Tile {
    fn from_raw(raw: [u8; 16]) -> Self {
        Self { raw, pixels: Self::convert_to_pixels(raw)}
    }
    
    fn set_byte(&mut self, index: usize, value: u8) {
        // println!("Setting byte {} to {}", index, value);
        assert!(index < 16, "Tile index out of bounds");
        self.raw[index] = value;
        self.pixels = Self::convert_to_pixels(self.raw);
    }
    
    fn convert_to_pixels(raw: [u8; 16]) -> [u8; 8*8] {
        let mut data = [0; 8*8];
        let mut pixel = 0;
        for line in raw.chunks(2) {
            let [lsb, msb] = line else { unreachable!() };
            for i in 0..8u8 {
                let lsb_bit = (lsb >> i) & 1;
                let msb_bit = (msb >> i) & 1;
                let color = (msb_bit << 1) | lsb_bit;
                data[pixel] = color;
                pixel += 1;
            }
        }
        data
    }
}

impl Ppu {
    pub fn new(send: Sender<FrameData>) -> Self {
        Self {
            mode: PpuMode::HBlank,
            mode_counter: 0,
            framebuffer: [255; 160 * 144 * 4],
            line: 0,
            lyc: 0,
            send,
            bg_win_enable: false,
            obj_enable: false,
            obj_size: false,
            bg_tile_map: false,
            bg_win_tile_data: false,
            win_enable: false,
            win_tile_map: false,
            lcd_enable: false,
            mode_0_int: false,
            mode_1_int: false,
            mode_2_int: false,
            lyc_int: false,
            viewport_x: 0,
            viewport_y: 0,
            bg_palette: Palette::default(),
            obj_palette_0: Palette::default(),
            obj_palette_1: Palette::default(),
            window_x: 0,
            window_y: u8::MAX,
            vram: [0; 0x2000],
            oam: [0; 0xa0],
            win_y_trigger: false,
            tiles: core::array::from_fn(|_| Tile::from_raw([0; 16])),
            interrupt: 0,
            hblank: false,
            vblank: false,
        }
    }

    pub fn cycle(&mut self) {
        self.mode_counter += 1;
        if self.mode_counter >= 114 {  // time it takes to render a scanline
            self.mode_counter -= 114;
            self.line = (self.line + 1) % 154;
            if self.lyc_int && self.line == self.lyc {
                self.interrupt |= u8::from(Interrupt::LcdStat);
            }
            if self.line >= 144 && self.mode != PpuMode::VBlank {
                self.interrupt |= self.set_mode(PpuMode::VBlank);
            }
        }
        
        if self.line < 144 {
            if self.mode_counter <= 20 {
                self.interrupt |= self.set_mode(PpuMode::OamScan);
            } else if self.mode_counter <= 63 {
                self.interrupt |= self.set_mode(PpuMode::DrawingPixels);
            } else if self.mode_counter <= 114 {
                self.interrupt |= self.set_mode(PpuMode::HBlank);
            }
        }
    }
    
    fn set_mode(&mut self, mode: PpuMode) -> u8 {
        self.mode = mode;
        self.vblank = false;
        self.hblank = false;
        match mode {
            PpuMode::OamScan => {
                if self.mode_2_int { u8::from(Interrupt::LcdStat) } else { 0 }
            }
            PpuMode::DrawingPixels => {
                if self.win_enable && !self.win_y_trigger && self.line == self.window_y {
                    self.win_y_trigger = true;
                    self.window_y = u8::MAX;
                }
                0
            }
            PpuMode::HBlank => {
                self.render_scanline();
                self.hblank = true;
                if self.mode_0_int { u8::from(Interrupt::LcdStat) } else { 0 }
            }
            PpuMode::VBlank => {
                self.send
                    .send(FrameData { framebuffer: self.framebuffer.chunks_exact(4)
                    .map(|pixel| Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3])).collect::<Vec<_>>() })
                    .unwrap();
                self.win_y_trigger = false;
                self.vblank = true;
                if self.mode_1_int { u8::from(Interrupt::LcdStat) | u8::from(Interrupt::VBlank) } else { 0 }
            }
        }
    }



    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff40 => {
                let mut reg = 0;
                reg |= (self.lcd_enable as u8) << 7;
                reg |= (self.win_tile_map as u8) << 6;
                reg |= (self.win_enable as u8) << 5;
                reg |= (self.bg_win_tile_data as u8) << 4;
                reg |= (self.bg_tile_map as u8) << 3;
                reg |= (self.obj_size as u8) << 2;
                reg |= (self.obj_enable as u8) << 1;
                reg |= self.bg_win_enable as u8;
                reg
            }
            0xff41 => {
                let mut reg = 0;
                reg |= (self.lyc_int as u8) << 6;
                reg |= (self.mode_2_int as u8) << 5;
                reg |= (self.mode_1_int as u8) << 4;
                reg |= (self.mode_0_int as u8) << 3;
                reg |= (self.lyc_int as u8) << 2;
                reg |= self.mode as u8;
                reg
            }
            0xff42 => self.viewport_y,
            0xff43 => self.viewport_x,
            0xff44 => self.line,
            0xff45 => self.lyc,
            0xff47 => {
                let mut reg = 0;
                reg |= self.bg_palette.id_0 << 0;
                reg |= self.bg_palette.id_1 << 2;
                reg |= self.bg_palette.id_2 << 4;
                reg |= self.bg_palette.id_3 << 6;
                reg
            }
            0xff48 => {
                let mut reg = 0;
                reg |= self.obj_palette_0.id_0 << 0;
                reg |= self.obj_palette_0.id_1 << 2;
                reg |= self.obj_palette_0.id_2 << 4;
                reg |= self.obj_palette_0.id_3 << 6;
                reg
            }
            0xff49 => {
                let mut reg = 0;
                reg |= self.obj_palette_1.id_0 << 0;
                reg |= self.obj_palette_1.id_1 << 2;
                reg |= self.obj_palette_1.id_2 << 4;
                reg |= self.obj_palette_1.id_3 << 6;
                reg
            }
            0xff4a => self.window_y,
            0xff4b => self.window_x,
            0x8000..=0x9fff => self.vram[(addr - 0x8000) as usize],
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],
            x => panic!("PPU read from unimplemented register: {:#06x}", x),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xff40 => {
                let initial_lcd_enable = self.lcd_enable;
                self.lcd_enable = value & 0b10000000 != 0;
                self.win_tile_map = value & 0b01000000 != 0;
                self.win_enable = value & 0b00100000 != 0;
                self.bg_win_tile_data = value & 0b00010000 != 0;
                self.bg_tile_map = value & 0b00001000 != 0;
                self.obj_size = value & 0b00000100 != 0;
                self.obj_enable = value & 0b00000010 != 0;
                self.bg_win_enable = value & 0b00000001 != 0;
                let toggled_lcd_off = initial_lcd_enable && !self.lcd_enable;
                
                if toggled_lcd_off {
                    self.line = 0;
                    self.mode = PpuMode::OamScan;
                    self.win_y_trigger = false;
                    self.mode_counter = 0;
                    self.clear_framebuffer();
                }
            }
            0xff41 => {
                self.lyc_int = value & 0b01000000 != 0;
                self.mode_2_int = value & 0b00100000 != 0;
                self.mode_1_int = value & 0b00010000 != 0;
                self.mode_0_int = value & 0b00001000 != 0;
            }
            0xff42 => {
                self.viewport_y = value },
            0xff43 => {
                self.viewport_x = value },
            0xff45 => {
                self.lyc = value;
                if self.lyc_int && self.line == self.lyc {
                    self.interrupt |= u8::from(Interrupt::LcdStat);
                }
            },
            0xff47 => {
                self.bg_palette.id_0 = value & 0b00000011;
                self.bg_palette.id_1 = (value & 0b00001100) >> 2;
                self.bg_palette.id_2 = (value & 0b00110000) >> 4;
                self.bg_palette.id_3 = (value & 0b11000000) >> 6;
            }
            0xff48 => {
                self.obj_palette_0.id_0 = value & 0b00000011;
                self.obj_palette_0.id_1 = (value & 0b00001100) >> 2;
                self.obj_palette_0.id_2 = (value & 0b00110000) >> 4;
                self.obj_palette_0.id_3 = (value & 0b11000000) >> 6;
            }
            0xff49 => {
                self.obj_palette_1.id_0 = value & 0b00000011;
                self.obj_palette_1.id_1 = (value & 0b00001100) >> 2;
                self.obj_palette_1.id_2 = (value & 0b00110000) >> 4;
                self.obj_palette_1.id_3 = (value & 0b11000000) >> 6;
            }
            0xff4a => {
                self.window_y = value },
            0xff4b => {
                self.window_x = value },
            0x8000..=0x97ff => {
                self.tiles[(addr - 0x8000) as usize / 16].set_byte((addr - 0x8000) as usize % 16, value) },
            0x9800..=0x9bff => 
                self.tile_map_0[(addr - 0x9800) as usize] = value,
            0x9c00..=0x9fff => 
                self.tile_map_1[(addr - 0x9c00) as usize] = value,
            
            0xFE00..=0xFE9F => {
                self.oam[(addr - 0xFE00) as usize] = value },
            _ => unimplemented!("PPU write to unimplemented register: {:#06x}", addr),
        }
    }

    // we need it in rgba for egui to display (even though game boy doesn't support rgba, obviously)
    pub fn set_pixel_rgba(&mut self, x: u8, r: u8, g: u8, b: u8, a: u8) {
        self.framebuffer[(self.line as usize * SCREEN_WIDTH * 4) + x as usize * 4] = r;
        self.framebuffer[(self.line as usize * SCREEN_WIDTH * 4) + x as usize * 4 + 1] = g;
        self.framebuffer[(self.line as usize * SCREEN_WIDTH * 4) + x as usize * 4 + 2] = b;
        self.framebuffer[(self.line as usize * SCREEN_WIDTH * 4) + x as usize * 4 + 3] = a;
    }
    pub fn render_scanline(&mut self) {
        self.render_background();
        self.dump_vram();
        // self.render_sprites();
    }

    fn render_background(&mut self) {
        if self.line % 2 == 0 {
            for x in 0..SCREEN_WIDTH {
                let color = Color32::from_rgba_unmultiplied(96, 96, 96, 255);
                self.set_pixel_rgba(x as u8, color.r(), color.g(), color.b(), color.a());
            }
        }
    }
    
    fn clear_framebuffer(&mut self) {
        let initial_line = self.line;
        for y in 0..SCREEN_HEIGHT {
            self.line = y as u8;
            for x in 0..SCREEN_WIDTH {
                self.set_pixel_rgba(x as u8, 0, 0, 0, 255);
            }
        }
        self.line = initial_line;
    }

    // render all loaded sprites on the current scanline
    fn dump_vram(&mut self) {
        // tiles are 8x8. we are rendering just a single scanline in this function. lay out all tiles in a row, wrapping to the next row
        for tile_x in 0..20 { // can fit 20 tiles on screen
            let tile_y = self.line / 8;
            let tile_index = tile_y as usize * 20 + tile_x;
            let tile = &self.tiles[tile_index];
            let pixels = tile.pixels;
            let tile_line = self.line % 8;
            for x in 0..8 {
                let pixel = pixels[tile_line as usize * 8 + x as usize];
                let color = match pixel {
                    0 => Color32::from_rgba_unmultiplied(255, 255, 255, 255),
                    1 => Color32::from_rgba_unmultiplied(192, 192, 192, 255),
                    2 => Color32::from_rgba_unmultiplied(96, 96, 96, 255),
                    3 => Color32::from_rgba_unmultiplied(0, 0, 0, 255),
                    _ => unreachable!(),
                };
                self.set_pixel_rgba(tile_x as u8 * 8 + x as u8, color.r(), color.g(), color.b(), color.a());
            }
        }
    }
    
    fn render_sprites() {
        
    }
}