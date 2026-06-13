pub mod autosave;
pub mod snapshot;
pub mod restore;

pub use autosave::AutoSaver;
pub use snapshot::*;
pub use restore::{RestoreResult, RestoredWorkspace, RestoredPane};
