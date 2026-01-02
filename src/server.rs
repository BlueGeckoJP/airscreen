use crate::FrameSender;

pub mod tcp_server;

pub trait Server {
    async fn new(port: u16, tx: FrameSender) -> color_eyre::Result<Self>
    where
        Self: Sized;
    async fn listen(&self) -> color_eyre::Result<()>;
}
