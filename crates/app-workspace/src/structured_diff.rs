use app_core::{
    DiffHunkVm, DiffRowKind, DiffRowVm, FilePreview, GitFileStatus, ProjectId, StructuredDiffVm,
};
use similar::{ChangeTag, TextDiff};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use crate::git_overlay::load_file_at_revision;
use crate::{
    highlighted_lines_for_line_numbers, highlighted_lines_for_text,
    language_for_path, looks_binary, WorkspaceError, WorkspaceManager, MAX_HIGHLIGHT_BYTES,
};

type PendingChange = (ChangeTag, Option<u32>, Option<u32>);

pub fn compute_structured_diff(
    workspace: &WorkspaceManager,
    _project_id: ProjectId,
    root: &Path,
    base_revision: &str,
    path: &str,
    old_path: Option<&str>,
    status: GitFileStatus,
) -> Result<StructuredDiffVm, WorkspaceError> {
    let now_path = workspace.safe_join(root, path)?;
    let now_bytes = if status == GitFileStatus::Deleted {
        None
    } else if now_path.exists() && now_path.is_file() {
        Some(std::fs::read(&now_path)?)
    } else {
        None
    };

    let prev_text = if matches!(status, GitFileStatus::Added | GitFileStatus::Untracked) {
        None
    } else {
        load_file_at_revision(root, base_revision, path, old_path)?
    };

    let now_text = match now_bytes {
        Some(bytes) if looks_binary(&bytes) => {
            return Ok(binary_diff_vm(path, old_path, status));
        }
        Some(bytes) => Some(String::from_utf8_lossy(&bytes).to_string()),
        None => None,
    };

    let old_label = old_path.unwrap_or(path);
    let path_ref = Path::new(path);
    let language_hint = language_for_path(path_ref);
    let stat = diff_stat(prev_text.as_deref(), now_text.as_deref());

    if status == GitFileStatus::Added {
        return Ok(added_diff_vm(
            path,
            now_text.as_deref().unwrap_or_default(),
            &language_hint,
            stat,
        ));
    }
    if status == GitFileStatus::Deleted {
        return Ok(removed_diff_vm(
            old_label,
            prev_text.as_deref().unwrap_or_default(),
            &language_hint,
            stat,
        ));
    }

    let old_content = prev_text.unwrap_or_default();
    let new_content = now_text.unwrap_or_default();
    let diff = TextDiff::from_lines(&old_content, &new_content);

    let mut old_lines_needed = BTreeSet::new();
    let mut new_lines_needed = BTreeSet::new();
    let mut parsed_hunks: Vec<Vec<PendingChange>> = Vec::new();
    let mut old_line = 1_u32;
    let mut new_line = 1_u32;

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        let mut changes = Vec::new();
        for change in hunk.iter_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    old_lines_needed.insert(old_line);
                    new_lines_needed.insert(new_line);
                    changes.push((ChangeTag::Equal, Some(old_line), Some(new_line)));
                    old_line += 1;
                    new_line += 1;
                }
                ChangeTag::Delete => {
                    old_lines_needed.insert(old_line);
                    changes.push((ChangeTag::Delete, Some(old_line), None));
                    old_line += 1;
                }
                ChangeTag::Insert => {
                    new_lines_needed.insert(new_line);
                    changes.push((ChangeTag::Insert, None, Some(new_line)));
                    new_line += 1;
                }
            }
        }
        parsed_hunks.push(changes);
    }

    let new_highlights = highlighted_lines_for_line_numbers(
        &new_content,
        path_ref,
        language_hint.as_deref(),
        &new_lines_needed,
    );
    let old_highlights = highlighted_lines_for_line_numbers(
        &old_content,
        path_ref,
        language_hint.as_deref(),
        &old_lines_needed,
    );

    let mut hunks = Vec::new();
    for changes in parsed_hunks {
        let mut rows = vec![DiffRowVm {
            kind: DiffRowKind::HunkHeader,
            old_line: None,
            new_line: None,
            highlighted_html: "@@".into(),
        }];
        for (tag, old_num, new_num) in changes {
            rows.push(match tag {
                ChangeTag::Equal => DiffRowVm {
                    kind: DiffRowKind::Context,
                    old_line: old_num,
                    new_line: new_num,
                    highlighted_html: map_highlight(&new_highlights, new_num),
                },
                ChangeTag::Delete => DiffRowVm {
                    kind: DiffRowKind::Removed,
                    old_line: old_num,
                    new_line: None,
                    highlighted_html: map_highlight(&old_highlights, old_num),
                },
                ChangeTag::Insert => DiffRowVm {
                    kind: DiffRowKind::Added,
                    old_line: None,
                    new_line: new_num,
                    highlighted_html: map_highlight(&new_highlights, new_num),
                },
            });
        }
        hunks.push(DiffHunkVm {
            header: String::new(),
            rows,
        });
    }

    if hunks.is_empty() && old_content == new_content {
        hunks.push(DiffHunkVm {
            header: String::new(),
            rows: vec![DiffRowVm {
                kind: DiffRowKind::Notice,
                old_line: None,
                new_line: None,
                highlighted_html: "No changes".into(),
            }],
        });
    }

    Ok(StructuredDiffVm {
        old_path: Some(old_label.to_string()),
        new_path: Some(path.to_string()),
        status,
        stat,
        hunks,
    })
}

pub fn load_prev_preview(
    _workspace: &WorkspaceManager,
    project_id: ProjectId,
    root: &Path,
    base_revision: &str,
    path: &str,
    old_path: Option<&str>,
) -> Result<FilePreview, WorkspaceError> {
    let content = load_file_at_revision(root, base_revision, path, old_path)?;
    let path_ref = Path::new(path);
    let language_hint = language_for_path(path_ref);
    let text = content.clone();
    let truncated = text
        .as_ref()
        .map(|t| t.len() > MAX_HIGHLIGHT_BYTES)
        .unwrap_or(false);
    let size_bytes = text.as_ref().map(|t| t.len() as u64).unwrap_or(0);
    Ok(FilePreview {
        project_id,
        path: path.into(),
        text,
        highlighted_lines: None,
        binary: false,
        truncated,
        size_bytes,
        language_hint,
    })
}

fn added_diff_vm(
    path: &str,
    content: &str,
    language_hint: &Option<String>,
    stat: String,
) -> StructuredDiffVm {
    let path_ref = Path::new(path);
    let highlights = highlighted_lines_for_text(content, path_ref, language_hint.as_deref());
    let rows = highlights
        .into_iter()
        .enumerate()
        .map(|(idx, highlighted_html)| DiffRowVm {
            kind: DiffRowKind::Added,
            old_line: None,
            new_line: Some((idx + 1) as u32),
            highlighted_html,
        })
        .collect();
    StructuredDiffVm {
        old_path: None,
        new_path: Some(path.to_string()),
        status: GitFileStatus::Added,
        stat,
        hunks: vec![DiffHunkVm {
            header: String::new(),
            rows,
        }],
    }
}

fn removed_diff_vm(
    path: &str,
    content: &str,
    language_hint: &Option<String>,
    stat: String,
) -> StructuredDiffVm {
    let path_ref = Path::new(path);
    let highlights = highlighted_lines_for_text(content, path_ref, language_hint.as_deref());
    let rows = highlights
        .into_iter()
        .enumerate()
        .map(|(idx, highlighted_html)| DiffRowVm {
            kind: DiffRowKind::Removed,
            old_line: Some((idx + 1) as u32),
            new_line: None,
            highlighted_html,
        })
        .collect();
    StructuredDiffVm {
        old_path: Some(path.to_string()),
        new_path: None,
        status: GitFileStatus::Deleted,
        stat,
        hunks: vec![DiffHunkVm {
            header: String::new(),
            rows,
        }],
    }
}

fn binary_diff_vm(path: &str, old_path: Option<&str>, _status: GitFileStatus) -> StructuredDiffVm {
    StructuredDiffVm {
        old_path: old_path.map(str::to_string),
        new_path: Some(path.to_string()),
        status: GitFileStatus::Binary,
        stat: "binary".into(),
        hunks: vec![DiffHunkVm {
            header: String::new(),
            rows: vec![DiffRowVm {
                kind: DiffRowKind::Notice,
                old_line: None,
                new_line: None,
                highlighted_html: "Binary file — preview unavailable".into(),
            }],
        }],
    }
}

fn map_highlight(lines: &HashMap<u32, String>, line_number: Option<u32>) -> String {
    line_number
        .and_then(|line| lines.get(&line).cloned())
        .unwrap_or_default()
}

fn diff_stat(old: Option<&str>, new: Option<&str>) -> String {
    let additions = new.map(count_lines).unwrap_or(0);
    let deletions = old.map(count_lines).unwrap_or(0);
    format!("+{additions} -{deletions}")
}

fn count_lines(text: &str) -> u32 {
    if text.is_empty() {
        0
    } else {
        text.lines().count() as u32
    }
}
