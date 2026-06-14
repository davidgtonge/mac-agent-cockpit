use app_core::{ChangedFile, FileNode, GitFileStatus, GitOverlayState, GitStatusEntry, now_ms};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use crate::WorkspaceError;

pub fn capture_head_revision(root: &Path) -> Result<(String, Option<String>), WorkspaceError> {
    let revision = git(root, &["rev-parse", "HEAD"])?.trim().to_string();
    let branch = git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty() && b != "HEAD");
    Ok((revision, branch))
}

pub fn build_git_overlay(root: &Path, base_revision: &str) -> Result<GitOverlayState, WorkspaceError> {
    let mut entries = BTreeMap::new();
    let mut synthetic_nodes = Vec::new();

    let diff_output = git(root, &["diff", "--name-status", "-z", base_revision])?;
    for record in parse_name_status_z(&diff_output) {
        let entry = GitStatusEntry {
            status: record.status,
            old_path: record.old_path.clone(),
            additions: None,
            deletions: None,
        };
        entries.insert(record.path.clone(), entry);

        if record.status == GitFileStatus::Deleted {
            synthetic_nodes.push(synthetic_file_node(&record.path, GitFileStatus::Deleted));
        } else if record.status == GitFileStatus::Renamed {
            if let Some(old_path) = &record.old_path {
                synthetic_nodes.push(synthetic_file_node(old_path, GitFileStatus::Deleted));
            }
        }
    }

    let untracked = git(root, &["ls-files", "--others", "--exclude-standard"])?;
    for line in untracked.lines() {
        let path = line.trim();
        if path.is_empty() {
            continue;
        }
        entries.entry(path.to_string()).or_insert(GitStatusEntry {
            status: GitFileStatus::Untracked,
            old_path: None,
            additions: None,
            deletions: None,
        });
    }

    enrich_numstat(root, base_revision, &mut entries);

    let changed_files = entries
        .iter()
        .map(|(path, entry)| ChangedFile {
            path: path.clone(),
            status: entry.status,
            old_path: entry.old_path.clone(),
            additions: entry.additions,
            deletions: entry.deletions,
        })
        .collect::<Vec<_>>();

    Ok(GitOverlayState {
        entries,
        synthetic_nodes,
        changed_files,
        base_revision: base_revision.to_string(),
        refreshed_at_ms: now_ms(),
    })
}

pub fn load_file_at_revision(
    root: &Path,
    base_revision: &str,
    relative: &str,
    old_path: Option<&str>,
) -> Result<Option<String>, WorkspaceError> {
    let candidates = [Some(relative), old_path]
        .into_iter()
        .flatten()
        .map(|p| p.to_string())
        .collect::<Vec<_>>();
    for candidate in candidates {
        let spec = format!("{base_revision}:{candidate}");
        match git(root, &["show", &spec]) {
            Ok(text) => return Ok(Some(text)),
            Err(WorkspaceError::Git(_)) => continue,
            Err(err) => return Err(err),
        }
    }
    Ok(None)
}

fn enrich_numstat(root: &Path, base_revision: &str, entries: &mut BTreeMap<String, GitStatusEntry>) {
    for (path, entry) in entries.iter_mut() {
        if !matches!(
            entry.status,
            GitFileStatus::Modified | GitFileStatus::Added | GitFileStatus::Renamed
        ) {
            continue;
        }
        let args = ["diff", "--numstat", base_revision, "--", path];
        if let Ok(output) = git(root, &args) {
            if let Some((additions, deletions)) = parse_numstat_line(output.lines().next()) {
                entry.additions = Some(additions);
                entry.deletions = Some(deletions);
            }
        }
    }
}

struct NameStatusRecord {
    path: String,
    status: GitFileStatus,
    old_path: Option<String>,
}

fn parse_name_status_z(output: &str) -> Vec<NameStatusRecord> {
    let parts: Vec<&str> = output.split('\0').filter(|p| !p.is_empty()).collect();
    let mut records = Vec::new();
    let mut idx = 0_usize;
    while idx < parts.len() {
        let status_token = parts[idx];
        idx += 1;
        let code = status_token.chars().next().unwrap_or('M');
        let status = map_status_code(code);
        if status == GitFileStatus::Renamed || status == GitFileStatus::Copied {
            if idx + 1 >= parts.len() {
                break;
            }
            let old_path = parts[idx].to_string();
            let new_path = parts[idx + 1].to_string();
            idx += 2;
            records.push(NameStatusRecord {
                path: new_path,
                status,
                old_path: Some(old_path),
            });
            continue;
        }
        if idx >= parts.len() {
            break;
        }
        let path = parts[idx].to_string();
        idx += 1;
        if path.is_empty() {
            continue;
        }
        records.push(NameStatusRecord {
            path,
            status,
            old_path: None,
        });
    }
    records
}

fn map_status_code(code: char) -> GitFileStatus {
    match code {
        'M' => GitFileStatus::Modified,
        'A' => GitFileStatus::Added,
        'D' => GitFileStatus::Deleted,
        'R' => GitFileStatus::Renamed,
        'C' => GitFileStatus::Copied,
        'T' => GitFileStatus::TypeChanged,
        'U' => GitFileStatus::Conflicted,
        'B' => GitFileStatus::Binary,
        _ => GitFileStatus::Modified,
    }
}

fn parse_numstat_line(line: Option<&str>) -> Option<(u32, u32)> {
    let line = line?;
    let mut cols = line.split('\t');
    let add = cols.next()?.trim();
    let del = cols.next()?.trim();
    let additions = if add == "-" {
        0
    } else {
        add.parse().ok()?
    };
    let deletions = if del == "-" {
        0
    } else {
        del.parse().ok()?
    };
    Some((additions, deletions))
}

fn synthetic_file_node(path: &str, status: GitFileStatus) -> FileNode {
    let name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    FileNode {
        path: path.replace('\\', "/"),
        name,
        is_dir: false,
        size_bytes: None,
        modified_at_ms: None,
        ignored: false,
        git_status: Some(status),
        change_count: None,
        synthetic: true,
    }
}

pub(crate) fn git(root: &Path, args: &[&str]) -> Result<String, WorkspaceError> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(WorkspaceError::Git(stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
