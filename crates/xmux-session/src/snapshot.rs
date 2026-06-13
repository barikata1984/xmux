use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub version: u32,
    pub timestamp: u64,
    pub workspaces: Vec<WorkspaceSnapshot>,
    pub active_workspace: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub id: String,
    pub name: String,
    pub layout: LayoutNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    Pane(PaneSnapshot),
    Split {
        axis: SplitAxis,
        ratio: f32,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneSnapshot {
    pub working_dir: PathBuf,
    pub title: String,
    pub env_overrides: HashMap<String, String>,
}

impl SessionSnapshot {
    pub fn new(workspaces: Vec<WorkspaceSnapshot>, active: usize) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        Self {
            version: 1,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            workspaces,
            active_workspace: active,
        }
    }

    pub fn save_dir() -> PathBuf {
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("xmux").join("sessions")
    }

    pub fn save(&self) -> Result<PathBuf, String> {
        let dir = Self::save_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let filename = format!("session_{}.json", self.timestamp);
        let path = dir.join(&filename);
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        // Write to tmp file first, then rename for atomicity
        let tmp_path = dir.join(format!("{}.tmp", filename));
        std::fs::write(&tmp_path, &json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
        // Keep only latest 5 snapshots
        Self::cleanup_old(&dir);
        Ok(path)
    }

    pub fn load_latest() -> Result<Option<Self>, String> {
        let dir = Self::save_dir();
        if !dir.exists() {
            return Ok(None);
        }
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().map_or(false, |ext| ext == "json")
                    && e.file_name().to_str().map_or(false, |n| n.starts_with("session_"))
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());
        if let Some(latest) = entries.last() {
            let content = std::fs::read_to_string(latest.path()).map_err(|e| e.to_string())?;
            let snapshot: Self = serde_json::from_str(&content).map_err(|e| e.to_string())?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    fn cleanup_old(dir: &std::path::Path) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().map_or(false, |ext| ext == "json")
                    && e.file_name().to_str().map_or(false, |n| n.starts_with("session_"))
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());
        while entries.len() > 5 {
            if let Some(old) = entries.first() {
                let _ = std::fs::remove_file(old.path());
            }
            entries.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        // Create a SessionSnapshot with sample data
        let pane = PaneSnapshot {
            working_dir: PathBuf::from("/home/user"),
            title: "shell".to_string(),
            env_overrides: [("TERM".to_string(), "xterm-256color".to_string())]
                .iter()
                .cloned()
                .collect(),
        };

        let workspace = WorkspaceSnapshot {
            id: "ws-1".to_string(),
            name: "main".to_string(),
            layout: LayoutNode::Pane(pane),
        };

        let snapshot = SessionSnapshot::new(vec![workspace], 0);

        // Serialize to JSON string
        let json = serde_json::to_string(&snapshot).expect("Failed to serialize");

        // Deserialize back from JSON
        let deserialized: SessionSnapshot =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify fields match
        assert_eq!(snapshot.version, deserialized.version);
        assert_eq!(snapshot.timestamp, deserialized.timestamp);
        assert_eq!(snapshot.active_workspace, deserialized.active_workspace);
        assert_eq!(snapshot.workspaces.len(), deserialized.workspaces.len());

        let orig_ws = &snapshot.workspaces[0];
        let deser_ws = &deserialized.workspaces[0];
        assert_eq!(orig_ws.id, deser_ws.id);
        assert_eq!(orig_ws.name, deser_ws.name);
    }

    #[test]
    fn test_layout_roundtrip() {
        // Create a Split layout with two Pane children
        let pane1 = PaneSnapshot {
            working_dir: PathBuf::from("/home/user/project1"),
            title: "editor".to_string(),
            env_overrides: HashMap::new(),
        };

        let pane2 = PaneSnapshot {
            working_dir: PathBuf::from("/home/user/project2"),
            title: "terminal".to_string(),
            env_overrides: HashMap::new(),
        };

        let split_layout = LayoutNode::Split {
            axis: SplitAxis::Vertical,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane(pane1)),
            second: Box::new(LayoutNode::Pane(pane2)),
        };

        let workspace = WorkspaceSnapshot {
            id: "ws-split".to_string(),
            name: "split_workspace".to_string(),
            layout: split_layout,
        };

        let snapshot = SessionSnapshot::new(vec![workspace], 0);

        // Roundtrip through JSON
        let json = serde_json::to_string_pretty(&snapshot).expect("Failed to serialize");
        let deserialized: SessionSnapshot =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify structure
        assert_eq!(snapshot.workspaces.len(), deserialized.workspaces.len());
        let ws = &deserialized.workspaces[0];
        assert_eq!(ws.id, "ws-split");
        assert_eq!(ws.name, "split_workspace");

        // Verify it's a Split layout
        match &ws.layout {
            LayoutNode::Split { axis, ratio, .. } => {
                assert_eq!(*axis as u8, SplitAxis::Vertical as u8);
                assert!((ratio - 0.5).abs() < 0.001);
            }
            _ => panic!("Expected Split layout"),
        }
    }

    #[test]
    fn test_save_dir() {
        let path = SessionSnapshot::save_dir();
        let path_str = path.to_string_lossy();
        assert!(path_str.ends_with("xmux/sessions"));
    }
}
