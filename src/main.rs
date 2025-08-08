use eframe::egui::{Color32, Context, TextureOptions};
use eframe::epaint::TextureHandle;
use eframe::{egui, Frame};
use log::info;
use rustgb::cpu::Cpu;
use rustgb::joypad::JoypadKey;
use rustgb::memory::{MappedMemory, Mbc, RomOnlyMbc};
use rustgb::ppu::Ppu;
use rustgb::timer::Timer;
use rustgb::ui::{App, FrameHistory};
use rustgb::{CartridgeType, ControlMsg, FrameData};
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::{fs, thread};

pub fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .filter(Some("rustgb::cpu"), log::LevelFilter::Info)
        .filter(Some("rustgb::memory"), log::LevelFilter::Debug)
        // .filter(Some("rustgb::serial"), log::LevelFilter::Debug)
        .filter(Some("rustgb::timer"), log::LevelFilter::Debug)
        // .filter(Some("rustgb::joypad"), log::LevelFilter::Debug)
        // .filter(Some("rustgbs::disassembler"), log::LevelFilter::Debug)
        // .filter(Some("rustgb::ppu"), log::LevelFilter::Info)
        .init();

    let server_addr = "127.0.0.1:8585";
    let _server = puffin_http::Server::new(server_addr).unwrap();

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
        CartridgeType::RomOnly => RomOnlyMbc::new(rom),
        _ => panic!("Unsupported cartridge type {type_:?}"),
    };
    info!("Memory Bank Controller: {type_:?}");

    let (send_from_cpu, recv_from_cpu) = mpsc::channel::<FrameData>();
    let (send_to_cpu, recv_to_cpu) = mpsc::channel::<ControlMsg>();
    
    
    let framebuffer = Arc::new(Mutex::new(vec![Color32::BLACK; 160 * 144]));
    let framebuffer_dirty = Arc::new(Mutex::new(false));
    let debug_framebuffer = Arc::new(Mutex::new(vec![Color32::BLACK; 160 * 144]));
    let debug_framebuffer_dirty = Arc::new(Mutex::new(false));
    
    let ppu = Ppu::new(framebuffer.clone(), debug_framebuffer.clone(), framebuffer_dirty.clone(), debug_framebuffer_dirty.clone());
    let timer = Timer::new();
    let mmu = MappedMemory::new(mbc, ppu, timer);
    let mut cpu = Cpu::new(mmu, recv_to_cpu);
    let cpu_handle = thread::spawn(move || cpu.run());

    let app = App::new(
        recv_from_cpu,
        send_to_cpu.clone(),
        framebuffer.clone(),
        debug_framebuffer.clone(),
        framebuffer_dirty.clone(),
        debug_framebuffer_dirty.clone(),
    );
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([512.0, 780.0]),
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
    )
    .unwrap();
    send_to_cpu.send(ControlMsg::Terminate).unwrap();
    cpu_handle.join().unwrap();
}
