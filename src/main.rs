mod client;
mod color_utils;
mod header;
mod perf;
mod server;
mod source;
mod ui;

use crate::ui::App;

pub struct FrameData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl FrameData {
    pub fn new(data: Vec<u8>, width: u32, height: u32) -> Self {
        FrameData {
            data,
            width,
            height,
        }
    }
}

pub type FrameSender = crossbeam_channel::Sender<FrameData>;

#[tokio::main]
async fn main() -> eframe::Result {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder {
            title: Some("AirScreen".to_owned()),
            inner_size: Some(eframe::egui::vec2(400.0, 300.0)),
            ..Default::default()
        },
        vsync: false,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "AirScreen",
        native_options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
