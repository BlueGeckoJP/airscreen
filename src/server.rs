pub mod tcp_server;

pub trait Server {
    async fn new(port: u16, tx: std::sync::mpsc::Sender<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized;
}
