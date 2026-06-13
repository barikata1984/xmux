use std::path::PathBuf;
use std::fs;
use crate::registry::AgentConfig;

pub struct HookInstaller;

impl HookInstaller {
    /// Generate the hook script content for a given agent
    pub fn generate_hook_script(agent: &AgentConfig) -> String {
        agent.hook_script.clone()
    }

    /// Get the config file path for a known agent
    pub fn config_path(agent_name: &str) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        match agent_name {
            "claude-code" => Some(home.join(".claude").join("settings.json")),
            _ => None,
        }
    }

    /// Install hooks for Claude Code by updating settings.json
    pub fn install_claude_code_hooks() -> Result<(), String> {
        let home = dirs::home_dir().ok_or("cannot find home dir")?;
        let settings_path = home.join(".claude").join("settings.json");

        let mut settings: serde_json::Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        // Add xmux notification hook to hooks section
        let hooks = settings.as_object_mut().unwrap()
            .entry("hooks").or_insert(serde_json::json!({}));
        if let Some(hooks_obj) = hooks.as_object_mut() {
            hooks_obj.entry("Stop").or_insert(serde_json::json!([
                {"command": "xmux notify --title 'Claude Code' --body 'Task completed'"}
            ]));
        }

        // Ensure parent dir exists
        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
        fs::write(&settings_path, content).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// List installed hooks
    pub fn list_installed() -> Vec<(String, PathBuf)> {
        let mut installed = Vec::new();
        if let Some(path) = Self::config_path("claude-code") {
            if path.exists() {
                installed.push(("claude-code".into(), path));
            }
        }
        installed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_hook_script() {
        let agent = AgentConfig {
            name: "claude-code".into(),
            display_name: "Claude Code".into(),
            detect_env: vec!["CLAUDE_CODE".into()],
            hook_script: "xmux notify --title 'Claude Code' --body 'Task completed'".into(),
            resume_command: Some("claude --resume".into()),
        };

        let script = HookInstaller::generate_hook_script(&agent);
        assert_eq!(script, "xmux notify --title 'Claude Code' --body 'Task completed'");
    }

    #[test]
    fn test_config_path_claude() {
        let path = HookInstaller::config_path("claude-code");
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.ends_with(".claude/settings.json"));
    }

    #[test]
    fn test_config_path_unknown() {
        let path = HookInstaller::config_path("unknown-agent");
        assert!(path.is_none());
    }
}
