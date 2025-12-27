use tokio::{
    io::AsyncWriteExt,
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
};

use crate::client::Client;

pub struct TcpClient {
    ip: String,
    port: u16,
    writer: OwnedWriteHalf,
    reader: OwnedReadHalf,
}

impl Client for TcpClient {
    async fn new(ip: &str, port: u16) -> anyhow::Result<Self> {
        let address = format!("{}:{}", ip, port);
        let stream = TcpStream::connect(address).await?;

        let (reader, writer) = stream.into_split();

        Ok(TcpClient {
            ip: ip.to_string(),
            port,
            writer,
            reader,
        })
    }

    async fn send_data(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(data).await?;
        Ok(())
    }
}
