
use bitflags::bitflags;
use std::{fs, thread};
use std::fmt::format;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::Instant;
use eframe::egui::{Color32, Context, TextureOptions};
use eframe::{egui, Frame};
use eframe::epaint::TextureHandle;
use rustgb::{CartridgeType, ControlMsg, FrameData};
use rustgb::cpu::Cpu;
use rustgb::memory::{MappedMemory, Mbc, RomOnlyMbc};
use rustgb::ppu::Ppu;
use rustgb::timer::Timer;
use rustgb::ui::FrameHistory;

struct App {
    frame_history: FrameHistory,
    recv_from_cpu: Receiver<FrameData>,
    texture: Option<TextureHandle>,
}

impl App {
    pub fn new(recv_from_cpu: Receiver<FrameData>) -> Self {
        Self {
            frame_history: FrameHistory::default(),
            recv_from_cpu,
            texture: None,
        }
    }

}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        if let Ok(frame_data) = self.recv_from_cpu.try_recv() {
            self.frame_history.on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);
            let img = egui::ColorImage {
                size: [160, 144],
                pixels: frame_data.framebuffer,
            };
            self.texture = Some(ctx.load_texture("framebuffer", img, TextureOptions::NEAREST));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("FPS: ".to_string() + &self.frame_history.fps().to_string());
            if let Some(texture) = &self.texture {
                let img = egui::Image::new(texture).fit_to_exact_size(ui.available_size());
                ui.add(img);
            }
        });
        ctx.request_repaint();
    }
}



pub fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        // .filter(Some("rustgb::disassembler"), log::LevelFilter::Debug)
        // .filter(Some("rustgb::ppu"), log::LevelFilter::Info)
        .init();
    // let boot_rom = fs::read("boot.gb").expect("Unable to read boot rom");
    // let boot_rom = fs::read("gb-test-roms-master/cpu_instrs/individual/04-op r,imm.gb").expect("Unable to read boot rom");
    
    let rom = fs::read("roms/tetris.gb").expect("Unable to read rom");

    let title = rom[0x134..0x143]
        .iter()
        .map(|&c| c as char)
        .collect::<String>();
    println!("Loading {title}...");

    let mbc = rom[0x147];
    let type_ = CartridgeType::from(mbc);
    let mbc = match type_ {
        CartridgeType::RomOnly => {
            RomOnlyMbc::new(rom)
        }
        _ => panic!("Unsupported cartridge type {type_:?}"),
    };
    println!("Memory Bank Controller: {type_:?}");
    
    let (send_from_cpu, recv_from_cpu) = mpsc::channel::<FrameData>();
    let (send_to_cpu, recv_to_cpu) = mpsc::channel::<ControlMsg>();
    let ppu = Ppu::new(send_from_cpu);
    let timer = Timer::new();
    let mmu = MappedMemory::new(mbc, ppu, timer);
    let mut cpu = Cpu::new(mmu, recv_to_cpu);
    let cpu_handle = thread::spawn(move || {
        loop {
            if let Ok(ControlMsg::Terminate) = cpu.recv.try_recv() {
                println!("Terminating CPU thread");
                break;
            }
            cpu.cycle();
        }
    });
    
    let app = App::new(recv_from_cpu);
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
