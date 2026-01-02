use crate::FrameSender;

pub mod pipewire_source;

pub trait Source {
    async fn start(&mut self, tx: FrameSender) -> color_eyre::Result<()>;
}
