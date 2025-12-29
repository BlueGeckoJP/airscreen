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

    async fn send_data(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(data).await?;
        Ok(())
    }
}
