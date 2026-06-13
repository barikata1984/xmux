use serde_json::{json, Value};
use crate::protocol::RpcError;

pub fn handle_system_method(method: &str, _params: &Value) -> Result<Value, RpcError> {
    match method {
        "system.ping" => Ok(json!("pong")),
        "system.version" => Ok(json!({
            "name": "xmux",
            "version": env!("CARGO_PKG_VERSION")
        })),
        _ => Err(RpcError::method_not_found(method)),
    }
}

/// Check if a method is a system method that can be handled without app state.
pub fn is_system_method(method: &str) -> bool {
    matches!(method, "system.ping" | "system.version")
}
