use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, tcp::OwnedWriteHalf},
};

use crate::client::Client;

pub struct TcpClient {
    writer: OwnedWriteHalf,
    prev_frame: Option<Vec<u8>>,
}

impl Client for TcpClient {
    async fn new(ip: &str, port: u16) -> anyhow::Result<Self> {
        let address = format!("{}:{}", ip, port);
        let stream = TcpStream::connect(address).await?;

        let (_, writer) = stream.into_split();

        Ok(TcpClient {
            writer,
            prev_frame: None,
        })
    }

    async fn send_frame(&mut self, data: &[u8], width: u32, height: u32) -> anyhow::Result<()> {
        let total_len = data.len() as u32;

        let send_full = match &self.prev_frame {
            Some(prev) => prev.len() != data.len(),
            None => true,
        };

        if send_full {
            let mode: u8 = 0;
            let payload_len = data.len() as u32;

            self.writer.write_all(&[mode]).await?;
            self.writer.write_all(&width.to_le_bytes()).await?;
            self.writer.write_all(&height.to_le_bytes()).await?;
            self.writer.write_all(&total_len.to_le_bytes()).await?;
            self.writer.write_all(&payload_len.to_le_bytes()).await?;

            self.writer.write_all(data).await?;
            self.writer.flush().await?;

            self.prev_frame = Some(data.to_vec());
            return Ok(());
        }

        let prev = self.prev_frame.as_ref().unwrap();
        let mut chunks: Vec<(u32, usize)> = Vec::new();
        let mut i = 0usize;
        while i < data.len() {
            if data[i] != prev[i] {
                let start = i;
                i += 1;
                while i < data.len() && data[i] != prev[i] {
                    i += 1;
                }
                chunks.push((start as u32, i - start));
            } else {
                i += 1;
            }
        }

        if chunks.is_empty() {
            return Ok(());
        }

        let mut payload: Vec<u8> = Vec::new();
        payload.extend_from_slice(&(chunks.len() as u32).to_le_bytes());
        for (offset, len) in &chunks {
            payload.extend_from_slice(&offset.to_le_bytes());
            payload.extend_from_slice(&(*len as u32).to_le_bytes());
            let off = *offset as usize;
            payload.extend_from_slice(&data[off..(off + *len)]);
        }

        let mode: u8 = 1;
        let payload_len = payload.len() as u32;

        self.writer.write_all(&[mode]).await?;
        self.writer.write_all(&width.to_le_bytes()).await?;
        self.writer.write_all(&height.to_le_bytes()).await?;
        self.writer.write_all(&total_len.to_le_bytes()).await?;
        self.writer.write_all(&payload_len.to_le_bytes()).await?;

        self.writer.write_all(&payload).await?;
        self.writer.flush().await?;

        self.prev_frame = Some(data.to_vec());

        Ok(())
    }
}
