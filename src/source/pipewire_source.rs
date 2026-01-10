use std::{io::Cursor, os::fd::OwnedFd};

use crate::{FrameData, FrameSender, source::Source};

use ashpd::desktop::{
    PersistMode,
    screencast::{CursorMode, Screencast, SourceType},
};
use image::RgbImage;
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
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

struct StreamUserData {
    format: pw::spa::param::video::VideoInfoRaw,
}

pub struct PipeWireSource {}

#[async_trait::async_trait]
impl Source for PipeWireSource {
    async fn start(&mut self, tx: FrameSender) -> color_eyre::Result<JoinHandle<()>> {
        let (stream, fd) = Self::open_portal().await?;
        let node_id = stream.pipe_wire_node_id();

        let join_handle = tokio::spawn(async move {
            Self::start_pw_stream(tx, fd, node_id)
                .await
                .expect("failed to start pipewire stream")
        });

        Ok(join_handle)
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

                info!("got video format:");
                info!(
                    "\tformat: {} ({:?})",
                    user_data.format.format().as_raw(),
                    user_data.format.format()
                );
                info!(
                    "\tsize: {}x{}",
                    user_data.format.size().width,
                    user_data.format.size().height
                );
                info!(
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

                    let width = user_data.format.size().width;
                    let height = user_data.format.size().height;
                    let data = data.data().unwrap();

                    let rgb_data =
                        crate::utils::pixel_format_utils::rgba8_to_rgb(data, width, height);

                    let rgb_image = RgbImage::from_raw(width, height, rgb_data).unwrap();

                    let frame_data = FrameData::new(
                        rgb_image,
                        user_data.format.size().width,
                        user_data.format.size().height,
                    );
                    if let Err(e) = tx.send(frame_data) {
                        error!("Failed to send frame: {:?}", e);
                        stream.disconnect().unwrap();
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
