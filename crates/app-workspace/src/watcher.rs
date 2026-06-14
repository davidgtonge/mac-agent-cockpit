use app_core::ProjectId;
use crossbeam_channel::Sender;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::{path_has_skipped_segment, WorkspaceError};

#[derive(Debug, Clone)]
pub struct DirtySignal {
    pub project_id: ProjectId,
    pub rel_path: String,
}

#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("workspace io: {0}")]
    Io(#[from] std::io::Error),
}

pub struct WorkspaceWatcher {
    root: PathBuf,
    _watcher: RecommendedWatcher,
}

impl WorkspaceWatcher {
    pub fn watch(
        root: PathBuf,
        project_id: ProjectId,
        tx: Sender<DirtySignal>,
    ) -> Result<Self, WatcherError> {
        let canonical = root.canonicalize()?;
        let root_for_cb = canonical.clone();
        let project_for_cb = project_id.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    forward_event(&root_for_cb, &project_for_cb, &tx, event);
                }
            },
            Config::default(),
        )?;

        watcher.watch(&canonical, RecursiveMode::Recursive)?;

        Ok(Self {
            root: canonical,
            _watcher: watcher,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn forward_event(
    root: &Path,
    project_id: &ProjectId,
    tx: &Sender<DirtySignal>,
    event: Event,
) {
    if matches!(event.kind, EventKind::Access(_)) {
        return;
    }

    for path in event.paths {
        let Some(rel) = relative_path(root, &path) else {
            continue;
        };
        if should_ignore_watcher_path(&rel) {
            continue;
        }
        let _ = tx.send(DirtySignal {
            project_id: project_id.clone(),
            rel_path: rel,
        });
    }
}

fn relative_path(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let rel = rel.to_string_lossy().replace('\\', "/");
    if rel.is_empty() {
        return None;
    }
    Some(rel)
}

pub fn should_ignore_watcher_path(rel_path: &str) -> bool {
    let rel_path = rel_path.trim();
    if rel_path.is_empty() {
        return true;
    }
    if path_has_skipped_segment(rel_path) {
        return true;
    }
    let name = rel_path.rsplit('/').next().unwrap_or(rel_path);
    if name.starts_with('.') && name != ".cursor" {
        return true;
    }
    matches!(
        name,
        ".DS_Store" | "Thumbs.db" | ".swp" | ".swo" | ".tmp" | "Cargo.lock"
    ) || name.ends_with('~')
        || name.ends_with(".tmp")
        || name.ends_with(".swp")
}

impl From<WatcherError> for WorkspaceError {
    fn from(value: WatcherError) -> Self {
        match value {
            WatcherError::Io(err) => WorkspaceError::Io(err),
            WatcherError::Notify(err) => WorkspaceError::Git(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_git_and_build_artifacts() {
        assert!(should_ignore_watcher_path(".git/index"));
        assert!(should_ignore_watcher_path("node_modules/pkg/index.js"));
        assert!(should_ignore_watcher_path("target/debug/app"));
        assert!(!should_ignore_watcher_path("src/main.rs"));
    }
}
