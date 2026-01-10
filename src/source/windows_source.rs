use async_trait::async_trait;
use image::RgbImage;
use tracing::info;
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
};

use crate::{FrameData, FrameSender, source::Source};

pub struct WindowsSource {}

#[async_trait]
impl Source for WindowsSource {
    async fn start(
        &mut self,
        tx: crate::FrameSender,
    ) -> color_eyre::Result<tokio::task::JoinHandle<()>> {
        let join_handle = tokio::spawn(async move {
            let primary_monitor = Monitor::primary().expect("There is no primary monitor.");

            let settings = Settings::new(
                primary_monitor,
                CursorCaptureSettings::Default,
                DrawBorderSettings::Default,
                SecondaryWindowSettings::Default,
                MinimumUpdateIntervalSettings::Default,
                DirtyRegionSettings::Default,
                ColorFormat::Rgba8,
                tx,
            );

            Capture::start(settings).expect("Screen capture failed on WindowsSource");
        });

        Ok(join_handle)
    }
}

struct Capture {
    tx: crate::FrameSender,
}

impl GraphicsCaptureApiHandler for Capture {
    type Flags = FrameSender;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        info!("Created WindowsSource");

        Ok(Self { tx: ctx.flags })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut windows_capture::frame::Frame,
        _capture_control: windows_capture::graphics_capture_api::InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let width = frame.width();
        let height = frame.height();

        let mut data = frame.buffer()?;
        let raw_data = data.as_raw_buffer();
        let rgb_data = crate::utils::pixel_format_utils::rgba8_to_rgb(raw_data, width, height);

        let rgb_image = RgbImage::from_raw(width, height, rgb_data)
            .expect("Failed to create RgbImage from frame data");
        let frame_data = FrameData::new(rgb_image, width, height);

        self.tx.send(frame_data).unwrap();

        Ok(())
    }
}
