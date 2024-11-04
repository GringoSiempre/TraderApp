// Structs for market data updates
mod market_data;
pub use market_data::MarketData;

// Traits for pattern Observer
mod traits;
pub use traits::{Publisher, Subscriber};

// Security and authorisation functions
pub mod crypto_utils;
use crypto_utils::{User, Credentials};

use eframe::egui;
use egui::RichText;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use rand::rngs::OsRng;
use ring::rand::{SecureRandom, SystemRandom};

struct MyApp {
    email_input: String,
    password_input: String,
    is_authenticated: bool,
    users: Vec<User>,
    show_time: bool,
    error_message: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            email_input: String::new(),
            password_input: String::new(),
            is_authenticated: false,
            users: crypto_utils::load_users(),
            show_time: false,
            error_message: String::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.is_authenticated {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Configure button size.
                    let size = egui::Vec2::splat(60.0); // Размер кнопки.
                    // Button initialize.
                    if ui.add_sized(size, egui::Button::new("Time")).clicked() {
                        self.show_time = true;
                    }
                    // Display current time when button is pressed.
                    if self.show_time {
                        let now = Local::now();
                        ui.label(now.format("%H:%M:%S").to_string());
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
                            let decrypted_master_key = crypto_utils::decrypt_data(&encrypted_master_key, &derived_key);
                            let master_key = base64::decode(&decrypted_master_key).expect("Failed to decode decrypted_master_key");
                            let master_key_slice: &[u8; 32] = master_key.as_slice().try_into().expect("Invalid master key length");
                            println!("{}", decrypted_master_key);
                            
                            // let credentials = Credentials {
                            //     login: "ss.cz@icloud.com".to_string(),
                            //     password: "Freedom.asf10-WAxBh=".to_string(),
                            //     public_key: "1805d613d75aa24e9993a9fd0dc46373".to_string(),
                            //     secret_key: "a915db4dbde1c9889de62d7a57c42c2e097fbc54".to_string(),
                            // };
                            // crypto_utils::save_encrypted_credentials(&credentials, master_key_slice, "credentials.json.enc");
                            
                            let credentials2 = crypto_utils::load_credentials(master_key_slice, "credentials.json.enc");
                            println!("{}", credentials2.login);
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

fn main() {
    // crypto_utils::register_user("user1@mail.com","123");
    // crypto_utils::register_user("user2@mail.com","234");
    // crypto_utils::register_user("user3@mail.com","345");

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "TraderApp",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    ).expect("TODO: panic message");
}