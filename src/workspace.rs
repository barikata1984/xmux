use iced::widget::pane_grid;
use xmux_core::WorkspaceId;
use crate::pane::PaneState;

pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub panes: pane_grid::State<PaneState>,
    pub focus: Option<pane_grid::Pane>,
}

impl Workspace {
    pub fn new(name: String) -> Result<Self, xmux_core::XmuxError> {
        let initial = PaneState::new()?;
        let (panes, first_pane) = pane_grid::State::new(initial);
        Ok(Self {
            id: WorkspaceId::new(),
            name,
            panes,
            focus: Some(first_pane),
        })
    }
}

pub struct WorkspaceManager {
    pub workspaces: Vec<Workspace>,
    pub active_index: usize,
}

impl WorkspaceManager {
    pub fn new() -> Result<Self, xmux_core::XmuxError> {
        let ws = Workspace::new("Workspace 1".into())?;
        Ok(Self {
            workspaces: vec![ws],
            active_index: 0,
        })
    }

    pub fn active(&self) -> &Workspace {
        &self.workspaces[self.active_index]
    }

    pub fn active_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_index]
    }

    pub fn create_workspace(&mut self) -> Result<(), xmux_core::XmuxError> {
        let n = self.workspaces.len() + 1;
        let ws = Workspace::new(format!("Workspace {n}"))?;
        self.workspaces.push(ws);
        self.active_index = self.workspaces.len() - 1;
        Ok(())
    }

    pub fn close_workspace(&mut self, index: usize) {
        if self.workspaces.len() <= 1 {
            return;
        }
        let ws = self.workspaces.remove(index);
        for (_, pane) in ws.panes.iter() {
            pane.terminal.shutdown();
        }
        if self.active_index >= self.workspaces.len() {
            self.active_index = self.workspaces.len() - 1;
        }
    }

    pub fn next_workspace(&mut self) {
        if !self.workspaces.is_empty() {
            self.active_index = (self.active_index + 1) % self.workspaces.len();
        }
    }

    pub fn prev_workspace(&mut self) {
        if !self.workspaces.is_empty() {
            self.active_index = if self.active_index == 0 {
                self.workspaces.len() - 1
            } else {
                self.active_index - 1
            };
        }
    }
}
