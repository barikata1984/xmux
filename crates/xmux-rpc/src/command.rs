use std::sync::{Arc, Mutex};
use serde_json::Value;

/// Wraps a oneshot sender that can be cloned (for iced Message derive Clone).
/// Call respond() exactly once to send the result back to the RPC client.
#[derive(Debug, Clone)]
pub struct RpcResponder(Arc<Mutex<Option<tokio::sync::oneshot::Sender<Value>>>>);

impl RpcResponder {
    pub fn new() -> (Self, tokio::sync::oneshot::Receiver<Value>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (Self(Arc::new(Mutex::new(Some(tx)))), rx)
    }

    pub fn respond(self, value: Value) {
        if let Some(tx) = self.0.lock().unwrap().take() {
            let _ = tx.send(value);
        }
    }
}

impl Default for RpcResponder {
    fn default() -> Self {
        let (tx, _) = tokio::sync::oneshot::channel();
        Self(Arc::new(Mutex::new(Some(tx))))
    }
}

#[derive(Debug, Clone)]
pub struct RpcCommand {
    pub id: Option<Value>,
    pub method: String,
    pub params: Value,
    pub responder: RpcResponder,
}
