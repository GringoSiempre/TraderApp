use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::crypto_utils::Credentials;

pub const HTTPS_API_FF_URL: &str = "https://tradernet.com/api/check-login-password";
pub const WS_API_FF_URL: &str = "wss://wss.tradernet.com/";
pub const FF_GET_SID: &str = "/api/check-login-password";
pub const FF_SID: &str = "?SID=";

#[derive(Serialize, Deserialize)]
pub struct AuthMessage {
    login: String,
    password: String,
    rememberMe: i32,
}
impl AuthMessage {
    pub(crate) fn new(login: &str, password: &str) -> Self {
        AuthMessage {
            login: login.to_owned(),
            password: password.to_owned(),
            rememberMe: 1,
        }
    }
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

// WS requests
pub struct Request {
    pub cmd: &'static str,
    pub params: Option<Vec<String>>,
}
impl Request {
    pub fn order_book(params: Vec<String>) -> Self {
        Self { cmd: "orderBook", params: Some(params), }
    }
    pub fn quotes(params: Vec<String>) -> Self {
        Self { cmd: "quotes", params: Some(params), }
    }
    pub fn portfolio() -> Self {
        Self { cmd: "portfolio", params: None, }
    }
    pub fn orders() -> Self {
        Self { cmd: "orders", params: None, }
    }
    pub fn markets() -> Self {
        Self { cmd: "markets", params: None, }
    }
    pub fn message(&self) -> String {
        match &self.params {
            Some(params) => serde_json::to_string(&json!([self.cmd,params])).expect("Failed to serialize json"),
            None => serde_json::to_string(&json!([self.cmd])).expect("Failed to serialize json"),
        }
    }
}

// Parameters for sending orders
#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    Buy,
    Sell,
}
impl ActionType {
    pub fn ff_code(&self) -> u32 {
        match self {
            ActionType::Buy => 1,
            ActionType::Sell => 3,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
}
impl OrderType {
    pub fn ff_code(&self) -> u32 {
        match self {
            OrderType::Market => 1,
            OrderType::Limit => 2,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Expirations {
    Day,
}
impl Expirations {
    pub fn ff_code(&self) -> u32 {
        match self {
            Expirations::Day => 1,
        }
    }
}