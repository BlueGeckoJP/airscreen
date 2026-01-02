mod client;
mod header;
mod server;
mod source;
mod ui;

use crate::ui::App;

pub type FrameData = (Vec<u8>, u32, u32); // (data, width, height)
pub type FrameSender = std::sync::mpsc::SyncSender<FrameData>;

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
        ..Default::default()
    };

    eframe::run_native(
        "AirScreen",
        native_options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
