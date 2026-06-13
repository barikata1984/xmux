use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Parser)]
#[command(name = "xmux", about = "Terminal multiplexer")]
pub struct Cli {
    #[arg(long, help = "Socket path")]
    pub socket: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Ping,
    ListWorkspaces {
        #[arg(long)]
        json: bool,
    },
    NewWorkspace {
        #[arg(long)]
        name: Option<String>,
    },
    SelectWorkspace {
        #[arg(long)]
        index: usize,
    },
    CurrentWorkspace,
    ListSurfaces,
    NewSplit {
        direction: String,  // left, right, up, down
    },
    Send {
        text: String,
    },
    Notify {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: String,
    },
    ListNotifications,
    ClearNotifications,
    BrowserOpen {
        url: String,
        #[arg(long, default_value = "right")]
        split: String,
    },
    BrowserList,
    BrowserNavigate {
        url: String,
    },
    BrowserEval {
        script: String,
    },
    BrowserClose,
    TmuxCompat {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}

fn socket_path() -> PathBuf {
    if let Ok(path) = std::env::var("XMUX_SOCKET_PATH") {
        PathBuf::from(path)
    } else {
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/xmux-{uid}.sock"))
    }
}

async fn send_rpc(socket: &PathBuf, method: &str, params: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let stream = UnixStream::connect(socket).await?;
    let (reader, mut writer) = stream.into_split();

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    let mut line = serde_json::to_string(&request)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;

    let mut buf_reader = BufReader::new(reader);
    let mut response_line = String::new();
    buf_reader.read_line(&mut response_line).await?;

    let response: serde_json::Value = serde_json::from_str(&response_line)?;
    if let Some(error) = response.get("error") {
        Err(format!("RPC error: {}", error).into())
    } else {
        Ok(response.get("result").cloned().unwrap_or(serde_json::Value::Null))
    }
}

pub async fn run_cli(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let socket = cli.socket.unwrap_or_else(socket_path);

    match cli.command.unwrap() {
        Commands::Ping => {
            let result = send_rpc(&socket, "system.ping", serde_json::json!({})).await?;
            println!("{}", result);
        }
        Commands::ListWorkspaces { json } => {
            let result = send_rpc(&socket, "workspace.list", serde_json::json!({})).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                if let Some(arr) = result.as_array() {
                    for ws in arr {
                        let active = if ws["active"].as_bool().unwrap_or(false) { " *" } else { "" };
                        println!("{}: {}{}", ws["index"], ws["name"].as_str().unwrap_or("?"), active);
                    }
                }
            }
        }
        Commands::NewWorkspace { name: _ } => {
            let result = send_rpc(&socket, "workspace.create", serde_json::json!({})).await?;
            println!("{}", result);
        }
        Commands::SelectWorkspace { index } => {
            let result = send_rpc(&socket, "workspace.select", serde_json::json!({"index": index})).await?;
            println!("{}", result);
        }
        Commands::CurrentWorkspace => {
            let result = send_rpc(&socket, "workspace.current", serde_json::json!({})).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::ListSurfaces => {
            let result = send_rpc(&socket, "surface.list", serde_json::json!({})).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::NewSplit { direction } => {
            let result = send_rpc(&socket, "surface.split", serde_json::json!({"direction": direction})).await?;
            println!("{}", result);
        }
        Commands::Send { text } => {
            let result = send_rpc(&socket, "surface.send_text", serde_json::json!({"text": text})).await?;
            println!("{}", result);
        }
        Commands::Notify { title, body } => {
            let result = send_rpc(&socket, "notification.create", serde_json::json!({"title": title, "body": body})).await?;
            println!("{}", result);
        }
        Commands::ListNotifications => {
            let result = send_rpc(&socket, "notification.list", serde_json::json!({})).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::ClearNotifications => {
            let result = send_rpc(&socket, "notification.clear", serde_json::json!({})).await?;
            println!("{}", result);
        }
        Commands::BrowserOpen { url, split } => {
            let result = send_rpc(&socket, "browser.open", serde_json::json!({"url": url, "split": split})).await?;
            println!("{}", result);
        }
        Commands::BrowserList => {
            let result = send_rpc(&socket, "browser.list", serde_json::json!({})).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::BrowserNavigate { url } => {
            let result = send_rpc(&socket, "browser.navigate", serde_json::json!({"url": url})).await?;
            println!("{}", result);
        }
        Commands::BrowserEval { script } => {
            let result = send_rpc(&socket, "browser.eval", serde_json::json!({"script": script})).await?;
            println!("{}", result);
        }
        Commands::BrowserClose => {
            let result = send_rpc(&socket, "browser.close", serde_json::json!({})).await?;
            println!("{}", result);
        }
        Commands::TmuxCompat { args } => {
            let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            if let Some((method, params)) = crate::tmux_shim::parse_tmux_command(str_args.as_slice()) {
                let result = send_rpc(&socket, method, params).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                eprintln!("Unknown tmux command");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}
