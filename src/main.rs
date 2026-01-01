mod client;
mod color_utils;
mod header;
mod server;
mod source;
mod ui;

use std::sync::mpsc;

use crate::client::Client;
use crate::server::Server;
use crate::source::Source;
use crate::ui::App;

pub type FrameData = (Vec<u8>, u32, u32); // (data, width, height)
pub type FrameSender = std::sync::mpsc::SyncSender<FrameData>;

#[tokio::main]
async fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder {
            title: Some("AirScreen".to_owned()),
            inner_size: Some(eframe::egui::vec2(400.0, 300.0)),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "AirScreen",
        native_options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

async fn run_client(ip: String, port: String) -> anyhow::Result<()> {
    let port_u16 = port.parse::<u16>()?;

    let (tx, rx) = mpsc::sync_channel::<FrameData>(4);

    tokio::spawn(async move {
        let mut client = client::tcp_client::TcpClient::new(&ip, port_u16)
            .await
            .expect("failed to create client");

        while let Ok(data) = rx.recv() {
            if let Err(e) = client.send_frame(&data.0, data.1, data.2).await {
                eprintln!("Failed to send data: {:?}", e);
                break;
            }
        }
    });

    let mut source = source::pipewire_source::PipeWireSource {};
    source.start(tx).await.expect("failed to start source");

    Ok(())
}

async fn run_server(port: String, tx: FrameSender) -> anyhow::Result<()> {
    let port_u16 = port.parse::<u16>()?;

    tokio::spawn(async move {
        let server = server::tcp_server::TcpServer::new(port_u16, tx)
            .await
            .expect("failed to create server");
        if let Err(e) = server.listen().await {
            eprintln!("Server error: {:?}", e);
        }
    });

    Ok(())
}
