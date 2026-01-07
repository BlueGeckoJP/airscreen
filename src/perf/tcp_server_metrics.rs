use std::time::{Duration, Instant};

use tracing::info;

#[derive(Default)]
pub struct TcpServerMetrics {
    total_frames_received: u64,

    total_bytes_received: u64,

    receive_durations: Vec<Duration>,

    last_instant: Option<Instant>,
}

impl TcpServerMetrics {
    pub fn record_full_frame(&mut self, frame_size: usize) {
        let start = Instant::now();

        self.total_frames_received += 1;
        self.total_bytes_received += frame_size as u64;

        if let Some(last) = self.last_instant {
            let duration = start.duration_since(last);
            self.receive_durations.push(duration);
        }

        self.last_instant = Some(start);
    }

    fn log_stats(&self) {
        if self.total_bytes_received == 0 {
            info!("No TCP server metrics recorded.");
            return;
        }

        let _span = tracing::info_span!("TCP Server Metrics").entered();

        info!("Total Frames Received: {}", self.total_frames_received,);

        let total_mb = self.total_bytes_received as f64 / (1024.0 * 1024.0);

        info!("Total Data Received: {:.2} MB", total_mb);

        if self.total_frames_received > 0 {
            let avg_frame_size =
                self.total_bytes_received as f64 / self.total_frames_received as f64;
            info!(
                "Average Frame Size: {:.2} MB",
                avg_frame_size / (1024.0 * 1024.0)
            );
        }

        if !self.receive_durations.is_empty() {
            let sum: Duration = self.receive_durations.iter().sum();
            let avg = sum / self.receive_durations.len() as u32;
            let min = self
                .receive_durations
                .iter()
                .min()
                .copied()
                .unwrap_or_default();
            let max = self
                .receive_durations
                .iter()
                .max()
                .copied()
                .unwrap_or_default();

            info!(
                "Receive Durations - Avg: {:?}, Min: {:?}, Max: {:?}",
                avg, min, max
            );
        }
    }
}

impl Drop for TcpServerMetrics {
    fn drop(&mut self) {
        self.log_stats();
    }
}
