use std::time::{Duration, Instant};
use tracing::info;

#[derive(Default)]
pub struct FrameLatencyMetrics {
    texture_latencies: Vec<Duration>,
    prev_instant: Option<Instant>,
}

impl FrameLatencyMetrics {
    pub fn record_texture_latency(&mut self) {
        let now = Instant::now();

        if let Some(prev) = self.prev_instant {
            let latency = now.duration_since(prev);
            self.texture_latencies.push(latency);
            self.prev_instant = None;
        }

        self.prev_instant = Some(now);
    }

    fn log_stats(&self) {
        if self.texture_latencies.is_empty() {
            info!("No frame latencies recorded.");
            return;
        }

        let sum: Duration = self.texture_latencies.iter().sum();
        let avg = sum / self.texture_latencies.len() as u32;
        let min = self
            .texture_latencies
            .iter()
            .min()
            .copied()
            .unwrap_or_default();
        let max = self
            .texture_latencies
            .iter()
            .max()
            .copied()
            .unwrap_or_default();
        let texture_avg_fps = self.texture_latencies.len() as f64
            / self
                .texture_latencies
                .iter()
                .map(|d| d.as_secs_f64())
                .sum::<f64>();

        info!(
            "Frame Latency - Avg: {:?}, Min: {:?}, Max: {:?}, Count: {}, Avg Texture FPS: {:.2}",
            avg,
            min,
            max,
            self.texture_latencies.len(),
            texture_avg_fps
        );
    }
}

impl Drop for FrameLatencyMetrics {
    fn drop(&mut self) {
        self.log_stats();
    }
}
