use crate::snapshot::{LayoutNode, PaneSnapshot, SessionSnapshot};

/// Result of restoring a session from a snapshot
#[derive(Debug, Clone)]
pub struct RestoreResult {
    pub workspaces: Vec<RestoredWorkspace>,
    pub active_workspace: usize,
}

/// A restored workspace with its layout and panes
#[derive(Debug, Clone)]
pub struct RestoredWorkspace {
    pub id: String,
    pub name: String,
    pub panes: Vec<RestoredPane>,
    pub layout: LayoutNode,
}

/// A restored pane with its snapshot data
#[derive(Debug, Clone)]
pub struct RestoredPane {
    pub snapshot: PaneSnapshot,
}

impl SessionSnapshot {
    /// Restore a session from a snapshot
    ///
    /// Returns a RestoreResult containing all workspaces and the active workspace index.
    /// Panes are extracted from the layout tree for easy enumeration.
    pub fn restore(&self) -> RestoreResult {
        let workspaces = self.workspaces.iter().map(|ws| {
            let panes = collect_panes(&ws.layout);
            RestoredWorkspace {
                id: ws.id.clone(),
                name: ws.name.clone(),
                panes,
                layout: ws.layout.clone(),
            }
        }).collect();

        RestoreResult {
            workspaces,
            active_workspace: self.active_workspace,
        }
    }
}

/// Recursively collect all panes from a layout node
fn collect_panes(node: &LayoutNode) -> Vec<RestoredPane> {
    match node {
        LayoutNode::Pane(snap) => vec![RestoredPane {
            snapshot: snap.clone(),
        }],
        LayoutNode::Split {
            first,
            second,
            ..
        } => {
            let mut panes = collect_panes(first);
            panes.extend(collect_panes(second));
            panes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::WorkspaceSnapshot;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_restore_single_pane() {
        // Create snapshot with 1 workspace, 1 pane
        let pane = PaneSnapshot {
            working_dir: PathBuf::from("/home/user"),
            title: "shell".to_string(),
            env_overrides: HashMap::new(),
        };

        let workspace = WorkspaceSnapshot {
            id: "ws-1".to_string(),
            name: "main".to_string(),
            layout: LayoutNode::Pane(pane),
        };

        let snapshot = SessionSnapshot::new(vec![workspace], 0);

        // Restore and verify
        let result = snapshot.restore();

        assert_eq!(result.workspaces.len(), 1);
        assert_eq!(result.active_workspace, 0);

        let restored_ws = &result.workspaces[0];
        assert_eq!(restored_ws.id, "ws-1");
        assert_eq!(restored_ws.name, "main");
        assert_eq!(restored_ws.panes.len(), 1);

        let restored_pane = &restored_ws.panes[0];
        assert_eq!(restored_pane.snapshot.working_dir, PathBuf::from("/home/user"));
        assert_eq!(restored_pane.snapshot.title, "shell");
    }

    #[test]
    fn test_restore_split() {
        // Create snapshot with split layout (2 panes)
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
            axis: crate::snapshot::SplitAxis::Vertical,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane(pane1)),
            second: Box::new(LayoutNode::Pane(pane2)),
        };

        let workspace = WorkspaceSnapshot {
            id: "ws-split".to_string(),
            name: "split_ws".to_string(),
            layout: split_layout,
        };

        let snapshot = SessionSnapshot::new(vec![workspace], 0);

        // Restore and verify
        let result = snapshot.restore();

        assert_eq!(result.workspaces.len(), 1);

        let restored_ws = &result.workspaces[0];
        assert_eq!(restored_ws.id, "ws-split");
        assert_eq!(restored_ws.name, "split_ws");
        assert_eq!(restored_ws.panes.len(), 2);

        // Verify first pane
        assert_eq!(
            restored_ws.panes[0].snapshot.working_dir,
            PathBuf::from("/home/user/project1")
        );
        assert_eq!(restored_ws.panes[0].snapshot.title, "editor");

        // Verify second pane
        assert_eq!(
            restored_ws.panes[1].snapshot.working_dir,
            PathBuf::from("/home/user/project2")
        );
        assert_eq!(restored_ws.panes[1].snapshot.title, "terminal");
    }

    #[test]
    fn test_restore_active_workspace() {
        // Create multiple workspaces and verify active_workspace is preserved
        let pane1 = PaneSnapshot {
            working_dir: PathBuf::from("/home/user/ws1"),
            title: "ws1".to_string(),
            env_overrides: HashMap::new(),
        };

        let pane2 = PaneSnapshot {
            working_dir: PathBuf::from("/home/user/ws2"),
            title: "ws2".to_string(),
            env_overrides: HashMap::new(),
        };

        let pane3 = PaneSnapshot {
            working_dir: PathBuf::from("/home/user/ws3"),
            title: "ws3".to_string(),
            env_overrides: HashMap::new(),
        };

        let workspaces = vec![
            WorkspaceSnapshot {
                id: "ws-1".to_string(),
                name: "workspace1".to_string(),
                layout: LayoutNode::Pane(pane1),
            },
            WorkspaceSnapshot {
                id: "ws-2".to_string(),
                name: "workspace2".to_string(),
                layout: LayoutNode::Pane(pane2),
            },
            WorkspaceSnapshot {
                id: "ws-3".to_string(),
                name: "workspace3".to_string(),
                layout: LayoutNode::Pane(pane3),
            },
        ];

        // Set active workspace to index 1 (middle one)
        let snapshot = SessionSnapshot::new(workspaces, 1);

        let result = snapshot.restore();

        // Verify active_workspace index is preserved
        assert_eq!(result.active_workspace, 1);
        assert_eq!(result.workspaces.len(), 3);
        assert_eq!(result.workspaces[1].name, "workspace2");
    }

    #[test]
    fn test_restore_deeply_nested_splits() {
        // Create a complex nested split: ((A | B) | C)
        let pane_a = PaneSnapshot {
            working_dir: PathBuf::from("/a"),
            title: "pane_a".to_string(),
            env_overrides: HashMap::new(),
        };

        let pane_b = PaneSnapshot {
            working_dir: PathBuf::from("/b"),
            title: "pane_b".to_string(),
            env_overrides: HashMap::new(),
        };

        let pane_c = PaneSnapshot {
            working_dir: PathBuf::from("/c"),
            title: "pane_c".to_string(),
            env_overrides: HashMap::new(),
        };

        let left_split = LayoutNode::Split {
            axis: crate::snapshot::SplitAxis::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane(pane_a)),
            second: Box::new(LayoutNode::Pane(pane_b)),
        };

        let root_split = LayoutNode::Split {
            axis: crate::snapshot::SplitAxis::Vertical,
            ratio: 0.6,
            first: Box::new(left_split),
            second: Box::new(LayoutNode::Pane(pane_c)),
        };

        let workspace = WorkspaceSnapshot {
            id: "ws-nested".to_string(),
            name: "nested_split".to_string(),
            layout: root_split,
        };

        let snapshot = SessionSnapshot::new(vec![workspace], 0);
        let result = snapshot.restore();

        let restored_ws = &result.workspaces[0];
        // Should collect all 3 panes regardless of nesting
        assert_eq!(restored_ws.panes.len(), 3);

        // Verify panes are collected in order (depth-first left-to-right)
        assert_eq!(restored_ws.panes[0].snapshot.title, "pane_a");
        assert_eq!(restored_ws.panes[1].snapshot.title, "pane_b");
        assert_eq!(restored_ws.panes[2].snapshot.title, "pane_c");
    }
}
