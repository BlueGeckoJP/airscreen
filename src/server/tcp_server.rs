use image::RgbImage;
use tokio::{io::AsyncReadExt, net::TcpListener};
use tracing::{info, trace};

use crate::{
    FrameData, FrameSender,
    header::{HEADER_SIZE, Header},
    perf::tcp_server_metrics::TcpServerMetrics,
    server::Server,
};

pub struct TcpServer {
    port: u16,
    tx: FrameSender,
}

impl Server for TcpServer {
    async fn new(port: u16, tx: FrameSender) -> color_eyre::Result<Self> {
        Ok(TcpServer { port, tx })
    }

    async fn listen(&mut self) -> color_eyre::Result<()> {
        let address = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&address).await?;
        info!("Server listening on {}", address);

        let (mut socket, addr) = listener.accept().await?;
        info!("New connection from {}", addr);

        let mut metrics = TcpServerMetrics::default();

        loop {
            let mut raw_header = [0u8; HEADER_SIZE];
            match socket.read_exact(&mut raw_header).await {
                Ok(0) => break Err(color_eyre::eyre::eyre!("Connection closed")),
                Ok(_) => {
                    let header = Header::from(&raw_header);
                    let Header {
                        width,
                        height,
                        payload_len,
                    } = header;

                    trace!(
                        "Receiving frame: width={}, height={}, payload_len={}",
                        width, height, payload_len
                    );

                    let mut payload = vec![0u8; payload_len as usize];
                    socket.read_exact(&mut payload).await?;

                    let rgb_image: RgbImage = turbojpeg::decompress_image(&payload)?;

                    metrics.record_full_frame(payload.len() + raw_header.len());
                    let frame_data = FrameData::new(rgb_image, width, height);
                    if let Err(e) = self.tx.send(frame_data) {
                        break Err(color_eyre::eyre::eyre!(
                            "Failed to send frame to processing channel: {:?}",
                            e
                        ));
                    }
                }
                Err(e) => {
                    break Err(color_eyre::eyre::eyre!(
                        "Failed to read from socket: {:?}",
                        e
                    ));
                }
            }
        }
    }
}
