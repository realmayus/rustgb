
use bitflags::bitflags;
use std::{fs, thread};
use std::collections::HashSet;
use std::fmt::format;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;
use eframe::egui::{Color32, Context, TextureOptions};
use eframe::{egui, Frame};
use eframe::epaint::TextureHandle;
use log::info;
use rustgb::{CartridgeType, ControlMsg, FrameData};
use rustgb::cpu::Cpu;
use rustgb::joypad::JoypadKey;
use rustgb::memory::{MappedMemory, Mbc, RomOnlyMbc};
use rustgb::ppu::Ppu;
use rustgb::timer::Timer;
use rustgb::ui::FrameHistory;

struct App {
    frame_history: FrameHistory,
    recv_from_cpu: Receiver<FrameData>,
    send_to_cpu: Sender<ControlMsg>,
    texture: Option<TextureHandle>,
    framebuffer: Arc<Mutex<Vec<Color32>>>,
    framebuffer_dirty: Arc<Mutex<bool>>,
    show_vram: bool,
    keys: HashSet<egui::Key>,
}

impl App {
    pub fn new(recv_from_cpu: Receiver<FrameData>, send_to_cpu: Sender<ControlMsg>, framebuffer: Arc<Mutex<Vec<Color32>>>, framebuffer_dirty: Arc<Mutex<bool>>) -> Self {
        Self {
            frame_history: FrameHistory::default(),
            recv_from_cpu,
            send_to_cpu,
            texture: None,
            framebuffer,
            framebuffer_dirty,
            show_vram: false,
            keys: HashSet::new(),
        }
    }

}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        ctx.input(|i| {
            let keys = &i.keys_down;
            let new_keys = keys.difference(&self.keys).collect::<HashSet<_>>();
            let released_keys = self.keys.difference(&keys).collect::<HashSet<_>>();
            if new_keys.contains(&egui::Key::W) {
                self.send_to_cpu.send(ControlMsg::KeyDown(JoypadKey::Up)).unwrap();
            }
            if released_keys.contains(&egui::Key::W) {
                self.send_to_cpu.send(ControlMsg::KeyUp(JoypadKey::Up)).unwrap();
            }
            if new_keys.contains(&egui::Key::A) {
                self.send_to_cpu.send(ControlMsg::KeyDown(JoypadKey::Left)).unwrap();
            }
            if released_keys.contains(&egui::Key::A) {
                self.send_to_cpu.send(ControlMsg::KeyUp(JoypadKey::Left)).unwrap();
            }
            if new_keys.contains(&egui::Key::S) {
                self.send_to_cpu.send(ControlMsg::KeyDown(JoypadKey::Down)).unwrap();
            }
            if released_keys.contains(&egui::Key::S) {
                self.send_to_cpu.send(ControlMsg::KeyUp(JoypadKey::Down)).unwrap();
            }
            if new_keys.contains(&egui::Key::D) {
                self.send_to_cpu.send(ControlMsg::KeyDown(JoypadKey::Right)).unwrap();
            }
            if released_keys.contains(&egui::Key::D) {
                self.send_to_cpu.send(ControlMsg::KeyUp(JoypadKey::Right)).unwrap();
            }
            if new_keys.contains(&egui::Key::ArrowUp) {
                self.send_to_cpu.send(ControlMsg::KeyDown(JoypadKey::A)).unwrap();
            }
            if released_keys.contains(&egui::Key::ArrowUp) {
                self.send_to_cpu.send(ControlMsg::KeyUp(JoypadKey::A)).unwrap();
            }
            if new_keys.contains(&egui::Key::ArrowDown) {
                self.send_to_cpu.send(ControlMsg::KeyDown(JoypadKey::B)).unwrap();
            }
            if released_keys.contains(&egui::Key::ArrowDown) {
                self.send_to_cpu.send(ControlMsg::KeyUp(JoypadKey::B)).unwrap();
            }
            if new_keys.contains(&egui::Key::ArrowRight) {
                self.send_to_cpu.send(ControlMsg::KeyDown(JoypadKey::Start)).unwrap();
            }
            if released_keys.contains(&egui::Key::ArrowRight) {
                self.send_to_cpu.send(ControlMsg::KeyUp(JoypadKey::Start)).unwrap();
            }
            if new_keys.contains(&egui::Key::ArrowLeft) {
                self.send_to_cpu.send(ControlMsg::KeyDown(JoypadKey::Select)).unwrap();
            }
            if released_keys.contains(&egui::Key::ArrowLeft) {
                self.send_to_cpu.send(ControlMsg::KeyUp(JoypadKey::Select)).unwrap();
            }
            self.keys = keys.clone();
        });
        if *self.framebuffer_dirty.lock().unwrap() {
            self.frame_history.on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);
            let img = egui::ColorImage {
                size: [160, 144],
                pixels: self.framebuffer.lock().unwrap().clone(),
            };
            self.texture = Some(ctx.load_texture("framebuffer", img, TextureOptions::NEAREST));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("FPS: {:.1}", self.frame_history.fps()));

                if ui.button("Debug").clicked() {
                    info!("Sending debug message to CPU");
                    self.send_to_cpu.send(ControlMsg::Debug).unwrap();
                }
            });
            if let Some(texture) = &self.texture {
                let img = egui::Image::new(texture).fit_to_exact_size(ui.available_size());
                ui.add(img);
            }
            let initial_show_vram = self.show_vram;
            ui.checkbox(&mut self.show_vram, "Show VRAM");
            if self.show_vram != initial_show_vram {
                self.send_to_cpu.send(ControlMsg::ShowVRam(self.show_vram)).unwrap();
            }
        });
        ctx.request_repaint();
    }
}



pub fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .filter(Some("rustgb::cpu"), log::LevelFilter::Debug)
        .filter(Some("rustgb::memory"), log::LevelFilter::Debug)
        .filter(Some("rustgb::joypad"), log::LevelFilter::Debug)
        // .filter(Some("rustgb::disassembler"), log::LevelFilter::Debug)
        // .filter(Some("rustgb::ppu"), log::LevelFilter::Info)
        .init();


    let server_addr = "127.0.0.1:8585";
    let _server = puffin_http::Server::new(&server_addr).unwrap();
    
    // let boot_rom = fs::read("boot.gb").expect("Unable to read boot rom");
    // let boot_rom = fs::read("gb-test-roms-master/cpu_instrs/individual/04-op r,imm.gb").expect("Unable to read boot rom");
    
    let rom = fs::read("roms/tetris.gb").expect("Unable to read rom");
    // let rom = fs::read("gb-test-roms-master/cpu_instrs/individual/01-special.gb").expect("Unable to read rom");

    let title = rom[0x134..0x143]
        .iter()
        .map(|&c| c as char)
        .collect::<String>();
    info!("Loading {title}...");

    let mbc = rom[0x147];
    let type_ = CartridgeType::from(mbc);
    let mbc = match type_ {
        CartridgeType::RomOnly => {
            RomOnlyMbc::new(rom)
        }
        _ => panic!("Unsupported cartridge type {type_:?}"),
    };
    info!("Memory Bank Controller: {type_:?}");
    
    let (send_from_cpu, recv_from_cpu) = mpsc::channel::<FrameData>();
    let (send_to_cpu, recv_to_cpu) = mpsc::channel::<ControlMsg>();
    let framebuffer = Arc::new(Mutex::new(vec![Color32::BLACK; 160 * 144]));
    let framebuffer_dirty = Arc::new(Mutex::new(false));
    let ppu = Ppu::new(framebuffer.clone(), framebuffer_dirty.clone());
    let timer = Timer::new();
    let mmu = MappedMemory::new(mbc, ppu, timer);
    let mut cpu = Cpu::new(mmu, recv_to_cpu);
    let cpu_handle = thread::spawn(move || cpu.run());
    
    let app = App::new(recv_from_cpu, send_to_cpu.clone(), framebuffer.clone(), framebuffer_dirty.clone());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([512.0, 512.0]),
        vsync: true,
        ..Default::default()
    };
    eframe::run_native(
        format!("rustgb - {title}").as_str(),
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    ).unwrap();
    send_to_cpu.send(ControlMsg::Terminate).unwrap();
    cpu_handle.join().unwrap();
}
