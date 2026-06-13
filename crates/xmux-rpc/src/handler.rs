use serde_json::{json, Value};
use crate::protocol::RpcError;

pub fn handle_system_method(method: &str, _params: &Value) -> Result<Value, RpcError> {
    match method {
        "system.ping" => Ok(json!("pong")),
        "system.version" => Ok(json!({
            "name": "xmux",
            "version": env!("CARGO_PKG_VERSION")
        })),
        "browser.list" => Ok(json!([])),
        "browser.open" => Ok(json!({
            "status": "not_implemented",
            "message": "Browser backend not yet connected"
        })),
        "browser.close" => Ok(json!({
            "status": "not_implemented"
        })),
        "browser.navigate" => Ok(json!({
            "status": "not_implemented"
        })),
        "browser.eval" => Ok(json!({
            "status": "not_implemented"
        })),
        _ => Err(RpcError::method_not_found(method)),
    }
}

/// Check if a method is a system method that can be handled without app state.
pub fn is_system_method(method: &str) -> bool {
    matches!(method, "system.ping" | "system.version" | "browser.list" | "browser.open" | "browser.close" | "browser.navigate" | "browser.eval")
}
