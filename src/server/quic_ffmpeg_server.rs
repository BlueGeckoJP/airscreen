use quinn::Endpoint;
use rustls::pki_types::PrivateKeyDer;
use tracing::info;

use crate::{FrameSender, server::Server};

pub struct QuicFfmpegServer {
    server_config: quinn::ServerConfig,
    addr: std::net::SocketAddr,
    tx: FrameSender,
}

impl Server for QuicFfmpegServer {
    async fn new(port: u16, tx: crate::FrameSender) -> color_eyre::Result<Self>
    where
        Self: Sized,
    {
        let server_config = Self::configure_server()?;

        let addr = format!("0.0.0.0:{}", port).parse()?;

        Ok(QuicFfmpegServer {
            server_config,
            addr,
            tx,
        })
    }

    async fn listen(&self) -> color_eyre::Result<()> {
        let endpoint = Endpoint::server(self.server_config.clone(), self.addr)?;
        info!("QUIC server listening on {}", self.addr);

        while let Some(conn) = endpoint.accept().await {
            let conn = conn.await?;
            info!("New connection from {}", conn.remote_address());

            let mut recv = conn.accept_uni().await?;
        }

        Ok(())
    }
}

impl QuicFfmpegServer {
    fn configure_server() -> color_eyre::Result<quinn::ServerConfig> {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
        let cert_der = cert.cert.der().to_vec();
        let priv_key = cert.signing_key.serialize_der();

        let server_config = quinn::ServerConfig::with_single_cert(
            vec![cert_der.clone().into()],
            PrivateKeyDer::try_from(priv_key).map_err(|e| {
                color_eyre::eyre::eyre!("Failed to convert to PrivateKeyDer: {}", e)
            })?,
        )?;

        Ok(server_config)
    }
}
