mod git_overlay;
mod structured_diff;
pub mod watcher;

use app_core::{
    now_ms, ChangedFile, DiffResult, FileNode, FilePreview, FilenameIndexEntry, GitFileStatus,
    GitOverlayState, ProjectId, SessionBaseRevision, StructuredDiffVm, WorkspaceSearchHit,
    WorkspaceSearchMode,
};
pub use watcher::{DirtySignal, WorkspaceWatcher};
use git_overlay::{build_git_overlay, capture_head_revision};
use ignore::WalkBuilder;
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme, ThemeSet},
    html::{styled_line_to_highlighted_html, IncludeBackground},
    parsing::{SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    io::{BufRead, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex, OnceLock},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("workspace io: {0}")]
    Io(#[from] std::io::Error),
    #[error("walk error: {0}")]
    Walk(#[from] ignore::Error),
    #[error("path escapes project root")]
    PathEscape,
    #[error("git failed: {0}")]
    Git(String),
}

#[derive(Debug, Clone)]
struct CachedPreview {
    modified_at_ms: i64,
    size_bytes: u64,
    preview: FilePreview,
}

#[derive(Debug, Clone)]
pub struct WorkspaceManager {
    max_preview_bytes: u64,
    canonical_roots: Arc<Mutex<HashMap<PathBuf, PathBuf>>>,
    preview_cache: Arc<Mutex<HashMap<(PathBuf, String), CachedPreview>>>,
}

const MAX_PREVIEW_CACHE_ENTRIES: usize = 64;

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self {
            max_preview_bytes: 256 * 1024,
            canonical_roots: Arc::new(Mutex::new(HashMap::new())),
            preview_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl WorkspaceManager {
    pub fn new() -> Self {
        let manager = Self::default();
        warmup_syntax_highlighting();
        manager
    }

    pub fn load_directory(
        &self,
        root: &Path,
        relative: &str,
    ) -> Result<Vec<FileNode>, WorkspaceError> {
        let dir = self.safe_join(root, relative)?;
        let mut nodes = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if should_skip(&name) {
                continue;
            }
            let metadata = entry.metadata()?;
            let rel = path
                .strip_prefix(self.canonical_root(root)?)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            nodes.push(FileNode {
                path: rel,
                name,
                is_dir: metadata.is_dir(),
                size_bytes: if metadata.is_file() {
                    Some(metadata.len())
                } else {
                    None
                },
                modified_at_ms: metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64),
                ignored: false,
                git_status: None,
                change_count: None,
                synthetic: false,
            });
        }
        nodes.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(nodes)
    }

    pub fn load_file_preview(
        &self,
        project_id: ProjectId,
        root: &Path,
        relative: &str,
    ) -> Result<FilePreview, WorkspaceError> {
        let normalized = normalize_preview_path(relative);
        let canonical_root = self.canonical_root(root)?;
        let cache_key = (canonical_root.clone(), normalized.clone());
        let joined = self.join_under_root(&canonical_root, &normalized)?;
        let metadata = fs::metadata(&joined)?;
        let modified_at_ms = file_modified_at_ms(&metadata);
        let size_bytes = metadata.len();
        if metadata.is_file() {
            if let Some(cached) = self.preview_cache.lock().ok().and_then(|cache| {
                cache.get(&cache_key).and_then(|entry| {
                    if entry.modified_at_ms == modified_at_ms && entry.size_bytes == size_bytes {
                        Some(entry.preview.clone())
                    } else {
                        None
                    }
                })
            }) {
                return Ok(FilePreview {
                    project_id,
                    ..cached
                });
            }
        }
        if !metadata.is_file() {
            return Ok(FilePreview {
                project_id,
                path: normalized,
                text: None,
                highlighted_lines: None,
                binary: false,
                truncated: false,
                size_bytes,
                language_hint: None,
            });
        }
        let mut file = fs::File::open(&joined)?;
        let mut buf = Vec::new();
        file.by_ref()
            .take(self.max_preview_bytes + 1)
            .read_to_end(&mut buf)?;
        let truncated = buf.len() as u64 > self.max_preview_bytes;
        if truncated {
            buf.truncate(self.max_preview_bytes as usize);
        }
        let is_binary = looks_binary(&buf);
        let text = if is_binary {
            None
        } else {
            Some(String::from_utf8_lossy(&buf).to_string())
        };
        let language_hint = language_for_path(&joined);
        let preview = FilePreview {
            project_id: project_id.clone(),
            path: normalized.clone(),
            text,
            highlighted_lines: None,
            binary: is_binary,
            truncated,
            size_bytes,
            language_hint,
        };
        if let Ok(mut cache) = self.preview_cache.lock() {
            if cache.len() >= MAX_PREVIEW_CACHE_ENTRIES {
                if let Some(oldest) = cache.keys().next().cloned() {
                    cache.remove(&oldest);
                }
            }
            cache.insert(
                cache_key,
                CachedPreview {
                    modified_at_ms,
                    size_bytes,
                    preview: FilePreview {
                        project_id,
                        ..preview.clone()
                    },
                },
            );
        }
        Ok(preview)
    }

    pub fn changed_files(
        &self,
        _project_id: ProjectId,
        root: &Path,
    ) -> Result<Vec<ChangedFile>, WorkspaceError> {
        let (revision, _) = capture_head_revision(root)?;
        Ok(build_git_overlay(root, &revision)?.changed_files)
    }

    pub fn refresh_git_overlay(
        &self,
        root: &Path,
        base_revision: &str,
    ) -> Result<GitOverlayState, WorkspaceError> {
        let revision = if base_revision == "HEAD" {
            capture_head_revision(root)?.0
        } else {
            base_revision.to_string()
        };
        build_git_overlay(root, &revision)
    }

    pub fn capture_session_base(
        &self,
        conversation_id: app_core::ConversationId,
        project_id: ProjectId,
        root: &Path,
    ) -> Result<SessionBaseRevision, WorkspaceError> {
        let (revision, branch) = capture_head_revision(root)?;
        Ok(SessionBaseRevision {
            conversation_id,
            project_id,
            revision,
            branch,
            captured_at_ms: now_ms(),
        })
    }

    pub fn load_prev_file_preview(
        &self,
        project_id: ProjectId,
        root: &Path,
        base_revision: &str,
        path: &str,
        old_path: Option<&str>,
    ) -> Result<FilePreview, WorkspaceError> {
        let revision = if base_revision == "HEAD" {
            capture_head_revision(root)?.0
        } else {
            base_revision.to_string()
        };
        structured_diff::load_prev_preview(self, project_id, root, &revision, path, old_path)
    }

    pub fn compute_structured_diff(
        &self,
        project_id: ProjectId,
        root: &Path,
        base_revision: &str,
        path: &str,
        old_path: Option<&str>,
        status: GitFileStatus,
    ) -> Result<StructuredDiffVm, WorkspaceError> {
        let revision = if base_revision == "HEAD" {
            capture_head_revision(root)?.0
        } else {
            base_revision.to_string()
        };
        structured_diff::compute_structured_diff(
            self, project_id, root, &revision, path, old_path, status,
        )
    }

    pub fn compute_diff(
        &self,
        project_id: ProjectId,
        root: &Path,
        path: Option<&str>,
    ) -> Result<DiffResult, WorkspaceError> {
        let mut args = vec!["diff", "--"];
        let path_string;
        if let Some(p) = path {
            path_string = p.to_string();
            args.push(&path_string);
        }
        let text = git(root, &args)?;
        let mut additions = 0_u32;
        let mut deletions = 0_u32;
        for line in text.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                additions += 1;
            }
            if line.starts_with('-') && !line.starts_with("---") {
                deletions += 1;
            }
        }
        let stat = format!("+{} -{}", additions, deletions);
        Ok(DiffResult {
            project_id,
            path: path.map(|p| p.to_string()),
            stat,
            text,
            generated_at_ms: now_ms(),
        })
    }

    fn canonical_root(&self, root: &Path) -> Result<PathBuf, WorkspaceError> {
        let key = root.to_path_buf();
        if let Some(cached) = self
            .canonical_roots
            .lock()
            .ok()
            .and_then(|cache| cache.get(&key).cloned())
        {
            return Ok(cached);
        }
        let canonical = root.canonicalize()?;
        if let Ok(mut cache) = self.canonical_roots.lock() {
            cache.insert(key, canonical.clone());
        }
        Ok(canonical)
    }

    pub fn build_filename_index(&self, root: &Path) -> Result<Vec<FilenameIndexEntry>, WorkspaceError> {
        let root = self.canonical_root(root)?;
        let mut entries = Vec::new();
        let walker = WalkBuilder::new(&root)
            .hidden(false)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(true)
            .filter_entry(|entry| !should_skip(entry.file_name().to_string_lossy().as_ref()))
            .build();
        for result in walker {
            let entry = result?;
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(&root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            if path_has_skipped_segment(&rel) {
                continue;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let modified_at_ms = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64);
            entries.push(FilenameIndexEntry {
                path: rel,
                name,
                modified_at_ms,
            });
        }
        Ok(entries)
    }

    pub fn search_filenames(
        index: &[FilenameIndexEntry],
        query: &str,
        limit: usize,
    ) -> Vec<WorkspaceSearchHit> {
        let q = query.to_lowercase();
        let mut hits: Vec<(i32, WorkspaceSearchHit)> = index
            .iter()
            .filter(|entry| !path_has_skipped_segment(&entry.path))
            .filter_map(|entry| {
                let name_lower = entry.name.to_lowercase();
                let path_lower = entry.path.to_lowercase();
                let score = if name_lower == q {
                    100
                } else if name_lower.starts_with(&q) {
                    80
                } else if name_lower.contains(&q) {
                    60
                } else if path_lower.contains(&q) {
                    40
                } else {
                    return None;
                };
                Some((
                    score,
                    WorkspaceSearchHit {
                        path: entry.path.clone(),
                        line: None,
                        column: None,
                        snippet: entry.name.clone(),
                        kind: "filename".into(),
                    },
                ))
            })
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0));
        hits.into_iter()
            .take(limit)
            .map(|(_, hit)| hit)
            .collect()
    }

    pub fn search_content(
        &self,
        root: &Path,
        query: &str,
        limit: usize,
    ) -> Result<Vec<WorkspaceSearchHit>, WorkspaceError> {
        let root = self.canonical_root(root)?;
        let mut rg = Command::new("rg");
        rg.args([
            "--json",
            "-n",
            "--max-count",
            &limit.to_string(),
            "--ignore-case",
        ]);
        for segment in SKIPPED_PATH_SEGMENTS {
            rg.args(["--glob", &format!("!{segment}/**")]);
        }
        rg.arg(query).current_dir(&root);
        let output = rg
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        let output = match output {
            Ok(o) => o,
            Err(_) => return Ok(Vec::new()),
        };
        if !output.status.success() && output.stdout.is_empty() {
            return Ok(Vec::new());
        }
        let mut hits = Vec::new();
        for line in output.stdout.lines() {
            let line = line.map_err(WorkspaceError::Io)?;
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_str(&line).unwrap_or(serde_json::Value::Null);
            if value.get("type").and_then(|v| v.as_str()) != Some("match") {
                continue;
            }
            let data = value.get("data").and_then(|v| v.get("path")).and_then(|p| {
                let path_text = p.get("text")?.as_str()?.to_string();
                let line_number = value
                    .get("data")
                    .and_then(|d| d.get("line_number"))
                    .and_then(|n| n.as_u64())? as u32;
                let lines = value
                    .get("data")
                    .and_then(|d| d.get("lines"))
                    .and_then(|l| l.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                Some(WorkspaceSearchHit {
                    path: path_text,
                    line: Some(line_number),
                    column: None,
                    snippet: lines,
                    kind: "content".into(),
                })
            });
            if let Some(hit) = data {
                if !path_has_skipped_segment(&hit.path) {
                    hits.push(hit);
                }
            }
            if hits.len() >= limit {
                break;
            }
        }
        Ok(hits)
    }

    pub fn search_workspace(
        &self,
        root: &Path,
        index: Option<&[FilenameIndexEntry]>,
        query: &str,
        mode: &WorkspaceSearchMode,
        limit: usize,
    ) -> Result<Vec<WorkspaceSearchHit>, WorkspaceError> {
        let mut hits = Vec::new();
        if matches!(mode, WorkspaceSearchMode::Both | WorkspaceSearchMode::Filename) {
            if let Some(index) = index {
                hits.extend(Self::search_filenames(index, query, limit));
            }
        }
        if matches!(mode, WorkspaceSearchMode::Both | WorkspaceSearchMode::Content) {
            let remaining = limit.saturating_sub(hits.len());
            if remaining > 0 {
                hits.extend(self.search_content(root, query, remaining)?);
            }
        }
        Ok(hits)
    }

    pub(crate) fn safe_join(&self, root: &Path, relative: &str) -> Result<PathBuf, WorkspaceError> {
        let root = self.canonical_root(root)?;
        self.join_under_root(&root, &normalize_preview_path(relative))
    }

    fn join_under_root(&self, root: &Path, relative: &str) -> Result<PathBuf, WorkspaceError> {
        if relative.contains("..") {
            return Err(WorkspaceError::PathEscape);
        }
        let candidate = if relative.is_empty() || relative == "." {
            root.to_path_buf()
        } else {
            root.join(relative)
        };
        if !candidate.starts_with(root) {
            return Err(WorkspaceError::PathEscape);
        }
        Ok(candidate)
    }
}

const SKIPPED_PATH_SEGMENTS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".next",
    "dist",
    "build",
];
pub(crate) const MAX_HIGHLIGHT_BYTES: usize = 128 * 1024;
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static SYNTAX_THEME: OnceLock<Theme> = OnceLock::new();

fn should_skip(name: &str) -> bool {
    SKIPPED_PATH_SEGMENTS.contains(&name)
}

pub fn path_has_skipped_segment(path: &str) -> bool {
    path.split('/').any(|segment| should_skip(segment))
}

pub(crate) fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(4096).any(|b| *b == 0)
}

pub(crate) fn highlight_lines_for_preview(
    text: &str,
    path: &Path,
    language_hint: Option<&str>,
) -> Option<Vec<String>> {
    if text.len() > MAX_HIGHLIGHT_BYTES {
        return None;
    }

    let syntax_set = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let theme = SYNTAX_THEME.get_or_init(load_syntax_theme);
    let syntax = resolve_syntax(syntax_set, path, language_hint);
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut highlighted_lines = Vec::new();
    let mut consumed = 0_usize;
    for line_with_end in LinesWithEndings::from(text) {
        let plain_line = line_with_end.trim_end_matches('\n');
        let line_html = match highlighter
            .highlight_line(line_with_end, syntax_set)
            .ok()
            .and_then(|ranges| styled_line_to_highlighted_html(&ranges, IncludeBackground::No).ok())
        {
            Some(html) => html,
            None => escape_html(line_with_end),
        };
        highlighted_lines.push(line_html.trim_end_matches('\n').to_string());
        consumed += line_with_end.len();
        if plain_line.is_empty() && line_with_end.ends_with('\n') && consumed == text.len() {
            highlighted_lines.push(String::new());
        }
    }

    if highlighted_lines.is_empty() {
        highlighted_lines.push(String::new());
    }
    Some(highlighted_lines)
}

pub(crate) fn highlighted_lines_for_text(
    text: &str,
    path: &Path,
    language_hint: Option<&str>,
) -> Vec<String> {
    if let Some(lines) = highlight_lines_for_preview(text, path, language_hint) {
        return lines;
    }
    plain_highlighted_lines(text)
}

pub fn warmup_syntax_highlighting() {
    let syntax_set = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let _ = SYNTAX_THEME.get_or_init(load_syntax_theme);
    let _ = resolve_syntax(syntax_set, Path::new("warmup.ts"), Some("typescript"));
}

pub(crate) fn highlighted_lines_for_line_numbers(
    text: &str,
    path: &Path,
    language_hint: Option<&str>,
    line_numbers: &BTreeSet<u32>,
) -> HashMap<u32, String> {
    if text.is_empty() || line_numbers.is_empty() {
        return HashMap::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut out = HashMap::with_capacity(line_numbers.len());
    for (start, end) in contiguous_line_ranges(line_numbers) {
        let start_idx = start.saturating_sub(1) as usize;
        let end_idx = end as usize;
        if start_idx >= lines.len() {
            continue;
        }
        let end_idx = end_idx.min(lines.len());
        let slice = lines[start_idx..end_idx].join("\n");
        let highlighted = highlighted_lines_for_text(&slice, path, language_hint);
        for (offset, html) in highlighted.into_iter().enumerate() {
            out.insert(start + offset as u32, html);
        }
    }
    out
}

fn file_modified_at_ms(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn normalize_preview_path(path: &str) -> String {
    let cleaned = path
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string();
    if cleaned.is_empty() {
        ".".into()
    } else {
        cleaned
    }
}

fn plain_highlighted_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    LinesWithEndings::from(text)
        .map(|line| escape_html(line.trim_end_matches('\n')))
        .collect()
}

fn contiguous_line_ranges(line_numbers: &BTreeSet<u32>) -> Vec<(u32, u32)> {
    let mut ranges = Vec::new();
    let mut iter = line_numbers.iter().copied();
    let Some(mut start) = iter.next() else {
        return ranges;
    };
    let mut end = start;
    for line in iter {
        if line == end + 1 {
            end = line;
        } else {
            ranges.push((start, end));
            start = line;
            end = line;
        }
    }
    ranges.push((start, end));
    ranges
}

fn load_syntax_theme() -> Theme {
    let theme_set = ThemeSet::load_defaults();
    theme_set
        .themes
        .get("base16-ocean.dark")
        .cloned()
        .or_else(|| theme_set.themes.values().next().cloned())
        .unwrap_or_default()
}

pub(crate) fn resolve_syntax<'a>(
    syntax_set: &'a SyntaxSet,
    path: &Path,
    language_hint: Option<&str>,
) -> &'a SyntaxReference {
    if let Ok(Some(syntax)) = syntax_set.find_syntax_for_file(path) {
        return syntax;
    }

    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        let ext = ext.to_ascii_lowercase();
        if let Some(syntax) = syntax_set.find_syntax_by_extension(&ext) {
            return syntax;
        }
        if ext == "ts" || ext == "tsx" {
            if let Some(syntax) = syntax_set.find_syntax_by_extension("ts") {
                return syntax;
            }
            if let Some(syntax) = syntax_set.find_syntax_by_name("TypeScriptReact") {
                return syntax;
            }
            if let Some(syntax) = syntax_set.find_syntax_by_name("TypeScript (JSX)") {
                return syntax;
            }
            if let Some(syntax) = syntax_set.find_syntax_by_extension("js") {
                return syntax;
            }
            if let Some(syntax) = syntax_set.find_syntax_by_name("JavaScript") {
                return syntax;
            }
        }
    }

    if let Some(hint) = language_hint {
        let hint = hint.to_ascii_lowercase();
        if let Some(syntax) = syntax_set.find_syntax_by_token(&hint) {
            return syntax;
        }
        let alias = match hint.as_str() {
            "typescript" => Some("TypeScript"),
            "javascript" => Some("JavaScript"),
            "rust" => Some("Rust"),
            "json" => Some("JSON"),
            "yaml" => Some("YAML"),
            "toml" => Some("TOML"),
            "css" => Some("CSS"),
            "html" => Some("HTML"),
            "python" => Some("Python"),
            _ => None,
        };
        if let Some(name) = alias {
            if let Some(syntax) = syntax_set.find_syntax_by_name(name) {
                return syntax;
            }
        }
        if hint == "typescript" {
            if let Some(syntax) = syntax_set.find_syntax_by_extension("js") {
                return syntax;
            }
            if let Some(syntax) = syntax_set.find_syntax_by_name("JavaScript") {
                return syntax;
            }
        }
    }

    syntax_set.find_syntax_plain_text()
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(crate) fn language_for_path(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let lang = match ext.as_str() {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "json" => "json",
        "md" => "markdown",
        "toml" => "toml",
        "yml" | "yaml" => "yaml",
        "sql" => "sql",
        "css" => "css",
        "html" => "html",
        "py" => "python",
        _ => return None,
    };
    Some(lang.into())
}

fn git(root: &Path, args: &[&str]) -> Result<String, WorkspaceError> {
    git_overlay::git(root, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsx_resolves_to_non_plain_syntax() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = resolve_syntax(&syntax_set, Path::new("Component.tsx"), Some("typescript"));
        assert_ne!(syntax.name, syntax_set.find_syntax_plain_text().name);
    }

    #[test]
    fn highlight_lines_returns_html_for_typescript() {
        let text = "export const mode = \"agent\";\n";
        let highlighted =
            highlight_lines_for_preview(text, Path::new("viewer.tsx"), Some("typescript"))
                .expect("expected highlighted lines for tsx content");
        assert!(!highlighted.is_empty());
        assert!(highlighted[0].contains("<span"));
    }

    #[test]
    fn highlight_lines_skips_large_payloads() {
        let text = "a".repeat(MAX_HIGHLIGHT_BYTES + 1);
        let highlighted =
            highlight_lines_for_preview(&text, Path::new("big.ts"), Some("typescript"));
        assert!(highlighted.is_none());
    }

    #[test]
    fn language_hint_for_tsx_is_typescript() {
        let language = language_for_path(Path::new("file.tsx"));
        assert_eq!(language.as_deref(), Some("typescript"));
    }
}
