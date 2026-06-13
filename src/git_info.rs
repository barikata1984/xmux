use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitInfo {
    pub branch: Option<String>,
    pub is_dirty: bool,
}

impl GitInfo {
    /// Get git info for a directory (synchronous). Returns None if not a git repo.
    pub fn from_dir(dir: &Path) -> Option<Self> {
        let branch = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(dir)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
                } else {
                    None
                }
            });

        // If we can't get the branch, it's not a git repo
        let branch = branch?;

        let is_dirty = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(dir)
            .output()
            .ok()
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false);

        Some(Self { branch: Some(branch), is_dirty })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_dir_current() {
        let git_info = GitInfo::from_dir(Path::new("."));
        assert!(git_info.is_some(), "Current directory should be a git repo");
        if let Some(info) = git_info {
            assert!(info.branch.is_some(), "Branch should be Some for current git repo");
        }
    }

    #[test]
    fn test_from_dir_non_git() {
        let git_info = GitInfo::from_dir(Path::new("/tmp"));
        assert!(git_info.is_none(), "/tmp should not be a git repo");
    }
}
