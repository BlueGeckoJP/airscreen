use std::{sync::mpsc::Receiver, time::Duration};

use eframe::egui::{self, ViewportBuilder, ViewportId};
use tracing::{error, info};

use crate::{FrameData, FrameSender, run_client, run_server};

pub struct App {
    tx: FrameSender,
    rx: Receiver<FrameData>,

    is_server: bool,
    is_running: bool,
    ip_address: String,
    port: String,

    current_texture: Option<eframe::egui::TextureHandle>,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<FrameData>(4);

        App {
            tx,
            rx,
            is_server: false,
            is_running: false,
            ip_address: "0.0.0.0".to_string(),
            port: "51230".to_string(),
            current_texture: None,
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

                    if self.is_running {
                        if self.is_server {
                            info!("Requested to start server on port {}", self.port);
                            let port = self.port.clone();
                            let tx = self.tx.clone();
                            tokio::spawn(async move {
                                if let Err(e) = run_server(port, tx).await {
                                    error!("Server error: {:?}", e);
                                }
                            });
                        } else {
                            info!(
                                "Requested to start client connecting to {}:{}",
                                self.ip_address, self.port
                            );
                            let ip = self.ip_address.clone();
                            let port = self.port.clone();
                            tokio::spawn(async move {
                                if let Err(e) = run_client(ip, port).await {
                                    error!("Client error: {:?}", e);
                                }
                            });
                        }
                    } else {
                        info!("Requested to stop");
                    }
                }
            })
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let mut latest_frame = None;

        while let Ok(data) = self.rx.try_recv() {
            latest_frame = Some(data);
        }

        if let Some(data) = latest_frame {
            if let Some(texture) = &mut self.current_texture {
                let image =
                    eframe::egui::ColorImage::from_rgb([data.1 as usize, data.2 as usize], &data.0);
                texture.set(image, eframe::egui::TextureOptions::NEAREST);
            } else {
                let image =
                    eframe::egui::ColorImage::from_rgb([data.1 as usize, data.2 as usize], &data.0);
                let texture = ctx.load_texture(
                    "current_frame",
                    image,
                    eframe::egui::TextureOptions::NEAREST,
                );
                self.current_texture = Some(texture);
            }
        }

        if self.is_running
            && let Some(texture) = &self.current_texture
        {
            ctx.request_repaint_after(Duration::from_secs_f32(1.0 / 60.0));

            let texture = texture.clone();
            ctx.show_viewport_deferred(
                ViewportId::from_hash_of("frame_viewer"),
                ViewportBuilder::default().with_title("AirScreen Viewer"),
                move |ctx, _class| {
                    ctx.request_repaint_after(Duration::from_secs_f32(1.0 / 60.0));

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.image(&texture);
                    });
                },
            );
        }

        self.draw_central_panel(ctx);
    }
}
