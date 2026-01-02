use std::{io::Cursor, os::fd::OwnedFd};

use crate::{FrameSender, source::Source};

use ashpd::desktop::{
    PersistMode,
    screencast::{CursorMode, Screencast, SourceType},
};
use pipewire::{
    self as pw,
    properties::properties,
    spa::{
        param::{
            ParamType,
            format::{FormatProperties, MediaSubtype, MediaType},
            video::VideoFormat,
        },
        pod::serialize::PodSerializer,
        utils::{Fraction, Rectangle, SpaTypes},
    },
};
use tracing::{debug, error, info};

struct StreamUserData {
    format: pw::spa::param::video::VideoInfoRaw,
}

pub struct PipeWireSource {}

#[async_trait::async_trait]
impl Source for PipeWireSource {
    async fn start(&mut self, tx: FrameSender) -> color_eyre::Result<()> {
        let (stream, fd) = Self::open_portal().await?;
        let node_id = stream.pipe_wire_node_id();

        tokio::spawn(async move {
            Self::start_pw_stream(tx, fd, node_id)
                .await
                .expect("failed to start pipewire stream")
        });

        Ok(())
    }
}

impl PipeWireSource {
    async fn open_portal() -> color_eyre::Result<(ashpd::desktop::screencast::Stream, OwnedFd)> {
        let proxy = Screencast::new().await?;
        let session = proxy.create_session().await?;
        proxy
            .select_sources(
                &session,
                CursorMode::Embedded,
                SourceType::Monitor.into(),
                false,
                None,
                PersistMode::DoNot,
            )
            .await?;

        let response = proxy.start(&session, None).await?.response()?;
        let stream = response
            .streams()
            .first()
            .ok_or(color_eyre::eyre::anyhow!("No streams available"))?
            .to_owned();

        let fd = proxy.open_pipe_wire_remote(&session).await?;

        Ok((stream, fd))
    }

    async fn start_pw_stream(tx: FrameSender, fd: OwnedFd, node_id: u32) -> color_eyre::Result<()> {
        pw::init();

        let mainloop = pw::main_loop::MainLoopBox::new(None)?;
        let context = pw::context::ContextBox::new(mainloop.loop_(), None)?;
        let core = context.connect_fd(fd, None)?;

        let data = StreamUserData {
            format: Default::default(),
        };

        let stream = pw::stream::StreamBox::new(
            &core,
            "airscreen",
            properties! {
                *pw::keys::MEDIA_TYPE => "Video",
                *pw::keys::MEDIA_CATEGORY => "Capture",
                *pw::keys::MEDIA_ROLE => "Screen",
            },
        )?;

        let _listener = stream
            .add_local_listener_with_user_data(data)
            .state_changed(|_, _, old, new| {
                debug!("State changed: {:?} -> {:?}", old, new);
            })
            .param_changed(|_, user_data, id, param| {
                let Some(param) = param else {
                    return;
                };
                if id != ParamType::Format.as_raw() {
                    return;
                }

                let (media_type, media_subtype) =
                    match pw::spa::param::format_utils::parse_format(param) {
                        Ok(v) => v,
                        Err(_) => return,
                    };

                if media_type != MediaType::Video || media_subtype != MediaSubtype::Raw {
                    return;
                }

                user_data
                    .format
                    .parse(param)
                    .expect("Failed to parse param changed to VideoInfoRaw");

                debug!("got video format:");
                debug!(
                    "\tformat: {} ({:?})",
                    user_data.format.format().as_raw(),
                    user_data.format.format()
                );
                debug!(
                    "\tsize: {}x{}",
                    user_data.format.size().width,
                    user_data.format.size().height
                );
                debug!(
                    "\tframerate: {}/{}",
                    user_data.format.framerate().num,
                    user_data.format.framerate().denom
                );
            })
            .process(move |stream, user_data| match stream.dequeue_buffer() {
                None => debug!("out of buffers"),
                Some(mut buffer) => {
                    let datas = buffer.datas_mut();
                    if datas.is_empty() {
                        return;
                    }

                    let data = &mut datas[0];

                    let rgb_data = convert_to_rgb(
                        user_data.format.format(),
                        user_data.format.size().width,
                        user_data.format.size().height,
                        data.data().unwrap(),
                    );

                    if let Err(e) = tx.send((
                        rgb_data,
                        user_data.format.size().width,
                        user_data.format.size().height,
                    )) {
                        error!("Failed to send frame: {:?}", e);
                    }
                }
            })
            .register()?;

        info!("Created stream with id {}: {:#?}", node_id, stream);

        let obj = pw::spa::pod::object!(
            SpaTypes::ObjectParamFormat,
            ParamType::EnumFormat,
            pw::spa::pod::property!(FormatProperties::MediaType, Id, MediaType::Video),
            pw::spa::pod::property!(FormatProperties::MediaSubtype, Id, MediaSubtype::Raw),
            pw::spa::pod::property!(
                FormatProperties::VideoFormat,
                Choice,
                Enum,
                Id,
                VideoFormat::RGB,
                VideoFormat::RGB,
                VideoFormat::RGBA,
                VideoFormat::RGBx,
                VideoFormat::BGRx,
                VideoFormat::YUY2,
                VideoFormat::I420,
            ),
            pw::spa::pod::property!(
                FormatProperties::VideoSize,
                Choice,
                Range,
                Rectangle,
                Rectangle {
                    width: 320,
                    height: 240,
                },
                Rectangle {
                    width: 1,
                    height: 1,
                },
                Rectangle {
                    width: 4096,
                    height: 4096,
                }
            ),
            pw::spa::pod::property!(
                FormatProperties::VideoFramerate,
                Choice,
                Range,
                Fraction,
                Fraction { num: 25, denom: 1 },
                Fraction { num: 0, denom: 1 },
                Fraction {
                    num: 1000,
                    denom: 1,
                }
            ),
        );
        let values: Vec<u8> =
            PodSerializer::serialize(Cursor::new(Vec::new()), &pw::spa::pod::Value::Object(obj))?
                .0
                .into_inner();

        let mut params = [pw::spa::pod::Pod::from_bytes(&values)
            .ok_or(color_eyre::eyre::anyhow!("Failed to create pod from bytes"))?];

        stream.connect(
            pw::spa::utils::Direction::Input,
            Some(node_id),
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
            &mut params,
        )?;

        info!("Connected stream");

        mainloop.run();

        Ok(())
    }
}

/// Convert YUV components (with U/V already offset by -128) to RGB 0..255 tuple
/// Written by AI
fn yuv_to_rgb_f(y: f32, u: f32, v: f32) -> (u8, u8, u8) {
    let r = (y + 1.402 * v).round().clamp(0.0, 255.0) as u8;
    let g = (y - 0.344136 * u - 0.714136 * v).round().clamp(0.0, 255.0) as u8;
    let b = (y + 1.772 * u).round().clamp(0.0, 255.0) as u8;
    (r, g, b)
}

/// Convert a buffer in `format` to tightly-packed RGB (3 bytes per pixel).
///
/// Note: This assumes the input buffer is tightly packed (no per-line stride/padding).
/// Written by AI
fn convert_to_rgb(format: VideoFormat, width: u32, height: u32, src: &[u8]) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let pixel_count = w.saturating_mul(h);

    match format {
        VideoFormat::RGB => {
            // Already RGB (3 bytes per pixel)
            let need = pixel_count * 3;
            if src.len() >= need {
                src[0..need].to_vec()
            } else {
                let mut v = src.to_vec();
                v.resize(need, 0u8);
                v
            }
        }
        VideoFormat::RGBA => {
            // RGBA -> drop A
            let mut out = Vec::with_capacity(pixel_count * 3);
            let mut i = 0usize;
            while i + 3 < src.len() && out.len() / 3 < pixel_count {
                out.push(src[i]);
                out.push(src[i + 1]);
                out.push(src[i + 2]);
                i += 4;
            }
            out
        }
        VideoFormat::RGBx => {
            // 4 bytes per pixel, last ignored
            let mut out = Vec::with_capacity(pixel_count * 3);
            let mut i = 0usize;
            while i + 3 < src.len() && out.len() / 3 < pixel_count {
                out.push(src[i]);
                out.push(src[i + 1]);
                out.push(src[i + 2]);
                i += 4;
            }
            out
        }
        VideoFormat::BGRx => {
            // B G R x -> R G B
            let mut out = Vec::with_capacity(pixel_count * 3);
            let mut i = 0usize;
            while i + 3 < src.len() && out.len() / 3 < pixel_count {
                let b = src[i];
                let g = src[i + 1];
                let r = src[i + 2];
                out.push(r);
                out.push(g);
                out.push(b);
                i += 4;
            }
            out
        }
        VideoFormat::BGR => {
            // 3 bytes per pixel B,G,R -> R,G,B
            let mut out = Vec::with_capacity(pixel_count * 3);
            let mut i = 0usize;
            while i + 2 < src.len() && out.len() / 3 < pixel_count {
                let b = src[i];
                let g = src[i + 1];
                let r = src[i + 2];
                out.push(r);
                out.push(g);
                out.push(b);
                i += 3;
            }
            out
        }
        VideoFormat::YUY2 => {
            // Packed YUYV: Y0 U Y1 V  -> two pixels
            let mut out = Vec::with_capacity(pixel_count * 3);
            let mut i = 0usize;
            while i + 3 < src.len() && out.len() / 3 < pixel_count {
                let y0 = src[i] as f32;
                let u = src[i + 1] as f32 - 128.0;
                let y1 = src[i + 2] as f32;
                let v = src[i + 3] as f32 - 128.0;
                let (r0, g0, b0) = yuv_to_rgb_f(y0, u, v);
                let (r1, g1, b1) = yuv_to_rgb_f(y1, u, v);
                out.push(r0);
                out.push(g0);
                out.push(b0);
                if out.len() / 3 < pixel_count {
                    out.push(r1);
                    out.push(g1);
                    out.push(b1);
                }
                i += 4;
            }
            // truncate if over
            out.truncate(pixel_count * 3);
            out
        }
        VideoFormat::I420 => {
            // Planar YUV420 (I420): Y plane (w*h), then U (w/2*h/2), then V (w/2*h/2)
            let y_plane_len = w.saturating_mul(h);
            let uv_w = w.div_ceil(2);
            let uv_h = h.div_ceil(2);
            let uv_plane_len = uv_w.saturating_mul(uv_h);
            if src.len() < y_plane_len + 2 * uv_plane_len {
                return vec![0u8; pixel_count * 3];
            }
            let y_plane = &src[0..y_plane_len];
            let u_plane = &src[y_plane_len..y_plane_len + uv_plane_len];
            let v_plane = &src[y_plane_len + uv_plane_len..y_plane_len + 2 * uv_plane_len];

            let mut out = Vec::with_capacity(pixel_count * 3);
            for yy in 0..h {
                for xx in 0..w {
                    let y = y_plane[yy * w + xx] as f32;
                    let ux = xx / 2;
                    let uy = yy / 2;
                    let u = u_plane[uy * uv_w + ux] as f32 - 128.0;
                    let v = v_plane[uy * uv_w + ux] as f32 - 128.0;
                    let (r, g, b) = yuv_to_rgb_f(y, u, v);
                    out.push(r);
                    out.push(g);
                    out.push(b);
                }
            }
            out
        }
        _ => {
            // Fallback: try to interpret as tightly-packed RGB triplets
            let need = pixel_count * 3;
            if src.len() >= need {
                src[0..need].to_vec()
            } else {
                let mut v = src.to_vec();
                v.resize(need, 0u8);
                v
            }
        }
    }
}
