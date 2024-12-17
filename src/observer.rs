use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::task;
use tokio::sync::mpsc;
use crate::market_data::{deserialize_message, MarketData};
use crate::processed_data::*;
use crate::trading_utils::*;
use crate::api::*;
use crate::api_utils::*;

pub trait MessageSubscriber: Send + Sync {
    fn on_data(&mut self, id: &str, timestamp: chrono::DateTime<chrono::Local>, data: &str);
}
pub trait MessagePublisher {
    fn subscribe(&mut self, subscriber: Box<dyn MessageSubscriber>);
    fn notify_subscribers(&mut self, id: &str, timestamp: chrono::DateTime<chrono::Local>, data: &str);
}
pub trait MarketDataUpdateSubscriber: Send + Sync {
    fn on_data(&mut self, id: &str, market_data: &MarketData);
}
pub trait ProcessedDataSubscriber: Send + Sync {
    fn on_data(&mut self, id: &str, positions: Vec<Position>);
}
pub trait PortfolioUpdaterSubscriber: Send + Sync {
    fn on_data(&mut self, id: &str);
}

// Server messages publisher
#[derive(Clone)]
pub struct ServerMessagesPublisher {
    subscribers: Arc<Mutex<Vec<Box<dyn MessageSubscriber>>>>,
}
impl ServerMessagesPublisher {
    pub fn new() -> Self {
        Self { subscribers: Arc::new(Mutex::new(Vec::new())), }
    }
}
impl MessagePublisher for ServerMessagesPublisher {
    fn subscribe(&mut self, subscriber: Box<dyn MessageSubscriber>) {
        self.subscribers.lock().unwrap().push(subscriber);
    }
    fn notify_subscribers(&mut self, id: &str, timestamp: chrono::DateTime<chrono::Local>, data: &str) {
        let subscribers = Arc::clone(&self.subscribers);
        let data = data.to_string();
        let id = id.to_string();
        task::spawn(async move {
            let mut subs = subscribers.lock().unwrap();
            for subscriber in subs.iter_mut() {
                subscriber.on_data(&id, timestamp, &data);
            }
        });
    }
}

// Console output Subscriber
pub struct ConsoleOutputSubscriber;
impl MessageSubscriber for ConsoleOutputSubscriber {
    fn on_data(&mut self, id: &str, timestamp: chrono::DateTime<chrono::Local>, data: &str) {
        println!("{} {} {}", id, timestamp, data);
    }
}

// Save messages from server to file Subscriber
pub struct MessagesToFileSubscriber {
    file_path: String,
}
impl MessagesToFileSubscriber {
    pub fn new(file_path: String) -> Self {
        Self { file_path }
    }
}
impl MessageSubscriber for MessagesToFileSubscriber {
    fn on_data(&mut self, id: &str, timestamp: chrono::DateTime<chrono::Local>, data: &str) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .expect("Unable to open file");
        writeln!(file, "{} {} {}", id, timestamp, data).expect("Failed to write to file");
    }
}

// Deserialize messages from server Subscriber
#[derive(Clone)]
pub struct DataDeserializer {
    data_sender: mpsc::Sender<String>,
    subscribers: Arc<Mutex<Vec<Box<dyn MarketDataUpdateSubscriber>>>>,
}
impl DataDeserializer {
    pub fn new(data_sender: mpsc::Sender<String>) -> Self {
        Self { 
            data_sender,
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn subscribe(&mut self, subscriber: Box<dyn MarketDataUpdateSubscriber>) {
        self.subscribers.lock().unwrap().push(subscriber);
    }
    pub fn notify_subscribers(&mut self, id: &str, market_data_update: MarketData) {
        let subscribers = Arc::clone(&self.subscribers);
        let id = id.to_string();
        tokio::spawn(async move {
            let mut subscribers = subscribers.lock().unwrap();
            for subscriber in subscribers.iter_mut() {
                subscriber.on_data(&id, &market_data_update);
            }
        });
    }
}
impl MessageSubscriber for DataDeserializer {
    fn on_data(&mut self, id: &str, timestamp: chrono::DateTime<chrono::Local>, data: &str) {
        match deserialize_message(data) {
            Some(deserialized_data) => {
                // let message = serde_json::to_string_pretty(&deserialized_data).unwrap(); 
                // let _ = self.data_sender.try_send(message);
                self.notify_subscribers(id, deserialized_data);
            },
            None => {
                let _ = self.data_sender.try_send("Unable to deserialize message".to_string());
            },
        }
    }
}

// Update market data Subscriber
#[derive(Clone)]
pub struct DataProcessor {
    data_sender: mpsc::Sender<String>,
    order_books: Arc<RwLock<Vec<OrderBook>>>,
    quotes: Arc<RwLock<Vec<QuoteBook>>>,
    tickers: Arc<RwLock<Vec<TickerOptions>>>,
    days_to_expiration: Arc<AtomicI64>,
    subscribers: Arc<Mutex<Vec<Box<dyn ProcessedDataSubscriber>>>>,
}
impl DataProcessor {
    pub fn new(
        data_sender: mpsc::Sender<String>, 
        order_books: Arc<RwLock<Vec<OrderBook>>>, 
        quotes: Arc<RwLock<Vec<QuoteBook>>>,
        tickers: Arc<RwLock<Vec<TickerOptions>>>,
        days_to_expiration: Arc<AtomicI64>,
    ) -> Self {
        Self {
            data_sender,
            order_books,
            quotes,
            tickers,
            days_to_expiration,
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn subscribe(&mut self, subscriber: Box<dyn ProcessedDataSubscriber>) {
        self.subscribers.lock().unwrap().push(subscriber);
    }
    pub fn notify_subscribers(&mut self, id: &str, positions: Vec<Position>) {
        let subscribers = Arc::clone(&self.subscribers);
        let id = id.to_string();
        tokio::spawn(async move {
            let mut subscribers = subscribers.lock().unwrap();
            for subscriber in subscribers.iter_mut() {
                subscriber.on_data(&id, positions.clone());
            }
        });
    }
}
impl MarketDataUpdateSubscriber for DataProcessor {
    fn on_data(&mut self, id: &str, market_data: &MarketData) {
        let mut positions = Vec::new();
        match market_data {
            MarketData::OrderBookMessage(order_book_message) => {
                let mut order_books = self.order_books.write().unwrap();
                let order_book = if let Some(order_book) = order_books.iter_mut().find (|order_book| order_book.id == id) {
                    order_book        
                } else {
                    order_books.push(OrderBook::new(id));
                    order_books.last_mut().unwrap()
                };
                for ins_entry in &order_book_message.ins {
                    let row = OrderBookRow {
                        price: ins_entry.p,
                        side: match ins_entry.s.as_str() {
                            "B" => Side::Buy,
                            "S" => Side::Sell,
                            _ => continue,
                        },
                        quantity: ins_entry.q,
                        position: ins_entry.k,
                        message_number: order_book_message.n,
                    };
                    let mut tickers = self.tickers.write().unwrap();
                    // Обновляем тикеры опционов для отслеживаемых базовых активов
                    if let Some(ticker_row) = tickers.iter_mut().find (|ticker_row| ticker_row.ticker == order_book_message.i) {
                        ticker_row.update(row.side.clone(), row.price.clone(), self.days_to_expiration.load(Ordering::Relaxed));
                    }

                    order_book.add_row(&order_book_message.i, row.clone());
                    if row.side == Side::Buy {
                        let price_update = Position {
                            position_id: 0,
                            ticker: order_book_message.i.clone(),
                            quantity: 0,
                            open_price: 0.0,
                            current_price: ins_entry.p,
                            pnl: 0.0,
                            sl_strategy: SLStrategy::WithoutStops,
                            sl_type: SLType::None,
                            sl_price: 0.0,
                            close_alert: false,
                            closing: false,
                        };
                        positions.push(price_update);
                    };
                }
                for del_entry in &order_book_message.del {
                    order_book.remove_row(del_entry.p);
                }
                for upd_entry in &order_book_message.upd {
                    order_book.update_row(upd_entry.p, upd_entry.q, order_book_message.n);
                }
            }
            MarketData::QuoteMessage(quote_message) => {
                let mut quotes = self.quotes.write().unwrap();
                let quote_book = if let Some(quote_book) = quotes.iter_mut().find (|quote_book| quote_book.id == id) {
                    quote_book
                } else {
                    quotes.push(QuoteBook::new(id));
                    quotes.last_mut().unwrap()
                };
                let quote_data = QuoteData {
                    ticker: quote_message.clone().c,
                    ask_price: quote_message.bap,
                    bid_price: quote_message.bbp,
                    last_trade: quote_message.ltp,
                    last_trade_time: quote_message.clone().ltt,
                };
                quote_book.add_quote(quote_data);
            }
            MarketData::PortfolioMessage(portfolio_message) => {
                for pos_entry in &portfolio_message.pos {
                    let position = Position {
                        position_id: pos_entry.acc_pos_id,
                        ticker: pos_entry.i.to_string(),
                        quantity: pos_entry.q,
                        open_price: pos_entry.price_a,
                        current_price: 0.0,
                        pnl: 0.0,
                        sl_strategy: SLStrategy::WithoutStops,
                        sl_type: SLType::None,
                        sl_price: 0.0,
                        close_alert: false,
                        closing: false,
                    };
                    positions.push(position);
                }
            }
        }
        self.notify_subscribers(id, positions);
    }
}

#[derive(Clone)]
pub struct PortfolioUpdater {
    portfolios: Arc<RwLock<Vec<Portfolio>>>,
    subscribers: Arc<Mutex<Vec<Box<dyn PortfolioUpdaterSubscriber>>>>,
}
impl PortfolioUpdater {
    pub fn new(
        portfolios: Arc<RwLock<Vec<Portfolio>>>
    ) -> Self {
        Self {
            portfolios,
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn subscribe(&mut self, subscriber: Box<dyn PortfolioUpdaterSubscriber>) {
        self.subscribers.lock().unwrap().push(subscriber);
    }
    pub fn notify_subscribers(&mut self, id: &str) {
        let subscribers = Arc::clone(&self.subscribers);
        let id = id.to_string();
        tokio::spawn(async move {
            let mut subscribers = subscribers.lock().unwrap();
            for subscriber in subscribers.iter_mut() {
                subscriber.on_data(&id);
            }
        });
    }
}
impl ProcessedDataSubscriber for PortfolioUpdater {
    fn on_data(&mut self, id: &str, positions: Vec<Position>) {
        let mut portfolios = self.portfolios.write().unwrap();
        let portfolio = if let Some(portfolio) = portfolios.iter_mut().find (|portfolio| portfolio.id == id) {
            portfolio
        } else {
            portfolios.push(Portfolio::new(id));
            portfolios.last_mut().unwrap()
        };
        for position_update in positions {
            // If this is an update to the current price
            if position_update.position_id == 0 {
                if let Some(position) = portfolio.portfolio.iter_mut().find(|position| position.ticker == position_update.ticker) {
                    position.current_price = position_update.current_price;
                    position.pnl = ( position.current_price - position.open_price ) * position.quantity as f64;
                    // Checking stop-loss
                    (position.sl_type, position.sl_price, position.close_alert) = check_sl(&position);
                }
            } else {
                if let Some(position) = portfolio.portfolio.iter_mut().find(|position| position.ticker == position_update.ticker) {
                    // If the quantity in the position is zeroed (when closing a position)
                    if position_update.quantity == 0 {
                        portfolio.portfolio.retain(|position| position.ticker != position_update.ticker);
                    // If the quantity and current price have changed (when adding a position)
                    } else {
                        position.open_price = position_update.open_price;
                        position.quantity = position_update.quantity;
                    }
                // If this is a new position
                } else {
                    portfolio.portfolio.push(position_update);
                }
            }
        };
    }
}

#[derive(Clone)]
pub struct QuotesRequester {
    connections: Arc<RwLock<Vec<Connection>>>,
}
impl QuotesRequester {
    pub fn new(connections: Arc<RwLock<Vec<Connection>>>) -> Self {
        Self {
            connections,
        }
    }
}
impl ProcessedDataSubscriber for QuotesRequester {
    fn on_data(&mut self, id: &str, positions: Vec<Position>) {
        let mut connections = self.connections.write().unwrap();
        let connection = if let Some(connection) = connections.iter_mut().find (|connection| connection.credentials.id == id) {
            let mut tickers = connection.query_tickers.clone();
            let mut new_tickers = Vec::new();
            for position_update in positions.iter().filter(|position_update| position_update.position_id != 0) {
                if position_update.quantity == 0 {
                    let ticker_to_remove = position_update.ticker.to_string();
                    tickers.retain(|ticker| ticker != &ticker_to_remove);
                } else {
                    if !tickers.contains(&position_update.ticker.to_string()) {
                        tickers.push(position_update.ticker.to_string());
                        new_tickers.push(position_update.ticker.to_string());
                    }
                }
            }
            if !new_tickers.is_empty() {
                connection.query_tickers = tickers.clone();
                let quotes_request = Request::quotes(tickers.clone());
                let quotes_request_message = quotes_request.message();
                let order_book_request = Request::order_book(tickers.clone());
                let order_book_request_message = order_book_request.message();
                let sender = connection.channels.sender_to_connector.clone();
                tokio::spawn(async move {
                    if let Err(e) = sender.send(quotes_request_message) {
                        eprintln!("Failed to send quotes request message: {}", e);
                    }
                    if let Err(e) = sender.send(order_book_request_message) {
                        eprintln!("Failed to send order book request message: {}", e);
                    }
                });
            }
        };
    }
}