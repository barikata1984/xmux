use iced::keyboard::{Key, Modifiers};
use iced::keyboard::key::Named;

/// Calculate the xterm modifier parameter number.
///
/// Encoding: 1 + (shift ? 1 : 0) + (alt ? 2 : 0) + (ctrl ? 4 : 0)
fn modifier_number(m: &Modifiers) -> u8 {
    let mut n = 0u8;
    if m.shift() {
        n |= 1;
    }
    if m.alt() {
        n |= 2;
    }
    if m.control() {
        n |= 4;
    }
    n + 1
}

/// Build a CSI sequence: ESC [ code ; mod suffix
/// When mod_no == 1 (no modifiers), omit the ";mod" part.
fn csi(code: &str, suffix: &str, mod_no: u8) -> Vec<u8> {
    if mod_no > 1 {
        format!("\x1b[{code};{mod_no}{suffix}").into_bytes()
    } else {
        format!("\x1b[{code}{suffix}").into_bytes()
    }
}

/// Build a CSI sequence for arrow/home/end keys: ESC [ 1 ; mod code
/// When mod_no == 1, emit ESC [ code.
fn csi2(code: &str, mod_no: u8) -> Vec<u8> {
    if mod_no > 1 {
        format!("\x1b[1;{mod_no}{code}").into_bytes()
    } else {
        format!("\x1b[{code}").into_bytes()
    }
}

/// Build an SS3 sequence: ESC O code
/// If modifiers are present, fall back to CSI 1;mod code.
fn ss3(code: &str, mod_no: u8) -> Vec<u8> {
    if mod_no > 1 {
        format!("\x1b[1;{mod_no}{code}").into_bytes()
    } else {
        format!("\x1bO{code}").into_bytes()
    }
}

/// Convert an Iced keyboard event into VT100/xterm byte sequences suitable
/// for writing to a PTY.
///
/// `is_app_cursor` should reflect whether `TermMode::APP_CURSOR` is active.
pub fn key_to_bytes(
    key: &Key,
    text: Option<&str>,
    modifiers: &Modifiers,
    is_app_cursor: bool,
) -> Option<Vec<u8>> {
    let mod_no = modifier_number(modifiers);

    // Ctrl+letter: map a-z to 0x01-0x1A.
    if modifiers.control() {
        if let Key::Character(c) = key {
            let s = c.as_str();
            if s.len() == 1 {
                let ch = s.as_bytes()[0];
                if ch.is_ascii_lowercase() {
                    let mut bytes = vec![ch & 0x1f];
                    if modifiers.alt() {
                        bytes.insert(0, 0x1b);
                    }
                    return Some(bytes);
                }
                if ch.is_ascii_uppercase() {
                    let mut bytes = vec![ch.to_ascii_lowercase() & 0x1f];
                    if modifiers.alt() {
                        bytes.insert(0, 0x1b);
                    }
                    return Some(bytes);
                }
            }
        }
    }

    // Named keys.
    if let Key::Named(named) = key {
        let bytes = match named {
            Named::Enter => Some(b"\r".to_vec()),
            Named::Tab => {
                if modifiers.shift() {
                    Some(b"\x1b[Z".to_vec())
                } else {
                    Some(b"\t".to_vec())
                }
            }
            Named::Escape => Some(b"\x1b".to_vec()),
            Named::Backspace => {
                let mut b = vec![0x7f];
                if modifiers.alt() {
                    b.insert(0, 0x1b);
                }
                Some(b)
            }
            Named::Space => {
                if modifiers.control() {
                    Some(vec![0x00])
                } else {
                    Some(b" ".to_vec())
                }
            }

            // Arrow keys: SS3 in app-cursor mode, CSI otherwise.
            Named::ArrowUp => Some(if is_app_cursor && mod_no == 1 {
                ss3("A", mod_no)
            } else {
                csi2("A", mod_no)
            }),
            Named::ArrowDown => Some(if is_app_cursor && mod_no == 1 {
                ss3("B", mod_no)
            } else {
                csi2("B", mod_no)
            }),
            Named::ArrowRight => Some(if is_app_cursor && mod_no == 1 {
                ss3("C", mod_no)
            } else {
                csi2("C", mod_no)
            }),
            Named::ArrowLeft => Some(if is_app_cursor && mod_no == 1 {
                ss3("D", mod_no)
            } else {
                csi2("D", mod_no)
            }),

            // Home / End.
            Named::Home => Some(if is_app_cursor && mod_no == 1 {
                ss3("H", mod_no)
            } else {
                csi2("H", mod_no)
            }),
            Named::End => Some(if is_app_cursor && mod_no == 1 {
                ss3("F", mod_no)
            } else {
                csi2("F", mod_no)
            }),

            // Insert, Delete, PageUp, PageDown.
            Named::Insert => Some(csi("2", "~", mod_no)),
            Named::Delete => Some(csi("3", "~", mod_no)),
            Named::PageUp => Some(csi("5", "~", mod_no)),
            Named::PageDown => Some(csi("6", "~", mod_no)),

            // F1-F4: SS3 P/Q/R/S.
            Named::F1 => Some(ss3("P", mod_no)),
            Named::F2 => Some(ss3("Q", mod_no)),
            Named::F3 => Some(ss3("R", mod_no)),
            Named::F4 => Some(ss3("S", mod_no)),

            // F5-F12: CSI codes with ~ suffix.
            Named::F5 => Some(csi("15", "~", mod_no)),
            Named::F6 => Some(csi("17", "~", mod_no)),
            Named::F7 => Some(csi("18", "~", mod_no)),
            Named::F8 => Some(csi("19", "~", mod_no)),
            Named::F9 => Some(csi("20", "~", mod_no)),
            Named::F10 => Some(csi("21", "~", mod_no)),
            Named::F11 => Some(csi("23", "~", mod_no)),
            Named::F12 => Some(csi("24", "~", mod_no)),

            // Modifier-only keys and others we don't send.
            Named::Shift | Named::Control | Named::Alt | Named::Super | Named::Meta => None,

            _ => None,
        };
        if bytes.is_some() {
            return bytes;
        }
    }

    // Regular text input (printable characters).
    if let Some(txt) = text {
        if !txt.is_empty() {
            // Skip if this is a Ctrl+key combo already handled above.
            if modifiers.control() {
                return None;
            }
            let mut bytes = txt.as_bytes().to_vec();
            if modifiers.alt() {
                bytes.insert(0, 0x1b);
            }
            return Some(bytes);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_mods() -> Modifiers {
        Modifiers::empty()
    }

    #[test]
    fn test_arrow_normal() {
        let result = key_to_bytes(
            &Key::Named(Named::ArrowUp),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(b"\x1b[A".to_vec()));
    }

    #[test]
    fn test_arrow_app_cursor() {
        let result = key_to_bytes(
            &Key::Named(Named::ArrowUp),
            None,
            &no_mods(),
            true,
        );
        assert_eq!(result, Some(b"\x1bOA".to_vec()));
    }

    #[test]
    fn test_ctrl_c() {
        let key = Key::Character("c".into());
        let mods = Modifiers::CTRL;
        let result = key_to_bytes(&key, Some("c"), &mods, false);
        assert_eq!(result, Some(vec![0x03]));
    }

    #[test]
    fn test_ctrl_d() {
        let key = Key::Character("d".into());
        let mods = Modifiers::CTRL;
        let result = key_to_bytes(&key, Some("d"), &mods, false);
        assert_eq!(result, Some(vec![0x04]));
    }

    #[test]
    fn test_f5() {
        let result = key_to_bytes(
            &Key::Named(Named::F5),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(b"\x1b[15~".to_vec()));
    }

    #[test]
    fn test_shift_arrow() {
        let mods = Modifiers::SHIFT;
        let result = key_to_bytes(
            &Key::Named(Named::ArrowUp),
            None,
            &mods,
            false,
        );
        assert_eq!(result, Some(b"\x1b[1;2A".to_vec()));
    }

    #[test]
    fn test_enter() {
        let result = key_to_bytes(
            &Key::Named(Named::Enter),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(b"\r".to_vec()));
    }

    #[test]
    fn test_tab() {
        let result = key_to_bytes(
            &Key::Named(Named::Tab),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(b"\t".to_vec()));
    }

    #[test]
    fn test_shift_tab() {
        let mods = Modifiers::SHIFT;
        let result = key_to_bytes(
            &Key::Named(Named::Tab),
            None,
            &mods,
            false,
        );
        assert_eq!(result, Some(b"\x1b[Z".to_vec()));
    }

    #[test]
    fn test_escape() {
        let result = key_to_bytes(
            &Key::Named(Named::Escape),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(b"\x1b".to_vec()));
    }

    #[test]
    fn test_backspace() {
        let result = key_to_bytes(
            &Key::Named(Named::Backspace),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(vec![0x7f]));
    }

    #[test]
    fn test_alt_backspace() {
        let mods = Modifiers::ALT;
        let result = key_to_bytes(
            &Key::Named(Named::Backspace),
            None,
            &mods,
            false,
        );
        assert_eq!(result, Some(vec![0x1b, 0x7f]));
    }

    #[test]
    fn test_regular_text() {
        let key = Key::Character("a".into());
        let result = key_to_bytes(&key, Some("a"), &no_mods(), false);
        assert_eq!(result, Some(b"a".to_vec()));
    }

    #[test]
    fn test_alt_text() {
        let key = Key::Character("a".into());
        let mods = Modifiers::ALT;
        let result = key_to_bytes(&key, Some("a"), &mods, false);
        assert_eq!(result, Some(vec![0x1b, b'a']));
    }

    #[test]
    fn test_delete() {
        let result = key_to_bytes(
            &Key::Named(Named::Delete),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(b"\x1b[3~".to_vec()));
    }

    #[test]
    fn test_insert() {
        let result = key_to_bytes(
            &Key::Named(Named::Insert),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(b"\x1b[2~".to_vec()));
    }

    #[test]
    fn test_page_up() {
        let result = key_to_bytes(
            &Key::Named(Named::PageUp),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(b"\x1b[5~".to_vec()));
    }

    #[test]
    fn test_f1() {
        let result = key_to_bytes(
            &Key::Named(Named::F1),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(b"\x1bOP".to_vec()));
    }

    #[test]
    fn test_f12() {
        let result = key_to_bytes(
            &Key::Named(Named::F12),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(b"\x1b[24~".to_vec()));
    }

    #[test]
    fn test_modifier_only_returns_none() {
        let result = key_to_bytes(
            &Key::Named(Named::Shift),
            None,
            &Modifiers::SHIFT,
            false,
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_home_normal() {
        let result = key_to_bytes(
            &Key::Named(Named::Home),
            None,
            &no_mods(),
            false,
        );
        assert_eq!(result, Some(b"\x1b[H".to_vec()));
    }

    #[test]
    fn test_home_app_cursor() {
        let result = key_to_bytes(
            &Key::Named(Named::Home),
            None,
            &no_mods(),
            true,
        );
        assert_eq!(result, Some(b"\x1bOH".to_vec()));
    }

    #[test]
    fn test_ctrl_alt_c() {
        let key = Key::Character("c".into());
        let mods = Modifiers::CTRL | Modifiers::ALT;
        let result = key_to_bytes(&key, Some("c"), &mods, false);
        assert_eq!(result, Some(vec![0x1b, 0x03]));
    }
}
