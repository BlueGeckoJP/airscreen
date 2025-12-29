mod args;
mod client;
mod source;

use std::sync::mpsc;

use clap::Parser;

use crate::client::Client;
use crate::source::Source;

#[tokio::main]
async fn main() {
    let args = crate::args::Args::parse();

    match args.mode.as_str() {
        "client" => {
            if let Err(e) = run_client().await {
                eprintln!("Client error: {:?}", e);
            }
        }
        _ => {
            eprintln!("Unknown mode: {}", args.mode);
        }
    }
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
