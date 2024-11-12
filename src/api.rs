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
    let sid = get_sid_ff(credentials.clone()).await;
    sender.lock().await.send("Trying to connect...".to_string())?;
    let ws_url = WS_API_FF_URL.to_string() + FF_SID + &sid;
    let url = ws_url.as_str();
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect.");
    sender.lock().await.send("Connected.".to_string())?;
    let (mut write, mut read) = ws_stream.split();
    let write = Arc::new(Mutex::new(write));

    // Receive messages from Freedom
    let id = credentials.id.clone();
    let write_clone = write.clone();
    let sender_clone = sender.clone();
    tokio::spawn(async move {
        let mut write = write_clone.lock().await;
        let sender = sender_clone.lock().await;
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    sender.send(text).expect("Failed to send message");
                },
                Ok(Message::Binary(bin)) => println!("{} Binary data received: {:?}\n", id, bin),
                Ok(Message::Frame(frame)) => println!("{} Text message received: {}\n", id, frame),
                Ok(Message::Ping(ping)) => write.send(tokio_tungstenite::tungstenite::protocol::Message::Pong(ping)).await.expect("Ping Error"),
                Ok(Message::Pong(_)) => println!("{} Ping answer received.\n", id),
                Ok(Message::Close(_)) => { println!("{} Connection closed.\n", id); break; }
                Err(err) => { println!("{} Error at {}: {}", id, chrono::Local::now(), err); break; },
            }
        }        
    });
    // Receive messages from GUI
    let id = credentials.id.clone();
    let mut receiver_clone = receiver.lock().await;
    let write_clone = write.clone();
    while let Some(message) = receiver_clone.recv().await {
        let mut write = write_clone.lock().await;
        write.send(Message::Text(message)).await.expect("Failed to send message");
    }
    Ok(())
}

