use crate::traits::{Publisher, Subscriber};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::StreamExt;
use std::error::Error;

pub struct ConnectionChannels {
    send_channel: (mpsc::UnboundedSender<String>, mpsc::UnboundedReceiver<String>),
    receive_channel: (mpsc::UnboundedSender<String>, mpsc::UnboundedReceiver<String>),
}

pub struct Connection {
    id: String,
    channels: ConnectionChannels,
}

impl Connection {
    pub fn new(id: String) -> Self {
        let send_channel = mpsc::unbounded_channel();
        let receive_channel = mpsc::unbounded_channel();
        Connection {
            id,
            channels: ConnectionChannels {
                send_channel,
                receive_channel,
            },
        }
    }
}
// Пример организации канала: let connection = Connection::new("ff1");
// Доступ к каналам через именованные поля
// let (send_tx, send_rx) = &connection.channels.send_channel;
// let (recv_tx, recv_rx) = &connection.channels.receive_channel;
// Теперь вы можете использовать эти каналы для отправки и получения данных
// send_tx.send("Message".to_string()).await.unwrap();

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