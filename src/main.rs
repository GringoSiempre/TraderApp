// Structs for market data updates
mod market_data;
mod processed_data;

// Traits for pattern Observer
mod observer;

use std::arch::is_aarch64_feature_detected;
pub use observer::{MessagePublisher, MessageSubscriber};

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

use eframe::egui::{self, menu};
use egui::{RichText, ComboBox};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use ring::rand::SecureRandom;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicI64, Ordering};
use futures_util::task::Spawn;
use crate::api::{send_order, Connection, ConnectionStatus, BASE_TICKERS};
use crate::api_utils::*;
use crate::observer::{ConsoleOutputSubscriber, DataDeserializer, MessagesToFileSubscriber, ServerMessagesPublisher, DataProcessor, PortfolioUpdater, QuotesRequester};
use crate::processed_data::{OrderBook, Portfolio, QuoteBook};
use crate::trading_utils::{upgrade_sl, SLStrategy, TickerOptions};

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
    tickers: Arc<RwLock<Vec<TickerOptions>>>,

    server_messages_publisher: ServerMessagesPublisher,
    data_deserializer: DataDeserializer,
    data_processor: DataProcessor,
    portfolio_updater: PortfolioUpdater,
    quotes_requester: QuotesRequester,
    
    data_receiver: mpsc::Receiver<String>,
    display_data: String,

    days_to_expiration: Arc<AtomicI64>,

    error_message: String,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut tickers = Vec::new();
        for ticker in BASE_TICKERS.iter() {
            tickers.push(TickerOptions::new(ticker.to_string()));
        }
        let (data_sender, data_receiver) = mpsc::channel(100);
        let connections = Arc::new(RwLock::new(Vec::new()));
        let order_books = Arc::new(RwLock::new(Vec::new()));
        let quotes = Arc::new(RwLock::new(Vec::new()));
        let portfolios = Arc::new(RwLock::new(Vec::new()));
        let tickers = Arc::new(RwLock::new(tickers));
        let days_to_expiration = Arc::new(AtomicI64::new(1));
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
            tickers: Arc::clone(&tickers),
            server_messages_publisher: ServerMessagesPublisher::new(),
            data_deserializer: DataDeserializer::new(data_sender.clone()),
            data_processor: DataProcessor::new(data_sender, Arc::clone(&order_books), Arc::clone(&quotes), Arc::clone(&tickers), Arc::clone(&days_to_expiration)),
            portfolio_updater: PortfolioUpdater::new(Arc::clone(&portfolios)),
            quotes_requester: QuotesRequester::new(Arc::clone(&connections)),
            data_receiver,
            display_data: String::new(),
            days_to_expiration: Arc::clone(&days_to_expiration),
            error_message: String::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.is_authenticated {
            egui::TopBottomPanel::top("Menu").show(ctx, |ui| {
                menu::bar(ui, |ui| {
                    ui.menu_button("Settings", |ui| {
                        ui.label("Expiration");
                        let mut value = self.days_to_expiration.load(Ordering::Relaxed) as i64;
                        ui.add(
                            egui::Slider::new(&mut value, 0..=5).text("Days to Expiration"),
                        );
                        self.days_to_expiration.store(value, Ordering::Relaxed);
                    });
                });
            });
            egui::CentralPanel::default().show(ctx, |ui| {
                while let Ok(new_data) = self.data_receiver.try_recv() {
                    self.display_data = new_data;
                }
                ui.heading(egui::RichText::new("Credentials").strong());
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_sized(egui::Vec2::new(50.0, 20.0), egui::Label::new("status"));
                        ui.add_sized(egui::Vec2::new(50.0, 20.0), egui::Label::new("id"));
                        ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new("Login"));
                        ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new("Password"));
                        ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new("Public key"));
                        ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new("Secret key"));
                    });
                    let connections_read = self.connections.read().unwrap();
                    for connection in connections_read.iter() {
                        let connection_status = connection.status.clone();
                        ui.separator();
                        ui.horizontal(|ui| {
                            match connection_status {
                                ConnectionStatus::Connected => {
                                    ui.add_sized(
                                        egui::Vec2::new(50.0, 20.0),
                                        egui::Label::new(egui::RichText::new(connection_status.description()).color(egui::Color32::GREEN)),
                                    )}
                                ConnectionStatus::Disconnected => {
                                    ui.add_sized(
                                        egui::Vec2::new(50.0, 20.0),
                                        egui::Label::new(egui::RichText::new(connection_status.description()).color(egui::Color32::RED)),
                                    )}
                            };
                            ui.add_sized(egui::Vec2::new(50.0, 20.0), egui::Label::new(&connection.credentials.id));
                            ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(&connection.credentials.login));
                            ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new("**********"));
                            ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new(&connection.credentials.public_key));
                            ui.add_sized(egui::Vec2::new(180.0, 20.0), egui::Label::new("**********"));
                        });

                        let option_label_size = egui::vec2(150.0, 20.0);
                        let ticker_label_size = egui::vec2(50.0, 20.0);
                        let tickers_read = self.tickers.read().unwrap();
                        for row in tickers_read.iter() {
                            let short_option_text = RichText::new(row.short_option.clone());
                            let ticker_text = RichText::new(row.ticker.clone()).strong();
                            let long_option_text = RichText::new(row.long_option.clone());
                            let button_text_short = RichText::new("SHORT").color(egui::Color32::WHITE).strong();
                            let button_style_short = egui::Button::new(button_text_short).fill(egui::Color32::DARK_RED);
                            let button_text_long = RichText::new("LONG").color(egui::Color32::WHITE).strong();
                            let button_style_long = egui::Button::new(button_text_long).fill(egui::Color32::DARK_GREEN);
                            let public_key = connection.credentials.public_key.clone();
                            let secret_key = connection.credentials.secret_key.clone();
                            ui.horizontal(|ui| {
                                let mut ticket_for_order = "".to_string();
                                ui.add_sized(option_label_size, egui::Label::new(short_option_text));
                                if ui.add(button_style_short).clicked() {
                                    ticket_for_order = row.short_option.clone();
                                }
                                ui.add_sized(ticker_label_size, egui::Label::new(ticker_text));
                                if ui.add(button_style_long).clicked() {
                                    ticket_for_order = row.long_option.clone();
                                }
                                ui.add_sized(option_label_size, egui::Label::new(long_option_text));
                                if ticket_for_order != "" {
                                    tokio::spawn(async move {
                                        if let Err(e) = send_order(public_key, secret_key, ticket_for_order, ActionType::Buy, OrderType::Market, 0.0, 1, Expirations::Day).await
                                        {
                                            eprintln!("Failed to send short order {:?}", e)
                                        }
                                    });
                                }
                            });
                        }
                    }
                });
                ui.label(format!("\n\nlast message from server: {}", self.display_data));

                // Display Portfolios
                ui.separator();
                ui.heading(egui::RichText::new("Portfolios").strong());
                let mut portfolios = self.portfolios.write().unwrap();
                let credentials = self.credentials.clone();
                for portfolio in portfolios.iter_mut() {
                    ui.label(format!("Account id: {}", portfolio.id));

                    ui.horizontal(|ui| {
                        ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(egui::RichText::new("Position ID").strong()));
                        ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(egui::RichText::new("Ticker").strong()));
                        ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new("Quantity").strong()));
                        ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new("Open price").strong()));
                        ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new("Current price").strong()));
                        ui.add_sized(egui::Vec2::new(70.0, 20.0), egui::Label::new(egui::RichText::new("PNL").strong()));
                        ui.add_sized(egui::Vec2::new(70.0, 20.0), egui::Label::new(egui::RichText::new("Strategy").strong()));
                        ui.add_sized(egui::Vec2::new(70.0, 20.0), egui::Label::new(egui::RichText::new("SLType").strong()));
                        ui.add_sized(egui::Vec2::new(70.0, 20.0), egui::Label::new(egui::RichText::new("").strong()));
                        ui.add_sized(egui::Vec2::new(60.0, 20.0), egui::Label::new(egui::RichText::new("SL price").strong()));
                        ui.add_sized(egui::Vec2::new(60.0, 20.0), egui::Label::new(egui::RichText::new("Close alert").strong()));
                        ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new("Manual close").strong()));
                    });
                    for mut row in portfolio.portfolio.iter_mut() {
                        let mut close_alert = row.close_alert;
                        ui.horizontal(|ui| {
                            ui.add_sized(egui::Vec2::new(100.0, 20.0), egui::Label::new(format!("{}", row.position_id)));
                            ui.add_sized(egui::Vec2::new(150.0, 20.0), egui::Label::new(egui::RichText::new(format!("{}", row.ticker)).strong()));
                            ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(egui::RichText::new(format!("{}", row.quantity)).strong()));
                            ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(format!("{:.2}", row.open_price)));
                            ui.add_sized(egui::Vec2::new(80.0, 20.0), egui::Label::new(format!("{:.2}", row.current_price)));
                            ui.add_sized(egui::Vec2::new(70.0, 20.0), egui::Label::new(egui::RichText::new(format!("{:.2}", row.pnl)).strong()));

                            // ui.add_sized(egui::Vec2::new(70.0, 20.0),
                            egui::ComboBox::from_label("Select strategy")
                                .selected_text(row.sl_strategy.description())
                                .show_ui(ui, |ui| {
                                    for strategy in SLStrategy::ALL.iter() {
                                        ui.selectable_value(
                                            &mut row.sl_strategy,
                                            *strategy,
                                            strategy.description(),
                                        );
                                    }
                                });
                            // );
                            // ui.add_sized(egui::Vec2::new(70.0, 20.0), egui::Label::new(egui::RichText::new(format!("{}", row.sl_strategy.description())).strong()));

                            ui.add_sized(egui::Vec2::new(70.0, 20.0), egui::Label::new(egui::RichText::new(format!("{}", row.sl_type.description())).strong()));
                            if ui.button("SLUpgrade").clicked() {
                                (row.sl_type, row.sl_price) = upgrade_sl(&row);
                            }
                            ui.add_sized(egui::Vec2::new(60.0, 20.0), egui::Label::new(egui::RichText::new(format!("{:.2}", row.sl_price)).strong()));
                            ui.add_sized(egui::Vec2::new(60.0, 20.0), egui::Label::new(egui::RichText::new(format!("{}", row.close_alert)).strong()));
                            if ui.button("Close position").clicked() {
                                close_alert = true;
                            }
                        });
                        if close_alert && !row.closing  {
                            row.closing = true;
                            if let Some(creds) = credentials.iter().find (|creds| creds.id == portfolio.id) {
                                // println!("{} / {}", creds.public_key, creds.secret_key);
                                let public_key = creds.public_key.clone();
                                let secret_key = creds.secret_key.clone();
                                let ticket_for_order = row.ticker.clone();
                                let qty_for_order = row.quantity.clone() as u64;
                                tokio::spawn(async move {
                                    if let Err(e) = send_order(public_key, secret_key, ticket_for_order, ActionType::Sell, OrderType::Market, 0.0, qty_for_order, Expirations::Day).await
                                    {
                                        eprintln!("Failed to send short order {:?}", e)
                                    }
                                });
                            }
                        }
                    }
                    ui.separator();
                }

                // Display Order Books
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
                            
                            // self.server_messages_publisher.subscribe(Box::new(ConsoleOutputSubscriber));
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

                            self.credentials = crypto_utils::load_credentials(master_key_slice, "credentials.json.enc");
                            self.credentials = crypto_utils::filter_credentials(self.credentials.clone(), accessible_credentials);
                            for credentials in self.credentials.iter() {
                                let (sender_to_connector, connector_receiver) = mpsc::unbounded_channel();
                                let (sender_to_ui, mut ui_receiver) = mpsc::unbounded_channel();
                                let mut connection = Connection::new(credentials.clone(), sender_to_connector.clone(), sender_to_ui.clone());

                                // initialising connections with broker
                                let mut connections_write = self.connections.write().unwrap();
                                let tickers_for_initial_requests = connection.query_tickers.clone();
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
                                let mut sender = sender_to_connector.clone();
                                let quotes_message = Request::quotes(tickers_for_initial_requests.clone()).message();
                                let order_book_message = Request::order_book(tickers_for_initial_requests.clone()).message();
                                let portfolio_message = Request::portfolio().message();
                                tokio::spawn(async move {
                                    sender.send(quotes_message).unwrap();
                                    sender.send(order_book_message).unwrap();
                                    sender.send(portfolio_message).unwrap();
                                });
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
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 1400.0]),
        ..Default::default()    
    };
    eframe::run_native(
        "TraderApp",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    ).expect("TODO: panic message");
}