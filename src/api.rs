use crate::traits::{Publisher, Subscriber};
use std::sync::{Arc, Mutex};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::StreamExt;
use std::error::Error;

pub async fn connect_to_ff_ws(url: &str) -> Result<(), Box<dyn Error>> {
    let (ws_stream, _) = connect_async(url).await?;
    println!("WebSocket connection established!");
    
    let(_, mut read) = ws_stream.split();
    while let Some(message) = read.next().await {
        if let Ok(msg) = message {
            if msg.is_text() {
                let text = msg.into_text().unwrap();
            }
        }
    }
    Ok(())
}