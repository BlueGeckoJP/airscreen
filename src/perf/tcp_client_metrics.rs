use std::time::{Duration, Instant};

use tracing::info;

#[derive(Default)]
pub struct TcpClientMetrics {
    total_frames_sent: u64,

    total_bytes_sent: u64,

    send_durations: Vec<Duration>,

    last_instant: Option<Instant>,
}

impl TcpClientMetrics {
    pub fn record_full_frame(&mut self, frame_size: usize) {
        let start = Instant::now();

        self.total_frames_sent += 1;
        self.total_bytes_sent += frame_size as u64;

        if let Some(last) = self.last_instant {
            let duration = start.duration_since(last);
            self.send_durations.push(duration);
        }

        self.last_instant = Some(start);
    }

    fn log_stats(&self) {
        if self.total_bytes_sent == 0 {
            info!("No TCP client metrics recorded.");
            return;
        }

        let _span = tracing::info_span!("TCP Client Metrics").entered();

        info!("Total Frames Sent: {}", self.total_frames_sent,);

        let total_mb = self.total_bytes_sent as f64 / (1024.0 * 1024.0);

        info!("Total Bytes Sent: {:.2} MB", total_mb);

        if self.total_frames_sent > 0 {
            let avg_frame_size = self.total_bytes_sent as f64 / self.total_frames_sent as f64;
            info!(
                "Average Frame Size: {:.2} MB",
                avg_frame_size / (1024.0 * 1024.0)
            );
        }

        if !self.send_durations.is_empty() {
            let sum: Duration = self.send_durations.iter().sum();
            let avg = sum / self.send_durations.len() as u32;
            let min = self
                .send_durations
                .iter()
                .min()
                .copied()
                .unwrap_or_default();
            let max = self
                .send_durations
                .iter()
                .max()
                .copied()
                .unwrap_or_default();

            info!(
                "Send Durations - Avg: {:?}, Min: {:?}, Max: {:?}",
                avg, min, max
            );
        }
    }
}

impl Drop for TcpClientMetrics {
    fn drop(&mut self) {
        self.log_stats();
    }
}
