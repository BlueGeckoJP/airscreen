use tokio::{io::AsyncReadExt, net::TcpListener};

use crate::server::Server;

pub struct TcpServer {
    port: u16,
    tx: std::sync::mpsc::Sender<Vec<u8>>,
}

impl Server for TcpServer {
    async fn new(port: u16, tx: std::sync::mpsc::Sender<Vec<u8>>) -> anyhow::Result<Self> {
        Ok(TcpServer { port, tx })
    }

    async fn listen(&self) -> anyhow::Result<()> {
        let address = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(address).await?;
        println!("Server listening on port {}", self.port);

        let (mut socket, addr) = listener.accept().await?;
        println!("New connection from {}", addr);

        let mut buf = vec![0; 1024];
        loop {
            match socket.read(&mut buf).await {
                Ok(0) => break Err(anyhow::anyhow!("Connection closed")),
                Ok(n) => {
                    buf.truncate(n);
                    if let Err(e) = self.tx.send(buf[..n].to_vec()) {
                        break Err(anyhow::anyhow!("Channel send error: {:?}", e));
                    }
                }
                Err(e) => break Err(anyhow::anyhow!("Failed to read from socket: {:?}", e)),
            }
        }
    }
}
