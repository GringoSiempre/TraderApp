use crate::traits::{Publisher, Subscriber};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
// use futures_util::stream::{SplitSink, SplitStream};
// futures::stream::StreamExt;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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
    let ws_url = WS_API_FF_URL.to_string() + FF_SID + &sid;
    let url = ws_url.as_str();
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect.");
    let (mut write, mut read) = ws_stream.split();
    let write = Arc::new(Mutex::new(write));

    // Receive messages from Freedom
    let id = credentials.id.clone();
    let write_clone = write.clone();
    tokio::spawn(async move {
        while let Some(message) = read.next().await {
            let mut write = write_clone.lock().await;
            match message {
                Ok(Message::Text(text)) => {
                    println!("{} Received from Freedom WS at {}: {}", id, chrono::Local::now(), text);
                },
                Ok(Message::Binary(bin)) => println!("{} Binary data received: {:?}\n", id, bin),
                Ok(Message::Frame(frame)) => println!("{} Text message received: {}\n", id, frame),
                Ok(Message::Ping(ping)) => write.send(tokio_tungstenite::tungstenite::protocol::Message::Pong(ping)).await.expect("Ping Error"),
                Ok(Message::Pong(_)) => println!("{} Ping answer received.\n", id),
                Ok(Message::Close(_)) => { println!("{} Connection closed.\n", id); break; }
                Err(err) => println!("{} Error at {}: {}", id, chrono::Local::now(), err),
            }
        }        
    });
    // Send messages to GUI 
    let id = credentials.id.clone();
    let sender = sender.clone();
    tokio::spawn(async move {
        let sender = sender.lock().await;
        loop {
            time::sleep(Duration::from_secs(600)).await;
            sender
                .send(format!("{} on air...", id))
                .expect("Failed to send message");
        }        
    });
    // Receive messages from GUI
    let id = credentials.id.clone();
    let mut receiver = receiver.lock().await;
    while let Some(message) = receiver.recv().await {
        println!("{} Received from main: {}", id, message);        
    }
    
    Ok(())
}

// Receive Security ID (sid) from Freedom24
pub async fn get_sid_ff(credentials: Credentials) -> String {
    // Creating the request
    let client = Client::new();
    let auth_message = AuthMessage::new (credentials.login.as_str(), credentials.password.as_str());
    let mut headers = reqwest::header::HeaderMap::new();
    let content_type = reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded");
    headers.insert(reqwest::header::CONTENT_TYPE, content_type);
    let urlencoded_message = serde_urlencoded::to_string(&auth_message).unwrap();
    // Sending POST request
    let response = client.post(HTTPS_API_FF_URL).headers(headers).body(urlencoded_message).send().await.expect("Error sending POST request");
    let response_text = response.text().await.unwrap();
    let parsed_response_text: serde_json::Value = serde_json::from_str(&response_text).expect("Failed to parse json");
    parsed_response_text["SID"].as_str().unwrap().to_string()
}