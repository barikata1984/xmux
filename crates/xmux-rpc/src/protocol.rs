use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: &'static str,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

// For deserialization, use a separate struct with owned String
#[derive(Debug, Deserialize)]
pub struct RpcResponseOwned {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<RpcError>,
}

impl RpcResponseOwned {
    pub fn to_response(self) -> RpcResponse {
        RpcResponse {
            jsonrpc: "2.0",
            id: self.id,
            result: self.result,
            error: self.error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError { code, message }),
        }
    }
}

impl RpcError {
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".into(),
        }
    }

    pub fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "Invalid Request".into(),
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {method}"),
        }
    }

    pub fn invalid_params(msg: &str) -> Self {
        Self {
            code: -32602,
            message: format!("Invalid params: {msg}"),
        }
    }

    pub fn internal_error(msg: &str) -> Self {
        Self {
            code: -32603,
            message: format!("Internal error: {msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_request_parse() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test_method","params":{"key":"value"}}"#;
        let req: RpcRequest = serde_json::from_str(json).expect("Failed to parse request");
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "test_method");
        assert_eq!(req.id, Some(serde_json::Value::Number(1.into())));
    }

    #[test]
    fn test_rpc_response_serialize() {
        let resp = RpcResponse::success(
            Some(serde_json::Value::Number(1.into())),
            serde_json::json!({"status": "ok"}),
        );
        let json = serde_json::to_string(&resp).expect("Failed to serialize response");
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""result":{"status":"ok"}"#));
        assert!(!json.contains("error"));

        let err_resp = RpcResponse::error(
            Some(serde_json::Value::Number(1.into())),
            -32700,
            "Parse error".into(),
        );
        let err_json = serde_json::to_string(&err_resp).expect("Failed to serialize error");
        assert!(!err_json.contains("result"));
        assert!(err_json.contains(r#""code":-32700"#));
    }
}
