use std::time::{Duration, Instant};
use tracing::info;

#[derive(Default)]
pub struct FrameLatencyMetrics {
    latencies: Vec<Duration>,
    prev_instant: Option<Instant>,
}

impl FrameLatencyMetrics {
    pub fn record_period_latency(&mut self) {
        let now = Instant::now();

        if let Some(prev) = self.prev_instant {
            let latency = now.duration_since(prev);
            self.latencies.push(latency);
            self.prev_instant = None;
        }

        self.prev_instant = Some(now);
    }

    fn log_stats(&self) {
        if self.latencies.is_empty() {
            info!("No frame latencies recorded.");
            return;
        }

        let sum: Duration = self.latencies.iter().sum();
        let avg = sum / self.latencies.len() as u32;
        let min = self.latencies.iter().min().copied().unwrap_or_default();
        let max = self.latencies.iter().max().copied().unwrap_or_default();

        info!(
            "Frame Latency - Avg: {:?}, Min: {:?}, Max: {:?}, Count: {}",
            avg,
            min,
            max,
            self.latencies.len()
        );
    }
}

impl Drop for FrameLatencyMetrics {
    fn drop(&mut self) {
        self.log_stats();
    }
}
