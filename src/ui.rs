use std::time::Duration;

use eframe::egui::{self, Color32, ColorImage, Vec2, ViewportBuilder, ViewportId};
use rayon::{iter::ParallelIterator, slice::ParallelSlice};
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::{
    FrameReceiver, FrameSender, client::run_client,
    perf::frame_latency_metrics::FrameLatencyMetrics, server::run_server,
};

pub struct App {
    tx: FrameSender,
    rx: FrameReceiver,

    is_server: bool,
    is_running: bool,
    ip_address: String,
    port: String,

    current_texture: Option<eframe::egui::TextureHandle>,
    viewport_available_rect: Vec2,

    metrics: FrameLatencyMetrics,

    join_handles: Vec<JoinHandle<()>>,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = crossbeam_channel::bounded(4);

        App {
            tx,
            rx,
            is_server: false,
            is_running: false,
            ip_address: "0.0.0.0".to_string(),
            port: "51230".to_string(),
            current_texture: None,
            viewport_available_rect: Vec2::new(1920.0, 1080.0),
            metrics: FrameLatencyMetrics::default(),
            join_handles: vec![],
        }
    }

    fn stop_handles(&mut self) {
        for handle in self.join_handles.drain(..) {
            handle.abort();
        }
    }

    fn draw_central_panel(&mut self, ctx: &eframe::egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("AirScreen");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Mode:");
                ui.radio_value(&mut self.is_server, true, "Server / Viewer");
                ui.radio_value(&mut self.is_server, false, "Client / Streamer");
            });
            ui.add_space(15.0);

            if self.is_server {
                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.text_edit_singleline(&mut self.port);
                    })
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("IP Address:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.text_edit_singleline(&mut self.ip_address);
                    })
                });
                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.text_edit_singleline(&mut self.port);
                    })
                });
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                let button_text = if self.is_server {
                    if self.is_running {
                        "Stop Server"
                    } else {
                        "Start Server"
                    }
                } else if self.is_running {
                    "Stop Client"
                } else {
                    "Start Client"
                };

                if ui.button(button_text).clicked() {
                    self.is_running = !self.is_running;

                    if !self.is_running {
                        info!("Requested to stop");
                        self.stop_handles();
                        return;
                    }

                    if self.is_server {
                        info!("Requested to start server on port {}", self.port);
                        let port = self.port.clone();
                        let tx = self.tx.clone();

                        let handle = tokio::spawn(async {
                            match run_server(port, tx).await {
                                Ok(join_handle) => {
                                    if let Err(e) = join_handle.await {
                                        error!("Server task error: {:?}", e);
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to start server: {:?}", e);
                                }
                            }
                        });
                        self.join_handles.push(handle);
                    } else {
                        info!(
                            "Requested to start client connecting to {}:{}",
                            self.ip_address, self.port
                        );
                        let ip = self.ip_address.clone();
                        let port = self.port.clone();

                        let handle = tokio::spawn(async {
                            match run_client(ip, port).await {
                                Ok(join_handles) => {
                                    for handle in join_handles {
                                        if let Err(e) = handle.await {
                                            error!("Client task error: {:?}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to start client: {:?}", e);
                                }
                            }
                        });
                        self.join_handles.push(handle);
                    }
                }
            })
        });
    }

    fn draw_viewer_viewport(&mut self, ctx: &eframe::egui::Context) {
        if !self.is_running || self.current_texture.is_none() {
            return;
        }

        ctx.request_repaint_after(Duration::from_secs_f32(1.0 / 60.0));

        let texture = match &self.current_texture {
            Some(tex) => tex.clone(),
            None => return,
        };

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("frame_viewer"),
            ViewportBuilder::default()
                .with_title("AirScreen Viewer")
                .with_active(true),
            move |ctx, _class| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::default().inner_margin(0.0))
                    .show(ctx, |ui| {
                        let rect = ui.available_rect_before_wrap();
                        self.viewport_available_rect = rect.size();

                        ui.centered_and_justified(|ui| {
                            ui.image(&texture);
                        });
                    });

                if ctx.input(|i| i.viewport().close_requested()) {
                    info!("Requested to stop");
                    self.is_running = false;
                    self.stop_handles();
                }
            },
        );
    }

    fn update_texture(&mut self, ctx: &eframe::egui::Context) {
        let mut latest_frame = None;

        while let Ok(data) = self.rx.try_recv() {
            latest_frame = Some(data);
        }

        if let Some(data) = latest_frame {
            self.metrics.record_period_latency();

            let viewport_available = self.viewport_available_rect;
            let viewport_width = viewport_available.x as u32;
            let viewport_height = viewport_available.y as u32;
            let viewport_aspect = viewport_width as f32 / viewport_height as f32;

            let image_aspect = data.width as f32 / data.height as f32;

            let (new_width, new_height) = if viewport_aspect > image_aspect {
                let h = viewport_height;
                let w = (h as f32 * image_aspect) as u32;
                (w, h)
            } else {
                let w = viewport_width;
                let h = (w as f32 / image_aspect) as u32;
                (w, h)
            };

            let resized = image::imageops::resize(
                &data.data,
                new_width,
                new_height,
                image::imageops::FilterType::Nearest,
            );

            let pixels: Vec<Color32> = resized
                .par_chunks_exact(3)
                .map(|chunk| Color32::from_rgb(chunk[0], chunk[1], chunk[2]))
                .collect();

            let image = ColorImage {
                size: [new_width as usize, new_height as usize],
                source_size: Vec2 {
                    x: new_width as f32,
                    y: new_height as f32,
                },
                pixels,
            };

            if let Some(texture) = &mut self.current_texture {
                texture.set(image, eframe::egui::TextureOptions::NEAREST);
            } else {
                let texture = ctx.load_texture(
                    "current_frame",
                    image,
                    eframe::egui::TextureOptions::NEAREST,
                );
                self.current_texture = Some(texture);
            }

            self.metrics.record_period_latency();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.update_texture(ctx);
        self.draw_viewer_viewport(ctx);
        self.draw_central_panel(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        info!("Exiting application, stopping all tasks");
        self.stop_handles();
    }
}
