use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationId(pub Uuid);

impl NotificationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NotificationId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscProtocol {
    Osc9,
    Osc99,
    Osc777,
}

#[derive(Debug, Clone)]
pub struct OscNotification {
    pub id: NotificationId,
    pub protocol: OscProtocol,
    pub title: Option<String>,
    pub body: String,
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    Normal,
    Escape,      // received ESC (0x1b)
    OscParam,    // received ESC ], reading param digits
    OscData,     // received ESC ] <param> ;, reading data
    OscEscape,   // received ESC inside OSC data (potential ST)
}

pub struct OscParser {
    state: ParserState,
    param_buf: Vec<u8>,
    data_buf: Vec<u8>,
}

impl OscParser {
    pub fn new() -> Self {
        Self {
            state: ParserState::Normal,
            param_buf: Vec::with_capacity(8),
            data_buf: Vec::with_capacity(256),
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<OscNotification> {
        let mut notifications = Vec::new();
        for &byte in bytes {
            if let Some(notif) = self.process_byte(byte) {
                notifications.push(notif);
            }
        }
        notifications
    }

    fn process_byte(&mut self, byte: u8) -> Option<OscNotification> {
        match self.state {
            ParserState::Normal => {
                if byte == 0x1b {
                    self.state = ParserState::Escape;
                }
                None
            }
            ParserState::Escape => {
                if byte == 0x5d {
                    // ]
                    self.state = ParserState::OscParam;
                    self.param_buf.clear();
                    self.data_buf.clear();
                } else {
                    self.state = ParserState::Normal;
                }
                None
            }
            ParserState::OscParam => {
                if byte == b';' {
                    self.state = ParserState::OscData;
                    None
                } else if byte.is_ascii_digit() {
                    self.param_buf.push(byte);
                    None
                } else {
                    // Not a valid OSC param, reset
                    self.state = ParserState::Normal;
                    None
                }
            }
            ParserState::OscData => {
                if byte == 0x07 {
                    // BEL - end of OSC
                    let result = self.try_parse_notification();
                    self.state = ParserState::Normal;
                    result
                } else if byte == 0x1b {
                    self.state = ParserState::OscEscape;
                    None
                } else {
                    self.data_buf.push(byte);
                    None
                }
            }
            ParserState::OscEscape => {
                if byte == 0x5c {
                    // \ - ST (String Terminator)
                    let result = self.try_parse_notification();
                    self.state = ParserState::Normal;
                    result
                } else if byte == 0x5d {
                    // ] - new OSC starting
                    // Treat previous as aborted, start new OSC
                    self.param_buf.clear();
                    self.data_buf.clear();
                    self.state = ParserState::OscParam;
                    None
                } else {
                    self.state = ParserState::Normal;
                    None
                }
            }
        }
    }

    fn try_parse_notification(&self) -> Option<OscNotification> {
        let param = std::str::from_utf8(&self.param_buf).ok()?;
        let data = String::from_utf8_lossy(&self.data_buf);

        match param {
            "9" => Some(OscNotification {
                id: NotificationId::new(),
                protocol: OscProtocol::Osc9,
                title: None,
                body: data.into_owned(),
                external_id: None,
            }),
            "99" => {
                // Format: i=<id>:<body> or just <body>
                let (external_id, body) = if let Some(rest) = data.strip_prefix("i=") {
                    if let Some((id, body)) = rest.split_once(':') {
                        (Some(id.to_string()), body.to_string())
                    } else {
                        (None, data.into_owned())
                    }
                } else {
                    (None, data.into_owned())
                };
                Some(OscNotification {
                    id: NotificationId::new(),
                    protocol: OscProtocol::Osc99,
                    title: None,
                    body,
                    external_id,
                })
            }
            "777" => {
                // Format: notify;<title>;<body>
                let parts: Vec<&str> = data.splitn(3, ';').collect();
                if parts.len() >= 3 && parts[0] == "notify" {
                    Some(OscNotification {
                        id: NotificationId::new(),
                        protocol: OscProtocol::Osc777,
                        title: Some(parts[1].to_string()),
                        body: parts[2].to_string(),
                        external_id: None,
                    })
                } else if parts.len() >= 2 && parts[0] == "notify" {
                    Some(OscNotification {
                        id: NotificationId::new(),
                        protocol: OscProtocol::Osc777,
                        title: None,
                        body: parts[1].to_string(),
                        external_id: None,
                    })
                } else {
                    None
                }
            }
            _ => None, // Not a notification OSC
        }
    }
}

impl Default for OscParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc9_bel() {
        let mut p = OscParser::new();
        let input = b"\x1b]9;hello\x07";
        let notifs = p.feed(input);
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].protocol, OscProtocol::Osc9);
        assert_eq!(notifs[0].body, "hello");
        assert!(notifs[0].title.is_none());
    }

    #[test]
    fn test_osc9_st() {
        let mut p = OscParser::new();
        let input = b"\x1b]9;world\x1b\\";
        let notifs = p.feed(input);
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].body, "world");
    }

    #[test]
    fn test_osc777_notify() {
        let mut p = OscParser::new();
        let input = b"\x1b]777;notify;Build;Done!\x07";
        let notifs = p.feed(input);
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].protocol, OscProtocol::Osc777);
        assert_eq!(notifs[0].title.as_deref(), Some("Build"));
        assert_eq!(notifs[0].body, "Done!");
    }

    #[test]
    fn test_osc99() {
        let mut p = OscParser::new();
        let input = b"\x1b]99;i=123:task complete\x07";
        let notifs = p.feed(input);
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].protocol, OscProtocol::Osc99);
        assert_eq!(notifs[0].body, "task complete");
        assert_eq!(notifs[0].external_id.as_deref(), Some("123"));
    }

    #[test]
    fn test_non_notification_osc_ignored() {
        let mut p = OscParser::new();
        let input = b"\x1b]0;My Title\x07";
        let notifs = p.feed(input);
        assert_eq!(notifs.len(), 0);
    }

    #[test]
    fn test_mixed_data() {
        let mut p = OscParser::new();
        let input = b"some text\x1b]9;alert\x07more text";
        let notifs = p.feed(input);
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].body, "alert");
    }

    #[test]
    fn test_multiple_notifications() {
        let mut p = OscParser::new();
        let input = b"\x1b]9;first\x07\x1b]9;second\x07";
        let notifs = p.feed(input);
        assert_eq!(notifs.len(), 2);
        assert_eq!(notifs[0].body, "first");
        assert_eq!(notifs[1].body, "second");
    }

    #[test]
    fn test_split_across_feeds() {
        let mut p = OscParser::new();
        assert!(p.feed(b"\x1b]").is_empty());
        assert!(p.feed(b"9;hel").is_empty());
        let notifs = p.feed(b"lo\x07");
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].body, "hello");
    }

    #[test]
    fn test_incomplete_osc_reset() {
        let mut p = OscParser::new();
        // Incomplete OSC followed by regular ESC sequence
        p.feed(b"\x1b]9;partial");
        // New ESC ] should restart
        let notifs = p.feed(b"\x1b]9;complete\x07");
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].body, "complete");
    }

    #[test]
    fn test_osc777_no_title() {
        let mut p = OscParser::new();
        let input = b"\x1b]777;notify;body only\x07";
        let notifs = p.feed(input);
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].protocol, OscProtocol::Osc777);
        assert!(notifs[0].title.is_none());
        assert_eq!(notifs[0].body, "body only");
    }

    #[test]
    fn test_empty_body() {
        let mut p = OscParser::new();
        let input = b"\x1b]9;\x07";
        let notifs = p.feed(input);
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].body, "");
    }
}
