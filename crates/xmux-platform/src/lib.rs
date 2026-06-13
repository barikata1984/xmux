use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitStatus;

use xmux_core::XmuxError;

#[cfg(target_os = "linux")]
mod linux;

#[derive(Debug, Clone)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

pub struct PtyConfig {
    pub shell: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<PathBuf>,
    pub size: PtySize,
}

pub struct PtyHandle {
    pub reader: Box<dyn Read + Send>,
    pub writer: Box<dyn Write + Send>,
    pub child: Box<dyn PtyChild + Send>,
    pub fd: Option<i32>,
}

pub trait PtyChild: Send {
    fn try_wait(&mut self) -> Result<Option<ExitStatus>, XmuxError>;
    fn kill(&mut self) -> Result<(), XmuxError>;
    fn pid(&self) -> u32;
}

pub trait PlatformPty: Send + Sync {
    fn spawn(&self, config: &PtyConfig) -> Result<PtyHandle, XmuxError>;
    fn resize(&self, handle: &PtyHandle, size: PtySize) -> Result<(), XmuxError>;
}

pub trait PlatformNotifier: Send + Sync {
    fn send_notification(&self, title: &str, body: &str) -> Result<(), XmuxError>;
    fn supports_actions(&self) -> bool;
}

pub trait PlatformClipboard: Send + Sync {
    fn get_text(&self) -> Result<String, XmuxError>;
    fn set_text(&self, text: &str) -> Result<(), XmuxError>;
}

pub trait PlatformShell: Send + Sync {
    fn default_shell(&self) -> PathBuf;
    fn shell_env(&self) -> HashMap<String, String>;
    fn config_dir(&self) -> PathBuf;
    fn data_dir(&self) -> PathBuf;
    fn socket_path(&self) -> PathBuf;
}

pub struct Platform {
    pub pty: Box<dyn PlatformPty>,
    pub notifier: Box<dyn PlatformNotifier>,
    pub clipboard: Box<dyn PlatformClipboard>,
    pub shell: Box<dyn PlatformShell>,
}

pub fn create_platform() -> Platform {
    #[cfg(target_os = "linux")]
    {
        linux::create_linux_platform()
    }
    #[cfg(not(target_os = "linux"))]
    {
        unimplemented!("only Linux is supported in P0–P6")
    }
}
