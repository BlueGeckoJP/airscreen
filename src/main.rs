mod client;
mod header;
mod perf;
mod server;
mod source;
mod ui;
mod utils;

use image::RgbImage;

use crate::ui::App;

pub struct FrameData {
    pub data: RgbImage,
    pub width: u32,
    pub height: u32,
}

impl FrameData {
    pub fn new(data: RgbImage, width: u32, height: u32) -> Self {
        FrameData {
            data,
            width,
            height,
        }
    }
}

pub type FrameSender = crossbeam_channel::Sender<FrameData>;
pub type FrameReceiver = crossbeam_channel::Receiver<FrameData>;

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
