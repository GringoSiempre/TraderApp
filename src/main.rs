// Structs for market data updates
mod market_data;
mod processed_data;

// Traits for pattern Observer
mod observer;
pub use observer::{MessagePublisher,MessageSubscriber};

// Security and authorisation functions
pub mod crypto_utils;
use crypto_utils::{User, Credentials};

// API functions
pub mod api;
use api::{connect_to_ff_ws};

// API tools and structures
pub mod api_utils;
use api_utils::Request;

pub mod trading_utils;

use eframe::egui;
use egui::RichText;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use ring::rand::SecureRandom;
use std::sync::{Arc, RwLock};

use crate::api::Connection;
use crate::observer::{ConsoleOutputSubscriber, DataDeserializer, MessagesToFileSubscriber, ServerMessagesPublisher, DataProcessor, PortfolioUpdater, QuotesRequester};
use crate::processed_data::{OrderBook, Portfolio, QuoteBook};

struct MyApp {
    email_input: String,
    password_input: String,
    is_authenticated: bool,
    users: Vec<User>,
    credentials: Vec<Credentials>,
    connections: Arc<RwLock<Vec<Connection>>>,
    order_books: Arc<RwLock<Vec<OrderBook>>>,
    quotes: Arc<RwLock<Vec<QuoteBook>>>,
    portfolios: Arc<RwLock<Vec<Portfolio>>>,
    requests: Vec<Request>,
    tickers: Vec<String>,
    
    server_messages_publisher: ServerMessagesPublisher,
    data_deserializer: DataDeserializer,
    data_processor: DataProcessor,
    portfolio_updater: PortfolioUpdater,
    quotes_requester: QuotesRequester,
    
    data_receiver: mpsc::Receiver<String>,
    display_data: String,
    error_message: String,
}

impl Default for MyApp {
    fn default() -> Self {
        let tickers = vec![
            "QQQ.US".to_string(),
            "SPY.US".to_string(),
            // "+SPY.26NOV2024.P597".to_string(),
        ];
        let (data_sender, data_receiver) = mpsc::channel(100);
        let connections = Arc::new(RwLock::new(Vec::new()));
        let order_books = Arc::new(RwLock::new(Vec::new()));
        let quotes = Arc::new(RwLock::new(Vec::new()));
        let portfolios = Arc::new(RwLock::new(Vec::new()));
        Self {
            email_input: String::new(),
            password_input: String::new(),
            is_authenticated: false,
            users: crypto_utils::load_users(),
            credentials: Vec::new(),
            connections: Arc::clone(&connections),
            order_books: Arc::clone(&order_books),
            quotes: Arc::clone(&quotes),
            portfolios: Arc::clone(&portfolios),
            requests: vec![
                Request::quotes(tickers.clone()),
                Request::portfolio(), 
                Request::order_book(tickers.clone()),
            ],
            tickers: tickers,
            server_messages_publisher: ServerMessagesPublisher::new(),
            data_deserializer: DataDeserializer::new(data_sender.clone()),
            data_processor: DataProcessor::new(data_sender, Arc::clone(&order_books), Arc::clone(&quotes)),
            portfolio_updater: PortfolioUpdater::new(Arc::clone(&portfolios)),
            quotes_requester: QuotesRequester::new(Arc::clone(&connections)),
            data_receiver: data_receiver,
            display_data: String::new(),
            error_message: String::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.is_authenticated {
            egui::CentralPanel::default().show(ctx, |ui| {
                while let Ok(new_data) = self.data_receiver.try_recv() {
                    self.display_data = new_data;
                }
                ui.heading(egui::RichText::new("Credentials").strong());
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_sized(egui::Vec2::new(50.0, 20.0), egui::Label::new("id"));
                        ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new("Login"));
                        ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new("Password"));
                        ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new("Public key"));
                        ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new("Secret key"));
                    });
                    let connections_read = self.connections.read().unwrap();
                    for connection in connections_read.iter() {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_sized(egui::Vec2::new(50.0, 20.0), egui::Label::new(&connection.credentials.id));
                            ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(&connection.credentials.login));
                            ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new("**********"));
                            ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new(&connection.credentials.public_key));
                            ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new("**********"));
                        });
                    }
                });
                ui.label(format!("\n\nlast message from server: {}", self.display_data));

                // Display Portfolios
                ui.separator();
                ui.heading(egui::RichText::new("Portfolios").strong());
                let portfolios = self.portfolios.read().unwrap();
                for portfolio in portfolios.iter() {
                    ui.label(format!("Account id: {}", portfolio.id));
                    ui.horizontal(|ui| {
                        ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new("Position ID").strong()));
                        ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(egui::RichText::new("Ticker").strong()));
                        ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new("Quantity").strong()));
                        ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new("Open price").strong()));
                        ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new("Current price").strong()));
                        ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new("PNL").strong()));
                        ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new("SLType").strong()));
                        ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new("SL price").strong()));
                        ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new("Close alert").strong()));
                    });
                    for row in portfolio.portfolio.iter() {
                        ui.horizontal(|ui| {
                            ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{}", row.position_id)));
                            ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(egui::RichText::new(format!("{}", row.ticker)).strong()));
                            ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new(format!("{}", row.quantity)).strong()));
                            ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(format!("{:.2}", row.open_price)));
                            ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(format!("{:.2}", row.current_price)));
                            ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new(format!("{:.2}", row.pnl)).strong()));
                            ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new(format!("{}", row.sl_type.description())).strong()));
                            ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new(format!("{:.2}", row.sl_price)).strong()));
                            ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new(format!("{}", row.close_alert)).strong()));
                        });
                    }
                }

                // Display Order Books
                ui.separator();
                ui.heading(egui::RichText::new("Order books").strong());
                let order_books = self.order_books.read().unwrap();
                for order_book in order_books.iter() {
                    ui.label(format!("Account id: {}", order_book.id));
                    ui.horizontal(|ui| {
                        ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(egui::RichText::new("Ticker").strong()));
                        ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new("Side").strong()));
                        ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new("Price").strong()));
                        ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new("Quantity").strong()));
                        ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new("Position").strong()));
                        ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new("Message number").strong()));
                    });
                    for (ticker, block) in &order_book.order_book {
                        for row in &block.buy_rows {
                            ui.horizontal(|ui| {
                                ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(format!("{}", ticker)));
                                ui.add_sized(
                                    egui::Vec2::new(100.0, 20.0), 
                                    egui::Label::new(egui::RichText::new("Buy").color(egui::Color32::DARK_GREEN)),
                                );
                                ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{:.2}", row.price)));
                                ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{}", row.quantity)));
                                ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{}", row.position)));
                                ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{}", row.message_number)));
                            });
                        }
                        for row in &block.sell_rows {
                            ui.horizontal(|ui| {
                                ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(format!("{}", ticker)));
                                ui.add_sized(
                                    egui::Vec2::new(100.0, 20.0),
                                    egui::Label::new(egui::RichText::new("Sell").color(egui::Color32::DARK_RED)),
                                );
                                ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{:.2}", row.price)));
                                ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{}", row.quantity)));
                                ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{}", row.position)));
                                ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{}", row.message_number)));
                            });
                        }
                    }
                }

                // Display Quotes
                ui.separator();
                ui.heading(egui::RichText::new("Quotes").strong());
                let quotes = self.quotes.read().unwrap();
                for quotes_book in quotes.iter() {
                    ui.label(format!("Account id: {}", quotes_book.id));
                    ui.horizontal(|ui| {
                        ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(egui::RichText::new("Ticker").strong()));
                        ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new("Bid price").strong()));
                        ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new("Ask price").strong()));
                        ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new("Last trade").strong()));
                        ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new("Last trade time").strong()));
                    });
                    for row in quotes_book.quotes_list.iter() {
                        ui.horizontal(|ui| {
                            ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(format!("{}", row.ticker.clone().unwrap_or_else(|| "N/A".to_string()))));
                            ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new(format!("{:.2}", row.bid_price.clone().unwrap_or(0.0))).strong()));
                            ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new(format!("{:.2}", row.ask_price.clone().unwrap_or(0.0))).strong()));
                            ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{:.2}", row.last_trade.clone().unwrap_or(0.0))));
                            ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{}", row.last_trade_time.clone().unwrap_or_else(|| "N/A".to_string()))));
                        });
                    }
                }
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
                            
                            self.server_messages_publisher.subscribe(Box::new(ConsoleOutputSubscriber));
                            // self.server_messages_publisher.subscribe(Box::new(MessagesToFileSubscriber::new("messages.log".to_string())));
                            self.server_messages_publisher.subscribe(Box::new(self.data_deserializer.clone()));
                            self.data_deserializer.subscribe(Box::new(self.data_processor.clone()));
                            self.data_processor.subscribe(Box::new(self.portfolio_updater.clone()));
                            self.data_processor.subscribe(Box::new(self.quotes_requester.clone()));
                            
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
                                let (sender_to_connector, connector_receiver) = mpsc::unbounded_channel();
                                let (sender_to_ui, mut ui_receiver) = mpsc::unbounded_channel();
                                let connection = Connection::new(credentials.clone(), sender_to_connector.clone(), sender_to_ui.clone());

                                // initialising connections with broker
                                let mut connections_write = self.connections.write().unwrap();
                                connections_write.push(connection);
                                let credentials_clone = credentials.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = connect_to_ff_ws(credentials_clone.clone(), connector_receiver, sender_to_ui.clone()).await {
                                        eprintln!("Failed to connect to {}, {}", credentials_clone.id.clone(), e)
                                    }
                                });
                                // Receiving messages from connector
                                let credentials_clone = credentials.clone();
                                let mut publisher = self.server_messages_publisher.clone();
                                tokio::spawn(async move {
                                    while let Some(message) = ui_receiver.recv().await {
                                        publisher.notify_subscribers(&credentials_clone.id.clone(), chrono::Local::now(), &message);
                                    }
                                });
                                // Sending initial requests
                                for request in &self.requests {
                                    let mut sender = sender_to_connector.clone();
                                    let message = request.message().clone();
                                    tokio::spawn(async move {
                                        sender.send(message).unwrap();
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
            .with_inner_size([1000.0, 1600.0]),
        ..Default::default()    
    };
    eframe::run_native(
        "TraderApp",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    ).expect("TODO: panic message");
}