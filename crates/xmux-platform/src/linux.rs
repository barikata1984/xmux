use std::collections::HashMap;
use std::path::PathBuf;

use xmux_core::XmuxError;

use crate::{
    Platform, PlatformClipboard, PlatformNotifier, PlatformPty, PlatformShell, PtyConfig,
    PtyHandle, PtySize,
};

pub struct LinuxPty;
pub struct LinuxNotifier;
pub struct LinuxClipboard;
pub struct LinuxShell;

impl PlatformPty for LinuxPty {
    fn spawn(&self, _config: &PtyConfig) -> Result<PtyHandle, XmuxError> {
        // P0 では alacritty_terminal::tty + EventLoop を直接使用するためスタブ
        Err(XmuxError::Pty(
            "LinuxPty::spawn is a stub; use alacritty_terminal::tty directly".into(),
        ))
    }

    fn resize(&self, _handle: &PtyHandle, _size: PtySize) -> Result<(), XmuxError> {
        Err(XmuxError::Pty("LinuxPty::resize is a stub".into()))
    }
}

impl PlatformNotifier for LinuxNotifier {
    fn send_notification(&self, _title: &str, _body: &str) -> Result<(), XmuxError> {
        Err(XmuxError::Pty("LinuxNotifier is a stub".into()))
    }

    fn supports_actions(&self) -> bool {
        true
    }
}

impl PlatformClipboard for LinuxClipboard {
    fn get_text(&self) -> Result<String, XmuxError> {
        Err(XmuxError::Pty("LinuxClipboard is a stub".into()))
    }

    fn set_text(&self, _text: &str) -> Result<(), XmuxError> {
        Err(XmuxError::Pty("LinuxClipboard is a stub".into()))
    }
}

impl PlatformShell for LinuxShell {
    fn default_shell(&self) -> PathBuf {
        std::env::var("SHELL")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/bin/bash"))
    }

    fn shell_env(&self) -> HashMap<String, String> {
        std::env::vars().collect()
    }

    fn config_dir(&self) -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("xmux")
    }

    fn data_dir(&self) -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("xmux")
    }

    fn socket_path(&self) -> PathBuf {
        std::env::var("XMUX_SOCKET_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/xmux.sock"))
    }
}

pub fn create_linux_platform() -> Platform {
    Platform {
        pty: Box::new(LinuxPty),
        notifier: Box::new(LinuxNotifier),
        clipboard: Box::new(LinuxClipboard),
        shell: Box::new(LinuxShell),
    }
}
