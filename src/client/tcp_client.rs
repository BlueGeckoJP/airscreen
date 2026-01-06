use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, tcp::OwnedWriteHalf},
};

use crate::{client::Client, header::Header, perf::tcp_client_metrics::TcpClientMetrics};

pub struct TcpClient {
    writer: OwnedWriteHalf,
    compressor: turbojpeg::Compressor,
    metrics: TcpClientMetrics,
}

impl Client for TcpClient {
    async fn new(ip: &str, port: u16) -> color_eyre::Result<Self> {
        let address = format!("{}:{}", ip, port);
        let stream = TcpStream::connect(address).await?;

        let (_, writer) = stream.into_split();
        let mut compressor = turbojpeg::Compressor::new()?;
        compressor.set_quality(80)?;

        Ok(TcpClient {
            writer,
            compressor,
            metrics: TcpClientMetrics::default(),
        })
    }

    async fn send_frame(&mut self, data: &[u8], width: u32, height: u32) -> color_eyre::Result<()> {
        let total_len = data.len() as u32;

        let image = turbojpeg::Image {
            pixels: data,
            width: width as usize,
            pitch: width as usize * 3, // RGB is 3 bytes per pixel
            height: height as usize,
            format: turbojpeg::PixelFormat::RGB,
        };
        let jpeg_data = self.compressor.compress_to_vec(image)?;

        let mode: u8 = 0;
        let payload_len = jpeg_data.len() as u32;
        let header = Header {
            mode,
            width,
            height,
            total_len,
            payload_len,
        };
        let header_bytes: [u8; 17] = header.into();

        self.writer.write_all(&header_bytes).await?;
        self.writer.write_all(&jpeg_data).await?;
        self.writer.flush().await?;

        self.metrics
            .record_full_frame(jpeg_data.len() + header_bytes.len());

        Ok(())
    }
}
