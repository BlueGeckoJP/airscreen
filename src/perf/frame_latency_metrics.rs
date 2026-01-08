use std::time::{Duration, Instant};
use tracing::info;

#[derive(Default)]
pub struct FrameLatencyMetrics {
    texture_latencies: Vec<Duration>,
    texture_prev_instant: Option<Instant>,

    draw_parent_vp_latencies: Vec<Duration>,
    draw_parent_vp_prev_instant: Option<Instant>,

    draw_image_vp_latencies: Vec<Duration>,
    draw_image_vp_prev_instant: Option<Instant>,

    total_latencies: Vec<Duration>,
    total_prev_instant: Option<Instant>,
}

impl FrameLatencyMetrics {
    fn record(latencies: &mut Vec<Duration>, prev_instant: &mut Option<Instant>) {
        let now = Instant::now();

        if let Some(prev) = prev_instant {
            let latency = now.duration_since(*prev);
            latencies.push(latency);
        }

        *prev_instant = Some(now);
    }

    pub fn record_texture_latency(&mut self) {
        Self::record(&mut self.texture_latencies, &mut self.texture_prev_instant);
    }

    pub fn record_draw_parent_vp_latency(&mut self) {
        Self::record(
            &mut self.draw_parent_vp_latencies,
            &mut self.draw_parent_vp_prev_instant,
        );
    }

    pub fn record_draw_image_vp_latency(&mut self) {
        Self::record(
            &mut self.draw_image_vp_latencies,
            &mut self.draw_image_vp_prev_instant,
        );
    }

    pub fn record_total_latency(&mut self) {
        Self::record(&mut self.total_latencies, &mut self.total_prev_instant);
    }

    fn summarize(durations: &[Duration], title: &str) -> String {
        let sum = durations.iter().sum::<Duration>();
        let avg = sum / durations.len() as u32;
        let min = *durations.iter().min().unwrap_or(&Duration::ZERO);
        let max = *durations.iter().max().unwrap_or(&Duration::ZERO);
        let avg_fps =
            durations.len() as f64 / durations.iter().map(|d| d.as_secs_f64()).sum::<f64>();

        format!(
            "{} - Avg: {:?}, Min: {:?}, Max: {:?}, Count: {}, Theoretical Avg FPS: {:.2}",
            title,
            avg,
            min,
            max,
            durations.len(),
            avg_fps
        )
    }

    fn log_stats(&self) {
        let _span = tracing::info_span!("Frame Latency Metrics").entered();

        let metrics = [
            (&self.texture_latencies, "Texture Latency"),
            (
                &self.draw_parent_vp_latencies,
                "Draw Parent Viewport Latency",
            ),
            (&self.draw_image_vp_latencies, "Draw Image Viewport Latency"),
            (&self.total_latencies, "Total Frame Latency"),
        ];

        for (latencies, title) in metrics {
            if latencies.is_empty() {
                info!("No {} recorded.", title);
            } else {
                let result = Self::summarize(latencies, title);
                info!("{}", result);
            }
        }
    }
}

impl Drop for FrameLatencyMetrics {
    fn drop(&mut self) {
        self.log_stats();
    }
}
