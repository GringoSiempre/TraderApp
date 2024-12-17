use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
use std::error::Error;
use reqwest::Client;
use reqwest::header::HeaderValue;
use serde_json::json;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC, AsciiSet, CONTROLS};
use ring::hmac::{Key, HMAC_SHA256, sign};
use once_cell::sync::Lazy;
use crate::crypto_utils::Credentials;
use crate::api_utils::*;

pub static BASE_TICKERS: Lazy<Vec<String>> = Lazy::new(|| {
    vec!["QQQ.US".to_string(), "SPY.US".to_string()]
});

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

impl ConnectionStatus {
    pub fn description(&self) -> &str {
        match self {
            ConnectionStatus::Connected => "online",
            ConnectionStatus::Disconnected => "offline",
        }
    }
}

pub struct ConnectionChannels {
    pub sender_to_connector: mpsc::UnboundedSender<String>,
    pub sender_to_ui: mpsc::UnboundedSender<String>,
}

pub struct Connection {
    pub credentials: Credentials,
    pub channels: ConnectionChannels,
    pub query_tickers: Vec<String>,
    pub status: ConnectionStatus,
}

impl Connection {
    pub fn new(credentials: Credentials, sender_to_connector: UnboundedSender<String>, sender_to_ui: UnboundedSender<String>) -> Self {
        Connection {
            credentials,
            channels: ConnectionChannels {
                sender_to_connector,
                sender_to_ui,
            },
            query_tickers: BASE_TICKERS.clone(),
            status: ConnectionStatus::Disconnected,
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
            let mut write_clone = write_clone.lock().await;
            write_clone.send(Message::Text(message)).await.expect("Failed to send message");
        }
    });
    Ok(())
}

const FRAGMENT: &AsciiSet = &CONTROLS.add(b'+');
pub async fn send_order (public_key: String, secret_key: String, ticker: String, action: ActionType, order: OrderType, price: f64, qty: u64, expiration: Expirations) -> Result<(), ()> {
    let client = Client::new();
    let url = "https://tradernet.com/api/v2/cmd/putTradeOrder";
    let current_time = chrono::Utc::now().timestamp_millis().to_string();
    let nonce = current_time.as_str();

    let cmd = "putTradeOrder";
    let mut params = json!({
        "instr_name": ticker,
        "action_id": action.ff_code(),
        "order_type_id": order.ff_code(),
        "qty": qty,
        "expiration_id": expiration.ff_code()
    });
    if price != 0.0 {
        params["limit_price"] = json!(price);
    }

    let string_header = format!("apiKey={}&cmd={}&nonce={}", public_key.as_str(), cmd, nonce); /* Проверить необходимость as_str() */
    let mut params_pairs_for_sign = Vec::new();
    let mut params_pairs_for_request = Vec::new();
    if let serde_json::Value::Object(map) = params {
        for(key, value) in map.iter() {
            let mut value_string = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => value.to_string(),
            };
            params_pairs_for_sign.push(format!("{}={}", key, value_string));
            let value_string = utf8_percent_encode(
                &value_string,
                FRAGMENT
            ).to_string();
            params_pairs_for_request.push(format!("params[{}]={}", key, value_string));
        }
    }
    let params_string_for_sign = params_pairs_for_sign.join("&");
    let params_string_for_request = params_pairs_for_request.join("&");

    let string_for_sign = format!("{}&params={}",string_header, params_string_for_sign);
    let string_for_request = format!("{}&{}",string_header, params_string_for_request);
    // println!("For sign: {}\nFor request: {}",string_for_sign,string_for_request);

    let key = Key::new(HMAC_SHA256,secret_key.as_str().as_bytes()); /* Проверить необходимость as_str() */
    let hmac_sha256_signature = sign(&key, string_for_sign.as_bytes());
    let signature = hex::encode(hmac_sha256_signature.as_ref());

    let mut headers = reqwest::header::HeaderMap::new();
    let content_type = reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded");
    headers.insert(reqwest::header::CONTENT_TYPE, content_type);
    let signature_value = HeaderValue::from_str(signature.as_str()).expect("Failed to create signature header value");
    headers.insert("X-NtApi-Sig",signature_value);

    println!("Order sending {}", chrono::Local::now());
    let response = client
        .post(url)
        .headers(headers)
        .body(string_for_request)
        .send()
        .await
        .expect("Error sending POST request");
    println!("Response received {}", chrono::Local::now());
    // println!("Status: {:?}", response.status());
    // println!("Headers:\n{:#?}", response.headers());
    // let response_text = response.text().await.unwrap();
    // println!("Text {}", response_text);

    Ok(())
}