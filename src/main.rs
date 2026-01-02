mod client;
mod header;
mod server;
mod source;
mod ui;

use std::sync::mpsc;

use tracing::error;

use crate::client::Client;
use crate::server::Server;
use crate::source::Source;
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

#[tracing::instrument]
async fn run_client(ip: String, port: String) -> color_eyre::Result<()> {
    let port_u16 = port.parse::<u16>()?;

    let (tx, rx) = mpsc::sync_channel::<FrameData>(4);

    let mut source = source::pipewire_source::PipeWireSource {};
    source.start(tx).await?;

    tokio::spawn(async move {
        let mut client = client::tcp_client::TcpClient::new(&ip, port_u16)
            .await
            .expect("failed to create client");

        while let Ok(data) = rx.recv() {
            if let Err(e) = client.send_frame(&data.0, data.1, data.2).await {
                error!("Failed to send frame to server: {:?}", e);
                break;
            }
        }
    });

    Ok(())
}

#[tracing::instrument]
async fn run_server(port: String, tx: FrameSender) -> color_eyre::Result<()> {
    let port_u16 = port.parse::<u16>()?;

    tokio::spawn(async move {
        let server = server::tcp_server::TcpServer::new(port_u16, tx)
            .await
            .expect("failed to create server");
        if let Err(e) = server.listen().await {
            error!("Server error: {:?}", e);
        }
    });

    Ok(())
}
