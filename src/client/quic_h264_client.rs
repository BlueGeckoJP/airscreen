use std::net::{ToSocketAddrs, UdpSocket};

use crate::{MAX_DATAGRAM_SIZE, client::Client};
use color_eyre::eyre::OptionExt;
use rand::TryRngCore;
use tracing::info;

pub struct QuicH264Client {
    socket: UdpSocket,
    conn: quiche::Connection,
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

        Ok(QuicH264Client { socket, conn })
    }

    async fn send_frame(&mut self, data: &[u8], width: u32, height: u32) -> color_eyre::Result<()> {
        Ok(())
    }
}
