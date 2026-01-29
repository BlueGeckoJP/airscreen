use tokio::task::JoinHandle;
use tracing::error;

use crate::{FrameSender, server};

pub mod quic_ffmpeg_server;
pub mod tcp_server;

pub trait Server {
    async fn new(port: u16, tx: FrameSender) -> color_eyre::Result<Self>
    where
        Self: Sized;
    async fn listen(&self) -> color_eyre::Result<()>;
}

#[tracing::instrument]
pub async fn run_server(port: String, tx: FrameSender) -> color_eyre::Result<JoinHandle<()>> {
    let port_u16 = port.parse::<u16>()?;

    let join_handle = tokio::spawn(async move {
        let server = server::tcp_server::TcpServer::new(port_u16, tx)
            .await
            .expect("failed to create server");
        if let Err(e) = server.listen().await {
            error!("Server error: {:?}", e);
        }
    });

    Ok(join_handle)
}
