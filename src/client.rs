pub mod tcp_client;

pub trait Client {
    async fn new(ip: &str, port: u16) -> anyhow::Result<Self>
    where
        Self: Sized;
    async fn send_data(&mut self, data: &[u8]) -> anyhow::Result<()>;
}
