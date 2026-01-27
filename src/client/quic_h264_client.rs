use std::{
    net::{ToSocketAddrs, UdpSocket},
    time::Duration,
};

use crate::{MAX_DATAGRAM_SIZE, client::Client};
use color_eyre::eyre::OptionExt;
use quiche::RecvInfo;
use rand::TryRngCore;
use tracing::info;

pub struct QuicH264Client {
    socket: UdpSocket,
    conn: quiche::Connection,
    buf: [u8; 65535],
    out: [u8; MAX_DATAGRAM_SIZE],
}

impl Client for QuicH264Client {
    async fn new(ip: &str, port: u16) -> color_eyre::Result<Self>
    where
        Self: Sized,
    {
        let local_addr = "0.0.0.0:0";
        let socket = UdpSocket::bind(local_addr)?;
        let local_addr = socket.local_addr()?;

        let peer_addr = format!("{}:{}", ip, port)
            .to_socket_addrs()?
            .next()
            .ok_or_eyre("Failed to resolve server address")?;
        socket.connect(peer_addr)?;
        info!("QUIC client connected to {}", peer_addr);

        let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;

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

        // Disable peer verification in debug mode
        #[cfg(debug_assertions)]
        config.verify_peer(false);

        let mut scid = [0; quiche::MAX_CONN_ID_LEN];
        rand::rngs::OsRng.try_fill_bytes(&mut scid)?;
        let scid = quiche::ConnectionId::from_ref(&scid);

        let conn = quiche::connect(Some("localhost"), &scid, local_addr, peer_addr, &mut config)?;

        Ok(QuicH264Client {
            socket,
            conn,
            buf: [0; 65535],
            out: [0; MAX_DATAGRAM_SIZE],
        })
    }

    async fn send_frame(&mut self, data: &[u8], width: u32, height: u32) -> color_eyre::Result<()> {
        self.complete_handshake()?;

        let stream_id = 0;

        self.conn.stream_send(stream_id, data, true)?;

        self.flush_egress()?;

        Ok(())
    }
}

impl QuicH264Client {
    fn complete_handshake(&mut self) -> color_eyre::Result<()> {
        loop {
            let (write, _) = match self.conn.send(&mut self.out) {
                Ok(v) => v,
                Err(quiche::Error::Done) => break,
                Err(e) => {
                    return Err(color_eyre::eyre::eyre!(
                        "Failed to send handshake packet: {:?}",
                        e
                    ));
                }
            };

            self.socket.send(&self.out[..write])?;
        }

        while !self.conn.is_established() {
            let len = match self.socket.recv(&mut self.buf) {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    self.socket.set_read_timeout(Some(Duration::from_secs(5)))?;
                    continue;
                }
                Err(e) => {
                    return Err(color_eyre::eyre::eyre!(
                        "Failed to receive handshake packet: {:?}",
                        e
                    ));
                }
            };

            let recv_info = RecvInfo {
                to: self.socket.local_addr()?,
                from: self.socket.peer_addr()?,
            };

            self.conn.recv(&mut self.buf[..len], recv_info)?;

            loop {
                let (write, _) = match self.conn.send(&mut self.out) {
                    Ok(v) => v,
                    Err(quiche::Error::Done) => break,
                    Err(e) => {
                        return Err(color_eyre::eyre::eyre!(
                            "Failed to send handshake packet: {:?}",
                            e
                        ));
                    }
                };

                self.socket.send(&self.out[..write])?;
            }
        }

        info!("QUIC handshake completed");
        Ok(())
    }

    fn flush_egress(&mut self) -> color_eyre::Result<()> {
        loop {
            let (write, _) = match self.conn.send(&mut self.out) {
                Ok(v) => v,
                Err(quiche::Error::Done) => break,
                Err(e) => {
                    return Err(color_eyre::eyre::eyre!("Failed to send packet: {:?}", e));
                }
            };

            self.socket.send(&self.out[..write])?;
        }

        Ok(())
    }
}
