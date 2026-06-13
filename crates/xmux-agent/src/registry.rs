use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub display_name: String,
    pub detect_env: Vec<String>,
    pub hook_script: String,
    pub resume_command: Option<String>,
}

pub struct AgentRegistry {
    agents: Vec<AgentConfig>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            agents: Vec::new(),
        };
        registry.register_builtin_agents();
        registry
    }

    fn register_builtin_agents(&mut self) {
        self.agents.push(AgentConfig {
            name: "claude-code".into(),
            display_name: "Claude Code".into(),
            detect_env: vec!["CLAUDE_CODE".into()],
            hook_script: "xmux notify --title 'Claude Code' --body 'Task completed'".into(),
            resume_command: Some("claude --resume".into()),
        });
        self.agents.push(AgentConfig {
            name: "codex".into(),
            display_name: "Codex".into(),
            detect_env: vec!["CODEX_SESSION".into()],
            hook_script: "xmux notify --title 'Codex' --body 'Task completed'".into(),
            resume_command: None,
        });
        self.agents.push(AgentConfig {
            name: "gemini".into(),
            display_name: "Gemini CLI".into(),
            detect_env: vec!["GEMINI_CLI".into()],
            hook_script: "xmux notify --title 'Gemini' --body 'Task completed'".into(),
            resume_command: None,
        });
        self.agents.push(AgentConfig {
            name: "copilot".into(),
            display_name: "GitHub Copilot".into(),
            detect_env: vec!["GITHUB_COPILOT".into()],
            hook_script: "xmux notify --title 'Copilot' --body 'Task completed'".into(),
            resume_command: None,
        });
        self.agents.push(AgentConfig {
            name: "amp".into(),
            display_name: "Amp".into(),
            detect_env: vec!["AMP_SESSION".into()],
            hook_script: "xmux notify --title 'Amp' --body 'Task completed'".into(),
            resume_command: None,
        });
    }

    pub fn register(&mut self, config: AgentConfig) {
        self.agents.push(config);
    }

    pub fn detect_agent(&self, env: &HashMap<String, String>) -> Option<&AgentConfig> {
        self.agents.iter().find(|agent| {
            agent
                .detect_env
                .iter()
                .any(|key| env.contains_key(key))
        })
    }

    pub fn get(&self, name: &str) -> Option<&AgentConfig> {
        self.agents.iter().find(|a| a.name == name)
    }

    pub fn list(&self) -> &[AgentConfig] {
        &self.agents
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_claude_code() {
        let registry = AgentRegistry::new();
        let mut env = HashMap::new();
        env.insert("CLAUDE_CODE".to_string(), "1".to_string());

        let agent = registry.detect_agent(&env);
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().name, "claude-code");
    }

    #[test]
    fn test_detect_none() {
        let registry = AgentRegistry::new();
        let env = HashMap::new();

        let agent = registry.detect_agent(&env);
        assert!(agent.is_none());
    }

    #[test]
    fn test_register_custom() {
        let mut registry = AgentRegistry::new();
        let custom = AgentConfig {
            name: "custom".into(),
            display_name: "Custom Agent".into(),
            detect_env: vec!["CUSTOM_AGENT".into()],
            hook_script: "echo custom".into(),
            resume_command: None,
        };

        registry.register(custom);

        let mut env = HashMap::new();
        env.insert("CUSTOM_AGENT".to_string(), "1".to_string());

        let agent = registry.detect_agent(&env);
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().name, "custom");
    }

    #[test]
    fn test_get_agent() {
        let registry = AgentRegistry::new();
        let agent = registry.get("claude-code");
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().display_name, "Claude Code");
    }

    #[test]
    fn test_list_agents() {
        let registry = AgentRegistry::new();
        let agents = registry.list();
        assert_eq!(agents.len(), 5);
    }

    #[test]
    fn test_default() {
        let registry = AgentRegistry::default();
        assert!(!registry.list().is_empty());
    }
}
