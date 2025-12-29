mod args;
mod client;
mod source;
mod ui;

use std::sync::mpsc;

use clap::Parser;

use crate::client::Client;
use crate::source::Source;
use crate::ui::App;

#[tokio::main]
async fn main() -> eframe::Result {
    let args = crate::args::Args::parse();

    if args.mode == "client" {
        run_client().await.expect("failed to run client");
        return Ok(());
    }

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "AirScreen",
        native_options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

async fn run_client() -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    tokio::spawn(async move {
        let mut client = client::tcp_client::TcpClient::new("127.0.0.1", 8080)
            .await
            .expect("failed to create client");

        while let Ok(data) = rx.recv() {
            if let Err(e) = client.send_data(&data).await {
                eprintln!("Failed to send data: {:?}", e);
            }
        }
    });

    let mut source = source::pipewire_source::PipeWireSource {};
    source.start(tx).await.expect("failed to start source");

    Ok(())
}
