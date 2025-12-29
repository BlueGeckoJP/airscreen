pub mod tcp_server;

pub trait Server {
    async fn new(
        port: u16,
        tx: std::sync::mpsc::Sender<(Vec<u8>, u32, u32)>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized;
    async fn listen(&self) -> anyhow::Result<()>;
}
