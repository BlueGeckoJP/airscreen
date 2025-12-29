use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, tcp::OwnedWriteHalf},
};

use crate::client::Client;

pub struct TcpClient {
    writer: OwnedWriteHalf,
}

impl Client for TcpClient {
    async fn new(ip: &str, port: u16) -> anyhow::Result<Self> {
        let address = format!("{}:{}", ip, port);
        let stream = TcpStream::connect(address).await?;

        let (_, writer) = stream.into_split();

        Ok(TcpClient { writer })
    }

    async fn send_frame(&mut self, data: &[u8], width: u32, height: u32) -> anyhow::Result<()> {
        // Header: 4 bytes width, 4 bytes height, 4 bytes data length
        self.writer.write_all(&width.to_le_bytes()).await?;
        self.writer.write_all(&height.to_le_bytes()).await?;
        self.writer
            .write_all(&(data.len() as u32).to_le_bytes())
            .await?;

        // Data
        self.writer.write_all(data).await?;

        self.writer.flush().await?;
        Ok(())
    }
}
