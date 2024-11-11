// Structs for market data updates
mod market_data;
pub use market_data::MarketData;

// Traits for pattern Observer
mod observer;
pub use observer::{Publisher, Subscriber};

// Security and authorisation functions
pub mod crypto_utils;
use crypto_utils::{User, Credentials};

// API functions
pub mod api;
use api::{connect_to_ff_ws};

// API tools and structures
pub mod api_utils;
use api_utils::Request;

use eframe::egui;
use egui::RichText;
use serde::{Deserialize, Serialize};
use std::fs;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use ring::rand::SecureRandom;

use crate::api::Connection;

struct MyApp {
    email_input: String,
    password_input: String,
    is_authenticated: bool,
    is_connected: bool,
    users: Vec<User>,
    credentials: Vec<Credentials>,
    connections: Vec<Connection>,
    requests: Vec<Request>,
    tickers: Vec<String>,
    error_message: String,
}

impl Default for MyApp {
    fn default() -> Self {
        let tickers = vec![
            "QQQ.US".to_string(),
            "SPY.US".to_string()
        ];
        Self {
            email_input: String::new(),
            password_input: String::new(),
            is_authenticated: false,
            is_connected: false,
            users: crypto_utils::load_users(),
            credentials: Vec::new(),
            connections: Vec::new(),
            requests: vec![
                Request::quotes(tickers.clone()), 
                Request::portfolio(), 
                Request::order_book(tickers.clone()),
            ],
            tickers: tickers,
            error_message: String::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.is_authenticated {
            egui::CentralPanel::default().show(ctx, |ui| {
                if !self.is_connected {
                    if ui.button("Connect to WebSocket").clicked() {
                        self.is_connected = true;
                    }
                    ui.label("Press the button to connect to WebSocket");
                } else {
                    ui.label("Connection established");
                }

                ui.heading("Credentials data");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_sized(egui::Vec2::new(50.0, 20.0), egui::Label::new("id"));
                        ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new("Login"));
                        ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new("Password"));
                        ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new("Public key"));
                        ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new("Secret key"));
                    });
                    for cred in &self.credentials {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_sized(egui::Vec2::new(50.0, 20.0), egui::Label::new(&cred.id));
                            ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(&cred.login));
                            ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(&cred.password));
                            ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new(&cred.public_key));
                            ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new(&cred.secret_key));
                        });
                    }
                });

            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Login");
                ui.add_space(10.0);
                ui.label("Email:");
                ui.text_edit_singleline(&mut self.email_input);
                ui.label("Password:");
                ui.add(egui::TextEdit::singleline(&mut self.password_input).password(true));

                if ui.button("Login").clicked() {
                    // Search user by email
                    if let Some(user) = self.users.iter().find(|u| u.email == self.email_input) {
                        let parsed_hash = PasswordHash::new(user.password_hash.as_str()).unwrap();
                        // Check password
                        if Argon2::default()
                            .verify_password(self.password_input.as_bytes(), &parsed_hash)
                            .is_ok()
                        {
                            self.is_authenticated = true;
                            self.error_message.clear();
                            let derived_key = crypto_utils::derive_key_from_password(&self.password_input);
                            let encrypted_master_key = base64::decode(&user.encrypted_master_key).expect("Failed to decode encrypted_master_key");
                            let decrypted_master_key = crypto_utils::decrypt_data(&encrypted_master_key, &derived_key); // Master key. Human view
                            let master_key = base64::decode(&decrypted_master_key).expect("Failed to decode decrypted_master_key");
                            let master_key_slice: &[u8; 32] = master_key.as_slice().try_into().expect("Invalid master key length");
                            let encrypted_accessible_credentials = base64::decode(&user.accessible_credentials).expect("Failed to decode encrypted_accessible_credentials");
                            let accessible_credentials = crypto_utils::decrypt_data(&encrypted_accessible_credentials, &derived_key); // List of accessible credentials.

                            // The place for saving credentials code 
                            
                            self.credentials = crypto_utils::load_credentials(master_key_slice, "credentials.json.enc");
                            self.credentials = crypto_utils::filter_credentials(self.credentials.clone(), accessible_credentials);
                            for credentials in self.credentials.iter() {
                                let connection = Connection::new(credentials.clone());
                                let conn_receiver = connection.channels.conn_rx.clone();
                                let conn_sender  = connection.channels.conn_tx.clone();
                                self.connections.push(connection.clone());
                                let credentials_clone = credentials.clone(); 
                                tokio::spawn(async move {
                                    if let Err(e) = connect_to_ff_ws(credentials_clone.clone(), conn_receiver, conn_sender).await {
                                        eprintln!("Failed to connect to {}, {}", credentials_clone.id.clone(), e)
                                    }
                                });
                                // Incoming message handler
                                let main_receiver = connection.channels.main_rx.clone();
                                let credentials_clone = credentials.clone();
                                tokio::spawn(async move {
                                    let mut main_receiver = main_receiver.lock().await;
                                    // let main_sender = main_sender.clone();
                                    while let Some(message) = main_receiver.recv().await {
                                        println!("{} {}: {}", credentials_clone.id.clone(), chrono::Local::now(), message);
                                        // let main_sender =main_sender.lock().await;
                                    }
                                });
                                // Sending initial requests
                                for request in &self.requests {
                                    let main_sender = connection.channels.main_tx.clone();
                                    let message = request.message().clone();
                                    tokio::spawn(async move {
                                        let main_sender = main_sender.lock().await;
                                        main_sender.send(message).unwrap();
                                    });
                                };
                            }
                        } else {
                            self.error_message = "Wrong password".to_string();
                        }
                    } else {
                        self.error_message = "User not found".to_string();
                    }
                    self.password_input.clear();
                }
                if !self.error_message.is_empty() {
                    let error_text = RichText::new(&self.error_message)
                        .color(egui::Color32::LIGHT_RED)
                        .strong();
                    ui.label(error_text);
                }
            });
        }
        ctx.request_repaint();
    }
}

#[tokio::main]
async fn main() {
    // The place for registering new users 

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 800.0]),
        ..Default::default()    
    };
    eframe::run_native(
        "TraderApp",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    ).expect("TODO: panic message");
}