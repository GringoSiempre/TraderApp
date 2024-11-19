use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex, RwLock};
use tokio::task;
use tokio::sync::mpsc;
use crate::market_data::{deserialize_message, MarketData};
// use crate::processed_data::{OrderBook, ProcessedData};
use crate::processed_data::*;

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
    portfolios: Arc<RwLock<Vec<Portfolio>>>,
    subscribers: Arc<Mutex<Vec<Box<dyn ProcessedDataSubscriber>>>>,
}
impl DataProcessor {
    pub fn new(
        data_sender: mpsc::Sender<String>, 
        order_books: Arc<RwLock<Vec<OrderBook>>>, 
        quotes: Arc<RwLock<Vec<QuoteBook>>>, 
        portfolios: Arc<RwLock<Vec<Portfolio>>>
    ) -> Self {
        Self {
            data_sender,
            order_books,
            quotes,
            portfolios,
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn subscribe(&mut self, subscriber: Box<dyn ProcessedDataSubscriber>) {
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
impl MarketDataUpdateSubscriber for DataProcessor {
    fn on_data(&mut self, id: &str, market_data: &MarketData) {
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
                    order_book.add_row(&order_book_message.i, row);                 
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

                // let message = "On air...".to_string();                
                // let _ = self.data_sender.try_send(message);
            }
            MarketData::PortfolioMessage(portfolio_message) => {
                // let message = format!("{} Для {} получены дата процессором {:?}", chrono::Local::now(), id, portfolio_message);
                let message = "On air...".to_string();
                let _ = self.data_sender.try_send(message);
            }
        }
        self.notify_subscribers(id);
    }
}