use std::collections::HashMap;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::SinkExt;

type WsStream = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;
type WsSink = futures_util::stream::SplitSink<WsStream, Message>;

pub struct DaemonClient {
    pub ws_url: String,
    pub token: String,
    sender: Option<WsSink>,
    next_id: u64,
    pending_calls: HashMap<u64, oneshot::Sender<Value>>,
    event_tx: mpsc::UnboundedSender<Value>,
}

impl DaemonClient {
    pub async fn connect(
        host: String,
        port: u16,
        token: String,
    ) -> Result<(Self, mpsc::UnboundedReceiver<Value>), Box<dyn std::error::Error + Send + Sync>> {
        let ws_url = format!("ws://{}:{}/ws", host, port);
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        match connect_async(&ws_url).await {
            Ok((stream, _)) => {
                let (sink, mut source) = stream.split();
                let tx_clone = event_tx.clone();

                tokio::spawn(async move {
                    while let Some(msg) = source.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(val) = serde_json::from_str::<Value>(&text) {
                                    let _ = tx_clone.send(val);
                                }
                            }
                            Ok(Message::Close(_)) => break,
                            Err(_) => break,
                            _ => {}
                        }
                    }
                });

                Ok((
                    DaemonClient {
                        ws_url,
                        token,
                        sender: Some(sink),
                        next_id: 1,
                        pending_calls: HashMap::new(),
                        event_tx,
                    },
                    event_rx,
                ))
            }
            Err(e) => {
                tracing::warn!("Daemon connection failed: {e}. Running in offline mode.");
                let (_, event_rx) = mpsc::unbounded_channel();
                Ok((
                    DaemonClient {
                        ws_url,
                        token,
                        sender: None,
                        next_id: 1,
                        pending_calls: HashMap::new(),
                        event_tx,
                    },
                    event_rx,
                ))
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.sender.is_some()
    }

    pub async fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let sender = match &mut self.sender {
            Some(s) => s,
            None => return Err("Not connected".into()),
        };

        let id = self.next_id;
        self.next_id += 1;

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let msg = Message::Text(request.to_string().into());
        sender.send(msg).await.map_err(|e| e.to_string())?;

        let (tx, rx) = oneshot::channel();
        self.pending_calls.insert(id, tx);

        tokio::time::timeout(std::time::Duration::from_secs(10), rx)
            .await
            .map_err(|_| "Timeout".to_string())?
            .map_err(|_| "Sender dropped".to_string())
    }

    pub async fn call_or_stub(&mut self, method: &str, params: Value) -> Value {
        match self.call(method, params).await {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!("call_or_stub({method}) failed: {e}");
                Value::Null
            }
        }
    }

    pub fn route_message(&mut self, msg: Value) {
        if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
            if let Some(tx) = self.pending_calls.remove(&id) {
                let result = msg.get("result").cloned().unwrap_or(Value::Null);
                let _ = tx.send(result);
                return;
            }
        }
        if msg.get("method").and_then(|v| v.as_str()) == Some("event") {
            let _ = self.event_tx.send(msg);
        }
    }
}
