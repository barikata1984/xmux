use thiserror::Error;

#[derive(Debug, Error)]
pub enum XmuxError {
    #[error("PTY error: {0}")]
    Pty(String),

    #[error("terminal error: {0}")]
    Terminal(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("session error: {0}")]
    Session(String),
}
