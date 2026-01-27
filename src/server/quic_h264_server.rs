use std::{
    net::{SocketAddr, UdpSocket},
    path::Path,
};

use color_eyre::eyre::OptionExt;
use rand::TryRngCore;
use rcgen::CertifiedKey;
use tracing::{debug, error, info};

use crate::{FrameSender, MAX_DATAGRAM_SIZE, server::Server};

pub struct QuicH264Server {
    socket: UdpSocket,
    config: quiche::Config,
    client: Option<(SocketAddr, quiche::Connection)>,
    buf: [u8; 65535],
    tx: FrameSender,
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

        Ok(QuicH264Server {
            socket,
            config,
            buf: [0; 65535],
            client: None,
            tx,
        })
    }

    async fn listen(&mut self) -> color_eyre::Result<()> {
        loop {
            let (len, from) = self.socket.recv_from(&mut self.buf)?;
            let pkt_buf = &mut self.buf[..len];

            let header = match quiche::Header::from_slice(pkt_buf, quiche::MAX_CONN_ID_LEN) {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to parse QUIC packet header: {:?}", e);
                    continue;
                }
            };

            if self.client.is_none() {
                if header.ty != quiche::Type::Initial {
                    error!("Expected Initial packet from new client: {:?}", header.ty);
                    continue;
                }

                let mut scid = [0; quiche::MAX_CONN_ID_LEN];
                rand::rngs::OsRng.try_fill_bytes(&mut scid)?;
                let scid = quiche::ConnectionId::from_ref(&scid);

                let odcid = Some(header.dcid.clone());
                let local_addr = self.socket.local_addr()?;

                info!(
                    "New QUIC connection from={}, dcid={:?}, scid={:?}",
                    from, header.dcid, scid
                );

                let conn =
                    quiche::accept(&scid, odcid.as_ref(), local_addr, from, &mut self.config)?;

                self.client = Some((from, conn));
            }

            if matches!(self.client, Some((addr, _)) if addr != from) {
                error!("Received packet from unknown client: {}", from);
                continue;
            }

            let (_, conn) = self
                .client
                .as_mut()
                .ok_or_eyre("Failed to get client connection")?;

            let recv_info = quiche::RecvInfo {
                to: self.socket.local_addr()?,
                from,
            };

            match conn.recv(pkt_buf, recv_info) {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to receive QUIC packet: {:?}", e);
                    continue;
                }
            };

            if conn.is_established() {
                Self::handle_stream(conn)?;
            }

            if conn.is_closed() {
                info!("Connection closed: {:?}", conn.stats());
                self.client = None;
            }
        }
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

    fn handle_stream(conn: &mut quiche::Connection) -> color_eyre::Result<()> {
        for stream_id in conn.readable() {
            let mut buf = [0; 65535];

            loop {
                match conn.stream_recv(stream_id, &mut buf) {
                    Ok((read, fin)) => {
                        debug!(
                            "Received {} bytes on stream {} (fin={})",
                            read, stream_id, fin
                        );

                        if read == 0 {
                            continue;
                        }

                        if fin {
                            break;
                        }

                        let data = &buf[..read];

                        info!(
                            "Processing frame data of length {} on stream {}",
                            data.len(),
                            stream_id
                        );
                    }

                    Err(quiche::Error::Done) => {
                        break;
                    }

                    Err(e) => {
                        error!("Failed to read from stream {}: {:?}", stream_id, e);
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
