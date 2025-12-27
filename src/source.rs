pub mod pipewire_source;

pub trait Source {
    async fn start(&mut self, data_tx: std::sync::mpsc::Sender<Vec<u8>>) -> anyhow::Result<()>;
}
