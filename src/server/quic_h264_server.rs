use std::{net::UdpSocket, path::Path};

use rcgen::CertifiedKey;
use tracing::info;

use crate::{MAX_DATAGRAM_SIZE, server::Server};

pub struct QuicH264Server {
    socket: UdpSocket,
    config: quiche::Config,
}

impl Server for QuicH264Server {
    async fn new(port: u16, tx: crate::FrameSender) -> color_eyre::Result<Self>
    where
        Self: Sized,
    {
        let local_addr = format!("0.0.0.0:{}", port);
        let socket = UdpSocket::bind(&local_addr)?;
        info!("QUIC server listening on {}", local_addr);

        let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;

        let cert_path = "cert.pem";
        let key_path = "key.pem";
        Self::ensure_pems(cert_path, key_path)?;
        config.load_cert_chain_from_pem_file(cert_path)?;
        config.load_priv_key_from_pem_file(key_path)?;

        config.set_application_protos(&[b"h3"])?;
        config.set_max_idle_timeout(5000);
        config.set_max_recv_udp_payload_size(MAX_DATAGRAM_SIZE);
        config.set_max_send_udp_payload_size(MAX_DATAGRAM_SIZE);
        config.set_initial_max_data(10_000_000);
        config.set_initial_max_stream_data_bidi_local(1_000_000);
        config.set_initial_max_stream_data_bidi_remote(1_000_000);
        config.set_initial_max_stream_data_uni(1_000_000);
        config.set_initial_max_streams_bidi(100);
        config.set_initial_max_streams_uni(100);
        config.set_disable_active_migration(true);

        Ok(QuicH264Server { socket, config })
    }

    async fn listen(&self) -> color_eyre::Result<()> {
        Ok(())
    }
}

impl QuicH264Server {
    fn ensure_pems(cert_path: &str, key_path: &str) -> color_eyre::Result<()> {
        let cert_exists = Path::new(cert_path).exists();
        let key_exists = Path::new(key_path).exists();

        if cert_exists && key_exists {
            return Ok(());
        }

        let CertifiedKey { cert, signing_key } =
            rcgen::generate_simple_self_signed(vec!["localhost".into()])?;

        std::fs::write(cert_path, cert.pem())?;
        std::fs::write(key_path, signing_key.serialize_pem())?;

        Ok(())
    }
}
