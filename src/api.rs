use crate::traits::{Publisher, Subscriber};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::StreamExt;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use serde::{Deserialize, Serialize};
use crate::crypto_utils::Credentials;

#[derive(Clone)]
pub struct ConnectionChannels {
    pub main_tx: Arc<Mutex<UnboundedSender<String>>>,
    pub conn_rx: Arc<Mutex<UnboundedReceiver<String>>>,
    pub conn_tx: Arc<Mutex<UnboundedSender<String>>>,
    pub main_rx: Arc<Mutex<UnboundedReceiver<String>>>,
}

#[derive(Clone)]
pub struct Connection {
    pub credentials: Credentials,
    pub channels: ConnectionChannels,
}

impl Connection {
    pub fn new(credentials: Credentials) -> Self {
        let (main_tx, conn_rx) = mpsc::unbounded_channel();
        let (conn_tx, main_rx) = mpsc::unbounded_channel();
        Connection {
            credentials,
            channels: ConnectionChannels {
                main_tx: Arc::new(Mutex::new(main_tx)), 
                conn_rx: Arc::new(Mutex::new(conn_rx)),
                conn_tx: Arc::new(Mutex::new(conn_tx)), 
                main_rx: Arc::new(Mutex::new(main_rx)),
            },
        }
    }
}

pub async fn connect_to_ff_ws(
    credentials: Credentials,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
    sender: Arc<Mutex<mpsc::UnboundedSender<String>>>,
    ) -> Result<(), Box<dyn Error>> {
    println!("WebSocket connection established! {}", &credentials.id);

    let mut receiver = receiver.lock().await;
    let sender = sender.clone();
        
    tokio::spawn(async move {
        let sender = sender.lock().await;
        loop {
            sender
                .send(format!("{} I'm ok...", credentials.id))
                .expect("Failed to send message");
            time::sleep(Duration::from_secs(2)).await;
        }        
    });
    
    while let Some(message) = receiver.recv().await {
        println!("Received from main: {}", message);        
    }
    Ok(())
}