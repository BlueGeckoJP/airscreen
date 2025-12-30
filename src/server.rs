use crate::FrameSender;

pub mod tcp_server;

pub trait Server {
    async fn new(port: u16, tx: FrameSender) -> anyhow::Result<Self>
    where
        Self: Sized;
    async fn listen(&self) -> anyhow::Result<()>;
}
