use tokio::task::JoinHandle;

use crate::FrameSender;

#[cfg(target_os = "linux")]
pub mod pipewire_source;

#[cfg(windows)]
pub mod windows_source;

#[async_trait::async_trait]
pub trait Source {
    async fn start(&mut self, tx: FrameSender) -> color_eyre::Result<JoinHandle<()>>;
}

pub fn get_source() -> Option<Box<dyn Source + Send>> {
    #[cfg(target_os = "linux")]
    {
        Some(Box::new(pipewire_source::PipeWireSource {}))
    }
    #[cfg(windows)]
    {
        Some(Box::new(windows_source::WindowsSource {}))
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        None
    }
}
