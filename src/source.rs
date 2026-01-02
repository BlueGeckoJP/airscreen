use crate::FrameSender;

#[cfg(target_os = "linux")]
pub mod pipewire_source;

#[async_trait::async_trait]
pub trait Source {
    async fn start(&mut self, tx: FrameSender) -> color_eyre::Result<()>;
}

pub fn get_source() -> Option<Box<dyn Source + Send>> {
    #[cfg(target_os = "linux")]
    {
        Some(Box::new(pipewire_source::PipeWireSource {}))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}
