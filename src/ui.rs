use std::time::Duration;

use eframe::egui::{self, ViewportBuilder, ViewportId};
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::{
    FrameData, FrameSender, client::run_client, perf::FrameLatencyMetrics, server::run_server,
};

pub struct App {
    tx: FrameSender,
    rx: crossbeam_channel::Receiver<FrameData>,

    is_server: bool,
    is_running: bool,
    ip_address: String,
    port: String,

    current_texture: Option<eframe::egui::TextureHandle>,

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
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.image(&texture);
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

            if let Some(texture) = &mut self.current_texture {
                let image = eframe::egui::ColorImage::from_rgb(
                    [data.width as usize, data.height as usize],
                    &data.data,
                );
                texture.set(image, eframe::egui::TextureOptions::NEAREST);
            } else {
                let image = eframe::egui::ColorImage::from_rgb(
                    [data.width as usize, data.height as usize],
                    &data.data,
                );
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
