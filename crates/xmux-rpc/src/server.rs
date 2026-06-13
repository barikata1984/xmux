use crate::protocol::{RpcRequest, RpcResponse};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

#[cfg(test)]
use crate::protocol::RpcResponseOwned;

pub type RpcHandler = Arc<
    dyn Fn(RpcRequest) -> Pin<Box<dyn Future<Output = RpcResponse> + Send>> + Send + Sync,
>;

pub struct RpcServer {
    socket_path: PathBuf,
}

impl RpcServer {
    pub fn new() -> Self {
        let uid = unsafe { libc::getuid() };
        let socket_path = PathBuf::from(format!("/tmp/xmux-{}.sock", uid));
        Self { socket_path }
    }

    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    pub fn bind(&self) -> std::io::Result<UnixListener> {
        // Remove existing socket if it exists
        let _ = std::fs::remove_file(&self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)?;

        // Set socket permissions to 0600
        #[cfg(unix)]
        {
            use std::fs;
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.socket_path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(listener)
    }

    pub async fn run(&self, handler: RpcHandler) -> std::io::Result<()> {
        let listener = self.bind()?;

        loop {
            let (socket, _) = listener.accept().await?;
            let handler = Arc::clone(&handler);
            tokio::spawn(Self::handle_connection(socket, handler));
        }
    }

    async fn handle_connection(socket: UnixStream, handler: RpcHandler) {
        let (reader, mut writer) = socket.into_split();
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            match buf_reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    let response = match serde_json::from_str::<RpcRequest>(line) {
                        Ok(req) => {
                            // Validate jsonrpc field
                            if req.jsonrpc != "2.0" {
                                RpcResponse::error(req.id, -32600, "Invalid Request".into())
                            } else {
                                handler(req).await
                            }
                        }
                        Err(_) => {
                            RpcResponse::error(None, -32700, "Parse error".into())
                        }
                    };

                    if let Ok(json) = serde_json::to_string(&response) {
                        let _ = writer.write_all(json.as_bytes()).await;
                        let _ = writer.write_all(b"\n").await;
                        let _ = writer.flush().await;
                    }
                }
                Err(_) => break,
            }
        }
    }
}

impl Default for RpcServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path() {
        let server = RpcServer::new();
        let uid = unsafe { libc::getuid() };
        let expected = format!("/tmp/xmux-{}.sock", uid);
        assert_eq!(server.socket_path().to_str().unwrap(), expected);
    }

    #[tokio::test]
    async fn test_simple_rpc_flow() {
        let server = RpcServer::new();

        // Create a simple echo handler
        let handler: RpcHandler = Arc::new(|req: RpcRequest| {
            Box::pin(async move {
                RpcResponse::success(req.id, serde_json::json!({"method": req.method}))
            })
        });

        // Spawn server in background
        let server_socket = server.bind().expect("Failed to bind");
        let socket_path = server.socket_path().clone();

        let server_task = tokio::spawn(async move {
            let (socket, _) = server_socket.accept().await.unwrap();
            RpcServer::handle_connection(socket, handler).await;
        });

        // Give server time to bind
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Connect and send request
        let mut stream = UnixStream::connect(&socket_path)
            .await
            .expect("Failed to connect");

        let request = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
        stream
            .write_all(request.as_bytes())
            .await
            .expect("Failed to write");
        stream.write_all(b"\n").await.expect("Failed to write");
        stream.flush().await.expect("Failed to flush");

        // Read response
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader
            .read_line(&mut response)
            .await
            .expect("Failed to read response");

        let resp: RpcResponseOwned = serde_json::from_str(&response).expect("Failed to parse response");
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());

        server_task.abort();
    }
}
