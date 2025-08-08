use eframe::egui::util::History;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use eframe::egui::{Color32, Context, TextureHandle, TextureOptions};
use std::collections::HashSet;
use eframe::{egui, Frame};
use log::info;
use crate::{ControlMsg, FrameData};
use crate::joypad::JoypadKey;

pub struct FrameHistory {
    frame_times: History<f32>,
}

impl Default for FrameHistory {
    fn default() -> Self {
        let max_age: f32 = 1.0;
        let max_len = (max_age * 300.0).round() as usize;
        Self {
            frame_times: History::new(0..max_len, max_age),
        }
    }
}

impl FrameHistory {
    // Called first
    pub fn on_new_frame(&mut self, now: f64, previous_frame_time: Option<f32>) {
        let previous_frame_time = previous_frame_time.unwrap_or_default();
        if let Some(latest) = self.frame_times.latest_mut() {
            *latest = previous_frame_time; // rewrite history now that we know
        }
        self.frame_times.add(now, previous_frame_time); // projected
    }

    pub fn mean_frame_time(&self) -> f32 {
        self.frame_times.average().unwrap_or_default()
    }

    pub fn fps(&self) -> f32 {
        1.0 / self.frame_times.mean_time_interval().unwrap_or_default()
    }
}

pub struct App {
    frame_history: FrameHistory,
    recv_from_cpu: Receiver<FrameData>,
    send_to_cpu: Sender<ControlMsg>,
    texture: Option<TextureHandle>,
    debug_texture: Option<TextureHandle>,
    framebuffer: Arc<Mutex<Vec<Color32>>>,
    framebuffer_dirty: Arc<Mutex<bool>>,
    keys: HashSet<egui::Key>,
    debug_framebuffer: Arc<Mutex<Vec<Color32>>>,
    debug_framebuffer_dirty: Arc<Mutex<bool>>,
}

impl App {
    pub fn new(
        recv_from_cpu: Receiver<FrameData>,
        send_to_cpu: Sender<ControlMsg>,
        framebuffer: Arc<Mutex<Vec<Color32>>>,
        debug_framebuffer: Arc<Mutex<Vec<Color32>>>,
        framebuffer_dirty: Arc<Mutex<bool>>,
        debug_framebuffer_dirty: Arc<Mutex<bool>>,
    ) -> Self {
        Self {
            frame_history: FrameHistory::default(),
            recv_from_cpu,
            send_to_cpu,
            texture: None,
            debug_texture: None,
            framebuffer,
            debug_framebuffer,
            framebuffer_dirty,
            debug_framebuffer_dirty,
            keys: HashSet::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        ctx.input(|i| {
            let keys = &i.keys_down;
            let new_keys = keys.difference(&self.keys).collect::<HashSet<_>>();
            let released_keys = self.keys.difference(keys).collect::<HashSet<_>>();
            if new_keys.contains(&egui::Key::W) {
                self.send_to_cpu
                    .send(ControlMsg::KeyDown(JoypadKey::Up))
                    .unwrap();
            }
            if released_keys.contains(&egui::Key::W) {
                self.send_to_cpu
                    .send(ControlMsg::KeyUp(JoypadKey::Up))
                    .unwrap();
            }
            if new_keys.contains(&egui::Key::A) {
                self.send_to_cpu
                    .send(ControlMsg::KeyDown(JoypadKey::Left))
                    .unwrap();
            }
            if released_keys.contains(&egui::Key::A) {
                self.send_to_cpu
                    .send(ControlMsg::KeyUp(JoypadKey::Left))
                    .unwrap();
            }
            if new_keys.contains(&egui::Key::S) {
                self.send_to_cpu
                    .send(ControlMsg::KeyDown(JoypadKey::Down))
                    .unwrap();
            }
            if released_keys.contains(&egui::Key::S) {
                self.send_to_cpu
                    .send(ControlMsg::KeyUp(JoypadKey::Down))
                    .unwrap();
            }
            if new_keys.contains(&egui::Key::D) {
                self.send_to_cpu
                    .send(ControlMsg::KeyDown(JoypadKey::Right))
                    .unwrap();
            }
            if released_keys.contains(&egui::Key::D) {
                self.send_to_cpu
                    .send(ControlMsg::KeyUp(JoypadKey::Right))
                    .unwrap();
            }
            if new_keys.contains(&egui::Key::ArrowUp) {
                self.send_to_cpu
                    .send(ControlMsg::KeyDown(JoypadKey::A))
                    .unwrap();
            }
            if released_keys.contains(&egui::Key::ArrowUp) {
                self.send_to_cpu
                    .send(ControlMsg::KeyUp(JoypadKey::A))
                    .unwrap();
            }
            if new_keys.contains(&egui::Key::ArrowDown) {
                self.send_to_cpu
                    .send(ControlMsg::KeyDown(JoypadKey::B))
                    .unwrap();
            }
            if released_keys.contains(&egui::Key::ArrowDown) {
                self.send_to_cpu
                    .send(ControlMsg::KeyUp(JoypadKey::B))
                    .unwrap();
            }
            if new_keys.contains(&egui::Key::ArrowRight) {
                self.send_to_cpu
                    .send(ControlMsg::KeyDown(JoypadKey::Start))
                    .unwrap();
            }
            if released_keys.contains(&egui::Key::ArrowRight) {
                self.send_to_cpu
                    .send(ControlMsg::KeyUp(JoypadKey::Start))
                    .unwrap();
            }
            if new_keys.contains(&egui::Key::ArrowLeft) {
                self.send_to_cpu
                    .send(ControlMsg::KeyDown(JoypadKey::Select))
                    .unwrap();
            }
            if released_keys.contains(&egui::Key::ArrowLeft) {
                self.send_to_cpu
                    .send(ControlMsg::KeyUp(JoypadKey::Select))
                    .unwrap();
            }
            self.keys = keys.clone();
        });
        if *self.framebuffer_dirty.lock().unwrap() {
            self.frame_history
                .on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);
            let img = egui::ColorImage {
                size: [160, 144],
                pixels: self.framebuffer.lock().unwrap().clone(),
            };
            self.texture = Some(ctx.load_texture("framebuffer", img, TextureOptions::NEAREST));
        }
        if *self.debug_framebuffer_dirty.lock().unwrap() {
            let img = egui::ColorImage {
                size: [160, 144],
                pixels: self.debug_framebuffer.lock().unwrap().clone(),
            };
            self.debug_texture = Some(ctx.load_texture("debug_framebuffer", img, TextureOptions::NEAREST));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("FPS: {:.1}", self.frame_history.fps()));

                if ui.button("Debug").clicked() {
                    info!("Sending debug message to CPU");
                    self.send_to_cpu.send(ControlMsg::Debug).unwrap();
                }
                
                if ui.button("Reset").clicked() {
                    info!("Sending reset message to CPU");
                    self.send_to_cpu.send(ControlMsg::Reset).unwrap();
                }
            });
            if let Some(texture) = &self.texture {
                let img = egui::Image::new(texture).fit_to_exact_size(ui.available_size());
                ui.add(img);
            }
            ui.separator();
            ui.label("VRAM");
            if let Some(debug_texture) = &self.debug_texture {
                let img = egui::Image::new(debug_texture).fit_to_exact_size(ui.available_size());
                ui.add(img);
            }
            
        });
        ctx.request_repaint();
    }
}