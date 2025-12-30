use eframe::egui;

use crate::{run_client, run_server};

pub struct App {
    is_server: bool,
    is_running: bool,
    ip_address: String,
    port: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            is_server: false,
            is_running: false,
            ip_address: "0.0.0.0".to_string(),
            port: "51230".to_string(),
        }
    }
}

impl App {
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
                            println!("Requested to start server on port {}", self.port);
                            let port = self.port.clone();
                            tokio::spawn(async move {
                                if let Err(e) = run_server(port).await {
                                    eprintln!("Server error: {:?}", e);
                                }
                            });
                        } else {
                            println!(
                                "Requested to start client connecting to {}:{}",
                                self.ip_address, self.port
                            );
                            let ip = self.ip_address.clone();
                            let port = self.port.clone();
                            tokio::spawn(async move {
                                if let Err(e) = run_client(ip, port).await {
                                    eprintln!("Client error: {:?}", e);
                                }
                            });
                        }
                    } else {
                        println!("Requested to stop");
                    }
                }
            })
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.draw_central_panel(ctx);
    }
}
