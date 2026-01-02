pub mod tcp_client;

pub trait Client {
    async fn new(ip: &str, port: u16) -> color_eyre::Result<Self>
    where
        Self: Sized;
    async fn send_frame(&mut self, data: &[u8], width: u32, height: u32) -> color_eyre::Result<()>;
}
