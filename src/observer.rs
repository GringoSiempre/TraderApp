use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::task;
use tokio::sync::mpsc;
use crate::market_data::{deserialize_message, MarketData};

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
pub struct DataProcessorSubscriber {
    data_sender: mpsc::Sender<String>,
    subscribers: Arc<Mutex<Vec<Box<dyn MarketDataUpdateSubscriber>>>>,
}
impl DataProcessorSubscriber {
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
impl MessageSubscriber for DataProcessorSubscriber {
    fn on_data(&mut self, id: &str, timestamp: chrono::DateTime<chrono::Local>, data: &str) {
        let x = deserialize_message(data);
        match deserialize_message(data) {
            Some(deserialized_data) => {
                let message = serde_json::to_string_pretty(&deserialized_data).unwrap(); 
                let _ = self.data_sender.try_send(message);
                self.notify_subscribers(id, deserialized_data);
            },
            None => {
                let _ = self.data_sender.try_send("Unable to deserialize message".to_string());
            },
        }
    }
}