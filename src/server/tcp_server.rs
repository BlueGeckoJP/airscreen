use tokio::{io::AsyncReadExt, net::TcpListener};

use crate::server::Server;

pub struct TcpServer {
    port: u16,
}

impl Server for TcpServer {
    async fn new(port: u16, tx: std::sync::mpsc::Sender<Vec<u8>>) -> anyhow::Result<Self> {
        let address = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(address).await?;
        println!("Server listening on port {}", port);

        let (mut socket, addr) = listener.accept().await?;
        println!("New connection from {}", addr);

        loop {
            let mut buf = vec![0; 1024];

            match socket.read(&mut buf).await {
                Ok(0) => break Err(anyhow::anyhow!("Connection closed")),
                Ok(n) => {
                    buf.truncate(n);
                    if let Err(e) = tx.send(buf) {
                        eprintln!("Failed to send data to channel: {:?}", e);
                    }
                }
                Err(e) => break Err(anyhow::anyhow!("Failed to read from socket: {:?}", e)),
            }
        }
    }
}
