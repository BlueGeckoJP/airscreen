use tokio::task::JoinHandle;
use tracing::error;

use crate::{client, source};

pub mod quic_h264_client;
pub mod tcp_client;

pub trait Client {
    async fn new(ip: &str, port: u16) -> color_eyre::Result<Self>
    where
        Self: Sized;
    async fn send_frame(&mut self, data: &[u8], width: u32, height: u32) -> color_eyre::Result<()>;
}

#[tracing::instrument]
pub async fn run_client(ip: String, port: String) -> color_eyre::Result<Vec<JoinHandle<()>>> {
    let port_u16 = port.parse::<u16>()?;

    let (tx, rx) = crossbeam_channel::bounded(4);

    let mut join_handles = vec![];

    // NOTE: If you create the TcpClient before initializing the Pipewire source,
    // data will not reach rx.recv(). Therefore, you must always initialize it first.
    if let Some(mut src) = source::get_source() {
        let join_handle = src.start(tx).await.expect("Failed to start source");
        join_handles.push(join_handle);
    } else {
        error!("No available source for this OS");
        return Ok(vec![]);
    }

    let join_handle = tokio::spawn(async move {
        let mut client = client::tcp_client::TcpClient::new(&ip, port_u16)
            .await
            .expect("failed to create client");

        while let Ok(data) = rx.recv() {
            if let Err(e) = client.send_frame(&data.data, data.width, data.height).await {
                error!("Failed to send frame to server: {:?}", e);
                break;
            }
        }
    });
    join_handles.push(join_handle);

    Ok(join_handles)
}
