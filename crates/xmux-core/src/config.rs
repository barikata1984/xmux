use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryColors {
    pub foreground: Rgb,
    pub background: Rgb,
    pub dim_foreground: Rgb,
    pub bright_foreground: Rgb,
}

impl Default for PrimaryColors {
    fn default() -> Self {
        Self {
            foreground: Rgb::new(0xd8, 0xd8, 0xd8),
            background: Rgb::new(0x18, 0x18, 0x18),
            dim_foreground: Rgb::new(0x82, 0x82, 0x82),
            bright_foreground: Rgb::new(0xff, 0xff, 0xff),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnsiColors {
    pub black: Rgb,
    pub red: Rgb,
    pub green: Rgb,
    pub yellow: Rgb,
    pub blue: Rgb,
    pub magenta: Rgb,
    pub cyan: Rgb,
    pub white: Rgb,
}

impl AnsiColors {
    pub fn normal() -> Self {
        Self {
            black: Rgb::new(0x18, 0x18, 0x18),
            red: Rgb::new(0xab, 0x46, 0x42),
            green: Rgb::new(0xa1, 0xb5, 0x6c),
            yellow: Rgb::new(0xf7, 0xca, 0x88),
            blue: Rgb::new(0x7c, 0xaf, 0xc2),
            magenta: Rgb::new(0xba, 0x8b, 0xaf),
            cyan: Rgb::new(0x86, 0xc1, 0xb9),
            white: Rgb::new(0xd8, 0xd8, 0xd8),
        }
    }

    pub fn bright() -> Self {
        Self {
            black: Rgb::new(0x58, 0x58, 0x58),
            red: Rgb::new(0xab, 0x46, 0x42),
            green: Rgb::new(0xa1, 0xb5, 0x6c),
            yellow: Rgb::new(0xf7, 0xca, 0x88),
            blue: Rgb::new(0x7c, 0xaf, 0xc2),
            magenta: Rgb::new(0xba, 0x8b, 0xaf),
            cyan: Rgb::new(0x86, 0xc1, 0xb9),
            white: Rgb::new(0xf8, 0xf8, 0xf8),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorColors {
    pub foreground: Rgb,
    pub background: Rgb,
}

impl Default for CursorColors {
    fn default() -> Self {
        Self {
            foreground: Rgb::new(0x18, 0x18, 0x18),
            background: Rgb::new(0xd8, 0xd8, 0xd8),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionColors {
    pub foreground: Rgb,
    pub background: Rgb,
}

impl Default for SelectionColors {
    fn default() -> Self {
        Self {
            foreground: Rgb::new(0xff, 0xff, 0xff),
            background: Rgb::new(0x44, 0x44, 0x44),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub primary: PrimaryColors,
    pub normal: AnsiColors,
    pub bright: AnsiColors,
    pub cursor: CursorColors,
    pub selection: SelectionColors,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            primary: PrimaryColors::default(),
            normal: AnsiColors::normal(),
            bright: AnsiColors::bright(),
            cursor: CursorColors::default(),
            selection: SelectionColors::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
    pub weight: u16,
    pub bold_is_bright: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: String::from("monospace"),
            size: 14.0,
            weight: 400,
            bold_is_bright: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl Default for ShellConfig {
    fn default() -> Self {
        let shell = std::env::var("SHELL")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/bin/bash"));
        Self {
            program: shell,
            args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyModifier {
    Ctrl,
    Alt,
    Shift,
    Super,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyAction {
    Copy,
    Paste,
    NewPane,
    ClosePane,
    SplitHorizontal,
    SplitVertical,
    NextPane,
    PreviousPane,
    NextWorkspace,
    PreviousWorkspace,
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybinding {
    pub key: String,
    pub modifiers: Vec<KeyModifier>,
    pub action: KeyAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub font: FontConfig,
    pub colors: ColorScheme,
    pub shell: ShellConfig,
    pub keybindings: Vec<Keybinding>,
    pub scrollback_lines: usize,
    pub socket_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font: FontConfig::default(),
            colors: ColorScheme::default(),
            shell: ShellConfig::default(),
            keybindings: Vec::new(),
            scrollback_lines: 100_000,
            socket_path: PathBuf::from("/tmp/xmux.sock"),
        }
    }
}
