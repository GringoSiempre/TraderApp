use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
use std::error::Error;
use reqwest::Client;
use crate::crypto_utils::Credentials;
use crate::api_utils::*;

pub struct ConnectionChannels {
    pub sender_to_connector: mpsc::UnboundedSender<String>,
    pub sender_to_ui: mpsc::UnboundedSender<String>,
}

pub struct Connection {
    pub credentials: Credentials,
    pub channels: ConnectionChannels,
    pub query_tickers: Vec<String>,
}

impl Connection {
    pub fn new(credentials: Credentials, sender_to_connector: UnboundedSender<String>, sender_to_ui: UnboundedSender<String>) -> Self {
        Connection {
            credentials,
            channels: ConnectionChannels {
                sender_to_connector,
                sender_to_ui,
            },
            query_tickers: vec![
                "QQQ.US".to_string(),
                "SPY.US".to_string(),
            ],
        }
    }
}

pub async fn connect_to_ff_ws(
    credentials: Credentials,
    mut receiver: UnboundedReceiver<String>,
    sender: UnboundedSender<String>,
    ) -> Result<(), Box<dyn Error>> {
    let sid = get_sid_ff(credentials.clone()).await;
    let ws_url = WS_API_FF_URL.to_string() + FF_SID + &sid;
    let url = ws_url.as_str();
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect.");
    let (mut write, mut read) = ws_stream.split();
    let mut write = Arc::new(Mutex::new(write));

    // Receive messages from Freedom
    let id = credentials.id.clone();
    let write_clone = write.clone();
    let sender_clone = sender.clone();
    tokio::spawn(async move {
        while let Some(message) = read.next().await {
            let mut write_clone = write_clone.lock().await;
            match message {
                Ok(Message::Text(text)) => {
                    sender_clone.send(text).expect("Failed to send message");
                },
                Ok(Message::Binary(bin)) => println!("{} Binary data received: {:?}\n", id, bin),
                Ok(Message::Frame(frame)) => println!("{} Text message received: {}\n", id, frame),
                Ok(Message::Ping(ping)) => write_clone.send(tokio_tungstenite::tungstenite::protocol::Message::Pong(ping)).await.expect("Ping Error"),
                Ok(Message::Pong(_)) => println!("{} Ping answer received.\n", id),
                Ok(Message::Close(_)) => { println!("{} Connection closed.\n", id); break; }
                Err(err) => { println!("{} Error at {}: {}", id, chrono::Local::now(), err); break; },
            }
        }        
    });
    // Receive messages from GUI
    let id = credentials.id.clone();
    let mut write_clone = write.clone();
    tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            println!("{}", message);
            let mut write_clone = write_clone.lock().await;
            write_clone.send(Message::Text(message)).await.expect("Failed to send message");
        }
    });
    Ok(())
}