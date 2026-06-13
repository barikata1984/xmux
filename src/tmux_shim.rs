/// tmux-style command parser and xmux RPC mapper
///
/// Maps tmux commands to xmux RPC method names and parameters.
/// This allows applications expecting tmux-like commands to work with xmux.

/// Parse a tmux-style command and map it to an xmux RPC method with parameters.
///
/// Returns Some((method_name, params)) for recognized tmux commands,
/// or None for unrecognized commands.
pub fn parse_tmux_command(args: &[&str]) -> Option<(&'static str, serde_json::Value)> {
    match args.first().copied() {
        Some("new-session") => Some(("workspace.create", serde_json::json!({}))),
        Some("split-window") => {
            let dir = if args.contains(&"-h") { "right" } else { "down" };
            Some(("surface.split", serde_json::json!({"direction": dir})))
        }
        Some("send-keys") => {
            let text = args
                .iter()
                .skip(1)
                .filter(|a| !a.starts_with('-'))
                .copied()
                .collect::<Vec<_>>()
                .join(" ");
            Some(("surface.send_text", serde_json::json!({"text": text})))
        }
        Some("list-sessions") => Some(("workspace.list", serde_json::json!({}))),
        Some("list-panes") => Some(("surface.list", serde_json::json!({}))),
        Some("display-message") => Some(("system.ping", serde_json::json!({}))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session() {
        let result = parse_tmux_command(&["new-session"]);
        assert!(result.is_some());
        let (method, _params) = result.unwrap();
        assert_eq!(method, "workspace.create");
    }

    #[test]
    fn test_split_window_horizontal() {
        let result = parse_tmux_command(&["split-window", "-h"]);
        assert!(result.is_some());
        let (method, params) = result.unwrap();
        assert_eq!(method, "surface.split");
        assert_eq!(params["direction"], "right");
    }

    #[test]
    fn test_split_window_vertical() {
        let result = parse_tmux_command(&["split-window"]);
        assert!(result.is_some());
        let (method, params) = result.unwrap();
        assert_eq!(method, "surface.split");
        assert_eq!(params["direction"], "down");
    }

    #[test]
    fn test_send_keys() {
        let result = parse_tmux_command(&["send-keys", "hello", "world"]);
        assert!(result.is_some());
        let (method, params) = result.unwrap();
        assert_eq!(method, "surface.send_text");
        assert_eq!(params["text"], "hello world");
    }

    #[test]
    fn test_send_keys_with_flags() {
        let result = parse_tmux_command(&["send-keys", "-t", "session", "hello"]);
        assert!(result.is_some());
        let (_method, params) = result.unwrap();
        // Non-flag args after command are included: "-t" is filtered, but "session" and "hello" are kept
        assert_eq!(params["text"], "session hello");
    }

    #[test]
    fn test_list_sessions() {
        let result = parse_tmux_command(&["list-sessions"]);
        assert!(result.is_some());
        let (method, _params) = result.unwrap();
        assert_eq!(method, "workspace.list");
    }

    #[test]
    fn test_list_panes() {
        let result = parse_tmux_command(&["list-panes"]);
        assert!(result.is_some());
        let (method, _params) = result.unwrap();
        assert_eq!(method, "surface.list");
    }

    #[test]
    fn test_display_message() {
        let result = parse_tmux_command(&["display-message"]);
        assert!(result.is_some());
        let (method, _params) = result.unwrap();
        assert_eq!(method, "system.ping");
    }

    #[test]
    fn test_unknown_command() {
        let result = parse_tmux_command(&["unknown"]);
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_args() {
        let result = parse_tmux_command(&[]);
        assert!(result.is_none());
    }
}
