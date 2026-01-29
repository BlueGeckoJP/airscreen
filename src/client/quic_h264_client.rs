use std::sync::Arc;

use openh264::{
    encoder::Encoder,
    formats::{RgbSliceU8, YUVBuffer},
};
use quinn::Endpoint;
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::client::Client;

pub struct QuicH264Client {
    conn: quinn::Connection,
    send: quinn::SendStream,

    encoder: Encoder,
}

impl Client for QuicH264Client {
    async fn new(ip: &str, port: u16) -> color_eyre::Result<Self>
    where
        Self: Sized,
    {
        let encoder = Encoder::new()?;

        let client_config = Self::configure_client()?;

        let mut endpoint = Endpoint::client("0.0.0.0".parse()?)?;
        endpoint.set_default_client_config(client_config);

        let server_addr = format!("{}:{}", ip, port).parse()?;
        let conn = endpoint.connect(server_addr, "localhost")?.await?;
        info!("Connected to server: {}", conn.remote_address());

        let send = conn.open_uni().await?;

        Ok(QuicH264Client {
            conn,
            send,
            encoder,
        })
    }

    async fn send_frame(&mut self, data: &[u8], width: u32, height: u32) -> color_eyre::Result<()> {
        let rgb_source = RgbSliceU8::new(data, (width as usize, height as usize));
        let yuv_source = YUVBuffer::from_rgb8_source(rgb_source);

        let bitstream = self.encoder.encode(&yuv_source)?;
        let mut bitstream_bytes = vec![];
        bitstream.write_vec(&mut bitstream_bytes);

        self.send.write_all(&bitstream_bytes).await?;
        self.send.flush().await?;

        Ok(())
    }
}

impl QuicH264Client {
    fn configure_client() -> color_eyre::Result<quinn::ClientConfig> {
        let crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();

        Ok(quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?,
        )))
    }
}

#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
