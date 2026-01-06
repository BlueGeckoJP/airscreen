use std::time::{Duration, Instant};

use tracing::info;

#[derive(Default)]
pub struct TcpServerMetrics {
    total_frames_received: u64,
    full_frames_received: u64,
    delta_frames_received: u64,

    total_bytes_received: u64,
    full_frame_bytes_received: u64,
    delta_frame_bytes_received: u64,

    decompression_ratios: Vec<f64>,

    receive_durations: Vec<Duration>,

    last_instant: Option<Instant>,
}

impl TcpServerMetrics {
    pub fn record_full_frame(&mut self, frame_size: usize) {
        let start = Instant::now();

        self.total_frames_received += 1;
        self.full_frames_received += 1;
        self.total_bytes_received += frame_size as u64;
        self.full_frame_bytes_received += frame_size as u64;

        self.decompression_ratios.push(1.0);

        if let Some(last) = self.last_instant {
            let duration = start.duration_since(last);
            self.receive_durations.push(duration);
        }

        self.last_instant = Some(start);
    }

    pub fn record_delta_frame(&mut self, payload_size: usize, reconstructed_size: usize) {
        let start = Instant::now();

        self.total_frames_received += 1;
        self.delta_frames_received += 1;
        self.total_bytes_received += payload_size as u64;
        self.delta_frame_bytes_received += payload_size as u64;

        if reconstructed_size > 0 {
            let ratio = payload_size as f64 / reconstructed_size as f64;
            self.decompression_ratios.push(ratio);
        }

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

        info!(
            "Frames - Total: {}, Full: {}, Delta: {}",
            self.total_frames_received, self.full_frames_received, self.delta_frames_received,
        );

        let total_mb = self.total_bytes_received as f64 / (1024.0 * 1024.0);
        let full_mb = self.full_frame_bytes_received as f64 / (1024.0 * 1024.0);
        let delta_mb = self.delta_frame_bytes_received as f64 / (1024.0 * 1024.0);

        info!(
            "Data Received - Total: {:.2} MB, Full Frames: {:.2} MB, Delta Frames: {:.2} MB",
            total_mb, full_mb, delta_mb
        );

        if self.total_frames_received > 0 {
            let avg_frame_size =
                self.total_bytes_received as f64 / self.total_frames_received as f64;
            info!(
                "Average Frame Size: {:.2} MB",
                avg_frame_size / (1024.0 * 1024.0)
            );
        }

        if !self.decompression_ratios.is_empty() {
            let avg_ratio = self.decompression_ratios.iter().sum::<f64>()
                / self.decompression_ratios.len() as f64;
            let min_ratio = self
                .decompression_ratios
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);
            let max_ratio = self
                .decompression_ratios
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);

            info!(
                "Decompression Ratios - Avg: {:.2}%, Min: {:.2}%, Max: {:.2}%",
                avg_ratio * 100.0,
                min_ratio * 100.0,
                max_ratio * 100.0
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
