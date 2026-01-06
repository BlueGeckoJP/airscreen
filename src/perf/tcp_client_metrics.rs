use std::time::{Duration, Instant};

use tracing::info;

#[derive(Default)]
pub struct TcpClientMetrics {
    total_frames_sent: u64,
    full_frames_sent: u64,
    delta_frames_sent: u64,
    skipped_frames: u64,

    total_bytes_sent: u64,
    full_frame_bytes_sent: u64,
    delta_frame_bytes_sent: u64,

    compression_ratios: Vec<f64>,

    send_durations: Vec<Duration>,

    last_instant: Option<Instant>,
}

impl TcpClientMetrics {
    pub fn record_full_frame(&mut self, frame_size: usize) {
        let start = Instant::now();

        self.total_frames_sent += 1;
        self.full_frames_sent += 1;
        self.total_bytes_sent += frame_size as u64;
        self.full_frame_bytes_sent += frame_size as u64;

        self.compression_ratios.push(1.0);

        if let Some(last) = self.last_instant {
            let duration = start.duration_since(last);
            self.send_durations.push(duration);
        }

        self.last_instant = Some(start);
    }

    pub fn record_delta_frame(&mut self, payload_size: usize, total_size: usize) {
        let start = Instant::now();

        self.total_frames_sent += 1;
        self.delta_frames_sent += 1;
        self.total_bytes_sent += payload_size as u64;
        self.delta_frame_bytes_sent += payload_size as u64;

        if total_size > 0 {
            let ratio = payload_size as f64 / total_size as f64;
            self.compression_ratios.push(ratio);
        }

        if let Some(last) = self.last_instant {
            let duration = start.duration_since(last);
            self.send_durations.push(duration);
        }

        self.last_instant = Some(start);
    }

    pub fn record_skipped_frame(&mut self) {
        self.skipped_frames += 1;
    }

    fn log_stats(&self) {
        if self.total_bytes_sent == 0 {
            info!("No TCP client metrics recorded.");
            return;
        }

        let _span = tracing::info_span!("TCP Client Metrics").entered();

        info!(
            "Frames - Total: {}, Full: {}, Delta: {}, Skipped: {}",
            self.total_frames_sent,
            self.full_frames_sent,
            self.delta_frames_sent,
            self.skipped_frames
        );

        let total_mb = self.total_bytes_sent as f64 / (1024.0 * 1024.0);
        let full_mb = self.full_frame_bytes_sent as f64 / (1024.0 * 1024.0);
        let delta_mb = self.delta_frame_bytes_sent as f64 / (1024.0 * 1024.0);

        info!(
            "Data Sent - Total: {:.2} MB, Full Frames: {:.2} MB, Delta Frames: {:.2} MB",
            total_mb, full_mb, delta_mb
        );

        if self.total_frames_sent > 0 {
            let avg_frame_size = self.total_bytes_sent as f64 / self.total_frames_sent as f64;
            info!(
                "Average Frame Size: {:.2} MB",
                avg_frame_size / (1024.0 * 1024.0)
            );
        }

        if !self.compression_ratios.is_empty() {
            let avg_ratio =
                self.compression_ratios.iter().sum::<f64>() / self.compression_ratios.len() as f64;
            let min_ratio = self
                .compression_ratios
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);
            let max_ratio = self
                .compression_ratios
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);

            info!(
                "Compression Ratios - Avg: {:.2}%, Min: {:.2}%, Max: {:.2}%",
                avg_ratio * 100.0,
                min_ratio * 100.0,
                max_ratio * 100.0
            )
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
