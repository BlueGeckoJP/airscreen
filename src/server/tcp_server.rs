use tokio::{io::AsyncReadExt, net::TcpListener};

use crate::server::Server;

pub struct TcpServer {
    port: u16,
    tx: std::sync::mpsc::Sender<(Vec<u8>, u32, u32)>,
}

impl Server for TcpServer {
    async fn new(
        port: u16,
        tx: std::sync::mpsc::Sender<(Vec<u8>, u32, u32)>,
    ) -> anyhow::Result<Self> {
        Ok(TcpServer { port, tx })
    }

    async fn listen(&self) -> anyhow::Result<()> {
        let address = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(address).await?;
        println!("Server listening on port {}", self.port);

        let (mut socket, addr) = listener.accept().await?;
        println!("New connection from {}", addr);

        loop {
            let mut header = [0u8; 12];
            match socket.read_exact(&mut header).await {
                Ok(0) => break Err(anyhow::anyhow!("Connection closed")),
                Ok(_) => {
                    let width = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
                    let height = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
                    let data_len =
                        u32::from_le_bytes([header[8], header[9], header[10], header[11]]);

                    let mut buf = vec![0u8; data_len as usize];
                    socket.read_exact(&mut buf).await?;

                    if let Err(e) = self
                        .tx
                        .send((buf[..data_len as usize].to_vec(), width, height))
                    {
                        break Err(anyhow::anyhow!("Channel send error: {:?}", e));
                    }
                }
                Err(e) => break Err(anyhow::anyhow!("Failed to read from socket: {:?}", e)),
            }
        }
    }
}
