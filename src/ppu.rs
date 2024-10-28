// pixel processing unit

use crate::memory::{Interrupt, MappedMemory, Mbc};
use crate::FrameData;
use bitflags::bitflags;
use eframe::egui::debug_text::print;
use eframe::egui::Color32;
use log::{debug, error, info};
use std::cmp::PartialEq;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

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

macro_rules! set_pixel {
    ($self:ident, $x:expr, $r:expr, $g:expr, $b:expr, $a:expr) => {
        $self.framebuffer[($self.line as usize * SCREEN_WIDTH * 4) + $x as usize * 4] = $r;
        $self.framebuffer[($self.line as usize * SCREEN_WIDTH * 4) + $x as usize * 4 + 1] = $g;
        $self.framebuffer[($self.line as usize * SCREEN_WIDTH * 4) + $x as usize * 4 + 2] = $b;
        $self.framebuffer[($self.line as usize * SCREEN_WIDTH * 4) + $x as usize * 4 + 3] = $a;
    };
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    pub show_vram: bool,
    mode: PpuMode,
    pub mode_counter: usize,
    framebuffer: [u8; 160 * 144 * 4],
    vram: [u8; 0x2000],
    pub(crate) oam: [u8; 0xa0],
    line: u8,
    lyc: u8,

    bg_win_enable: bool,
    obj_enable: bool,
    obj_size: u8,
    bg_tile_map: bool,
    bg_win_tile_data: bool,
    win_enable: bool,
    win_tile_map: bool,
    lcd_enable: bool,

    mode_0_int: bool,
    mode_1_int: bool,
    mode_2_int: bool,
    lyc_int: bool,

    viewport_x: u8, // SCX
    viewport_y: u8, // SCY

    bg_palette: Palette,
    obj_palette_0: Palette,
    obj_palette_1: Palette,

    window_x: u8, // window x position plus 7
    window_y: u8,
    tiles: [Tile; 3 * 128],
    tile_map_0: [u8; 0x400],
    tile_map_1: [u8; 0x400],
    hblank: bool,
    vblank: bool,
    win_y_trigger: bool,
    pub(crate) interrupt: u8,
    pub displaybuffer: Arc<Mutex<Vec<Color32>>>,
    pub displaybuffer_dirty: Arc<Mutex<bool>>,
}

// makes it easier to work with tiles, directly converts weird 16 byte format to 8x8 pixel format
struct Tile {
    raw: [u8; 16],
    pixels: [u8; 8 * 8],
}

impl Tile {
    fn from_raw(raw: [u8; 16]) -> Self {
        Self {
            raw,
            pixels: Self::convert_to_pixels(raw),
        }
    }

    fn set_byte(&mut self, index: usize, value: u8) {
        // println!("Setting byte {} to {}", index, value);
        assert!(index < 16, "Tile index out of bounds");
        self.raw[index] = value;
        self.pixels = Self::convert_to_pixels(self.raw);
    }

    fn convert_to_pixels(raw: [u8; 16]) -> [u8; 8 * 8] {
        let mut data = [0; 8 * 8];
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
    pub fn new(
        displaybuffer: Arc<Mutex<Vec<Color32>>>,
        displaybuffer_dirty: Arc<Mutex<bool>>,
    ) -> Self {
        Self {
            show_vram: false,
            mode: PpuMode::HBlank,
            mode_counter: 0,
            framebuffer: [255; 160 * 144 * 4],
            displaybuffer,
            displaybuffer_dirty,
            line: 0,
            lyc: 0,
            bg_win_enable: false,
            obj_enable: false,
            obj_size: 8,
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
            tile_map_0: [0; 0x400],
            tile_map_1: [0; 0x400],
            interrupt: 0,
            hblank: false,
            vblank: false,
        }
    }

    pub fn cycle(&mut self) {
        puffin::profile_function!();
        self.mode_counter += 1;
        if self.mode_counter >= 114 {
            // time it takes to render a scanline
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
                if self.mode != PpuMode::OamScan {
                    self.interrupt |= self.set_mode(PpuMode::OamScan);
                }
            } else if self.mode_counter <= 63 {
                if self.mode != PpuMode::DrawingPixels {
                    self.interrupt |= self.set_mode(PpuMode::DrawingPixels);
                }
            } else if self.mode != PpuMode::HBlank {
                self.interrupt |= self.set_mode(PpuMode::HBlank);
            }
        }
    }

    fn set_mode(&mut self, mode: PpuMode) -> u8 {
        puffin::profile_function!();
        self.mode = mode;
        self.vblank = false;
        self.hblank = false;
        match mode {
            PpuMode::OamScan => {
                if self.mode_2_int {
                    u8::from(Interrupt::LcdStat)
                } else {
                    0
                }
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
                if self.mode_0_int {
                    u8::from(Interrupt::LcdStat)
                } else {
                    0
                }
            }
            PpuMode::VBlank => {
                self.post_frame();
                self.win_y_trigger = false;
                self.vblank = true;
                if self.mode_1_int {
                    u8::from(Interrupt::LcdStat) | u8::from(Interrupt::VBlank)
                } else {
                    u8::from(Interrupt::VBlank)
                }
            }
        }
    }

    fn post_frame(&mut self) {
        puffin::profile_function!();
        *self.displaybuffer.lock().unwrap() = self
            .framebuffer
            .chunks_exact(4)
            .map(|pixel| Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]))
            .collect::<Vec<_>>();
        *self.displaybuffer_dirty.lock().unwrap() = true;
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
                reg |= ((self.obj_size != 8) as u8) << 2;
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
                let initial_bg_win_enable = self.bg_win_enable;
                self.lcd_enable = value & 0b10000000 != 0;
                self.win_tile_map = value & 0b01000000 != 0;
                self.win_enable = value & 0b00100000 != 0;
                self.bg_win_tile_data = value & 0b00010000 != 0;
                self.bg_tile_map = value & 0b00001000 != 0;
                self.obj_size = if value & 0b00000100 != 0 { 16 } else { 8 };
                self.obj_enable = value & 0b00000010 != 0;
                self.bg_win_enable = value & 0b00000001 != 0;
                let toggled_lcd_off = initial_lcd_enable && !self.lcd_enable;
                let toggled_lcd_on = !initial_lcd_enable && self.lcd_enable;
                if toggled_lcd_off {
                    info!("LCD turned off");
                    self.line = 0;
                    self.mode = PpuMode::OamScan;
                    self.win_y_trigger = false;
                    self.mode_counter = 0;
                    self.clear_framebuffer(0);
                }
                if toggled_lcd_on {
                    info!("LCD turned on");
                }
                if self.bg_win_enable && !initial_bg_win_enable {
                    info!("Background and window rendering enabled");
                }
                if !self.bg_win_enable && initial_bg_win_enable {
                    info!("Background and window rendering disabled");
                }
            }
            0xff41 => {
                self.lyc_int = value & 0b01000000 != 0;
                self.mode_2_int = value & 0b00100000 != 0;
                self.mode_1_int = value & 0b00010000 != 0;
                self.mode_0_int = value & 0b00001000 != 0;
            }
            0xff42 => self.viewport_y = value,
            0xff43 => self.viewport_x = value,
            0xff45 => {
                self.lyc = value;
                if self.lyc_int && self.line == self.lyc {
                    self.interrupt |= u8::from(Interrupt::LcdStat);
                }
            }
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
            0xff4a => self.window_y = value,
            0xff4b => self.window_x = value,
            0x8000..=0x97ff => self.tiles[(addr - 0x8000) as usize / 16]
                .set_byte((addr - 0x8000) as usize % 16, value),
            0x9800..=0x9bff => self.tile_map_0[(addr - 0x9800) as usize] = value,
            0x9c00..=0x9fff => self.tile_map_1[(addr - 0x9c00) as usize] = value,

            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = value,
            _ => unimplemented!("PPU write to unimplemented register: {:#06x}", addr),
        }
    }
    pub fn render_scanline(&mut self) {
        puffin::profile_function!();
        self.clear_scanline(0);
        if self.show_vram {
            self.dump_vram();
            return;
        }
        if self.bg_win_enable {
            self.render_background();
        } else {
            self.clear_scanline(255);
        }
        if self.obj_enable {
            self.render_objects();
        }
        // self.dump_vram();
        // self.render_sprites();
    }

    fn render_background(&mut self) {
        puffin::profile_function!();
        let tilemap = if self.bg_tile_map {
            &self.tile_map_1
        } else {
            &self.tile_map_0
        };
        let scx = self.viewport_x;
        let scy = self.viewport_y;

        let tilemap_line = self.line as usize / 8;
        for (i, tile_id) in tilemap[tilemap_line * 32..(tilemap_line + 1) * 32]
            .iter()
            .enumerate()
        {
            puffin::profile_scope!("Render bg tile");
            let tile_index = if self.bg_win_tile_data {
                *tile_id as usize
            } else {
                (0x100 + *tile_id as i8 as i16) as usize
            };
            let tile = self.tiles[tile_index].pixels;
            for x in 0..8 {
                let tile_line = scy as usize + self.line as usize % 8;
                let color = match tile[tile_line * 8 + x] {
                    0 => Color32::from_rgba_unmultiplied(255, 255, 255, 255),
                    1 => Color32::from_rgba_unmultiplied(192, 192, 192, 255),
                    2 => Color32::from_rgba_unmultiplied(96, 96, 96, 255),
                    3 => Color32::from_rgba_unmultiplied(0, 0, 0, 255),
                    _ => unreachable!(),
                };
                let draw_at = scx as usize + i * 8 + (8 - x);
                if draw_at < SCREEN_WIDTH {
                    set_pixel!(
                        self,
                        draw_at as u8,
                        color.r(),
                        color.g(),
                        color.b(),
                        color.a()
                    );
                }
            }
        }
    }

    fn clear_framebuffer(&mut self, color: u8) {
        puffin::profile_function!();
        self.framebuffer = [color; 160 * 144 * 4];
    }

    fn clear_scanline(&mut self, color: u8) {
        puffin::profile_function!();
        self.framebuffer
            [self.line as usize * SCREEN_WIDTH * 4..(self.line as usize + 1) * SCREEN_WIDTH * 4]
            .fill(color);
    }

    // render all loaded sprites on the current scanline
    fn dump_vram(&mut self) {
        puffin::profile_function!();
        // tiles are 8x8. we are rendering just a single scanline in this function. lay out all tiles in a row, wrapping to the next row
        for tile_x in 0..20 {
            // can fit 20 tiles on screen
            let tile_y = self.line / 8;
            let tile_index = tile_y as usize * 20 + tile_x;
            let tile = &self.tiles[tile_index];
            let pixels = tile.pixels;
            for x in 0..8 {
                let pixel = pixels[self.line as usize % 8 * 8 + x as usize];
                let color = match pixel {
                    0 => Color32::from_rgba_unmultiplied(255, 255, 255, 255),
                    1 => Color32::from_rgba_unmultiplied(192, 192, 192, 255),
                    2 => Color32::from_rgba_unmultiplied(96, 96, 96, 255),
                    3 => Color32::from_rgba_unmultiplied(0, 0, 0, 255),
                    _ => unreachable!(),
                };
                set_pixel!(
                    self,
                    tile_x as u8 * 8 + x as u8,
                    color.r(),
                    color.g(),
                    color.b(),
                    color.a()
                );
            }
        }
    }

    // renders all sprites on the current scanline
    fn render_objects(&mut self) {
        puffin::profile_function!();
        // all objects that are visible on the current scanline
        let mut draw = self
            .oam
            .chunks_exact(4)
            .filter(|obj| {
                let y = obj[0] as i32 - 16;
                let x = obj[1] as i32 - 8;
                y <= self.line as i32 && y + self.obj_size as i32 > self.line as i32
            })
            .collect::<Vec<_>>();
        // draw.sort_by_key(|obj| obj[1]);  // todo do we need to sort this?
        for obj in draw {
            puffin::profile_scope!("Render sprite");
            let x = obj[1] as i32 - 8; // sprite's position on screen
            let y = obj[0] as i32 - 16;
            let tile_id = obj[2];
            let flags = obj[3];
            let palette = flags & 0b00010000 != 0;
            let flip_x = flags & 0b00100000 != 0;
            let flip_y = flags & 0b01000000 != 0;
            let priority = flags & 0b10000000 == 0; // 1 is above background
            let tile = self.tiles[tile_id as usize].pixels;
            for i in 0..8 {
                let i = if flip_x { 7 - i } else { i };
                let sprite_line = self.line as i32 - y;
                let line = if flip_y { 7 - sprite_line } else { sprite_line };
                let pixel = tile[line as usize * 8 + i as usize];
                if pixel == 0 {
                    continue;
                }
                let color = match pixel {
                    0 => Color32::from_rgba_unmultiplied(255, 255, 255, 255),
                    1 => Color32::from_rgba_unmultiplied(192, 192, 192, 255),
                    2 => Color32::from_rgba_unmultiplied(96, 96, 96, 255),
                    3 => Color32::from_rgba_unmultiplied(0, 0, 0, 255),
                    _ => unreachable!(),
                };
                set_pixel!(
                    self,
                    x.wrapping_add(8 - i),
                    color.r(),
                    color.g(),
                    color.b(),
                    color.a()
                );
            }
        }
    }

    pub(crate) fn debug(&self) {
        // print tilemap as matrix
        for y in 0..32 {
            for x in 0..32 {
                print!("{:02X} ", self.tile_map_0[y * 32 + x]);
            }
            print!("\n");
        }
    }
}
