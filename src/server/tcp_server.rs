use tokio::{io::AsyncReadExt, net::TcpListener};
use tracing::{info, trace};

use crate::{FrameSender, header::Header, server::Server};

pub struct TcpServer {
    port: u16,
    tx: FrameSender,
}

impl Server for TcpServer {
    async fn new(port: u16, tx: FrameSender) -> color_eyre::Result<Self> {
        Ok(TcpServer { port, tx })
    }

    async fn listen(&self) -> color_eyre::Result<()> {
        let address = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&address).await?;
        info!("Server listening on {}", address);

        let (mut socket, addr) = listener.accept().await?;
        info!("New connection from {}", addr);

        let mut prev_frame = Option::<Vec<u8>>::None;

        loop {
            let mut raw_header = [0u8; 17];
            match socket.read_exact(&mut raw_header).await {
                Ok(0) => break Err(color_eyre::eyre::anyhow!("Connection closed")),
                Ok(_) => {
                    let header = Header::from(&raw_header);
                    let Header {
                        mode,
                        width,
                        height,
                        total_len,
                        payload_len,
                    } = header;

                    trace!(
                        "Receiving frame: mode={}, width={}, height={}, total_len={}, payload_len={}",
                        mode, width, height, total_len, payload_len
                    );

                    let mut payload = vec![0u8; payload_len as usize];
                    socket.read_exact(&mut payload).await?;

                    if mode == 0 {
                        prev_frame = Some(payload.clone());
                        if let Err(e) = self.tx.send((payload, width, height)) {
                            break Err(color_eyre::eyre::anyhow!(
                                "Failed to send frame to processing channel: {:?}",
                                e
                            ));
                        }
                    } else if mode == 1 {
                        let mut cursor = 0usize;
                        if payload_len < 4 {
                            continue;
                        }
                        let num_chunks =
                            u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
                                as usize;
                        cursor += 4;

                        if prev_frame.is_none()
                            || prev_frame.as_ref().unwrap().len() != total_len as usize
                        {
                            prev_frame = Some(vec![0u8; total_len as usize]);
                        }
                        let mut buf = prev_frame.take().unwrap();

                        for _ in 0..num_chunks {
                            if cursor + 8 > payload.len() {
                                break;
                            }
                            let offset = u32::from_le_bytes([
                                payload[cursor],
                                payload[cursor + 1],
                                payload[cursor + 2],
                                payload[cursor + 3],
                            ]) as usize;
                            cursor += 4;
                            let len = u32::from_le_bytes([
                                payload[cursor],
                                payload[cursor + 1],
                                payload[cursor + 2],
                                payload[cursor + 3],
                            ]) as usize;
                            cursor += 4;

                            if cursor + len > payload.len() || offset + len > buf.len() {
                                break;
                            }
                            buf[offset..offset + len]
                                .copy_from_slice(&payload[cursor..cursor + len]);
                            cursor += len;
                        }

                        if let Err(e) = self.tx.send((buf.clone(), width, height)) {
                            break Err(color_eyre::eyre::anyhow!(
                                "Failed to send frame to processing channel: {:?}",
                                e
                            ));
                        }
                        prev_frame = Some(buf);
                    } else {
                        continue;
                    }
                }
                Err(e) => {
                    break Err(color_eyre::eyre::anyhow!(
                        "Failed to read from socket: {:?}",
                        e
                    ));
                }
            }
        }
    }
}
