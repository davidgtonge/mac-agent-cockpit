use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display};
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

macro_rules! id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new() -> Self {
                Self(format!("{}_{}", $prefix, uuid::Uuid::new_v4().simple()))
            }
            pub fn from_string(value: impl Into<String>) -> Self {
                Self(value.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(ProjectId, "project");
id_type!(ConversationId, "conversation");
id_type!(MessageId, "message");
id_type!(PermissionRequestId, "permission");
id_type!(PreviewId, "preview");
id_type!(EffectId, "effect");
id_type!(ToastId, "toast");

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConversationStatus {
    Idle,
    Starting,
    Running,
    WaitingForPermission,
    Paused,
    Throttling,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProcessRuntimeState {
    Starting,
    Running,
    Paused,
    Throttling,
    Exited,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PreviewRuntimeState {
    Opening,
    Open,
    Suspended,
    Destroyed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RightPaneMode {
    FileTree,
    FilePreview,
    ChangedFiles,
    Diff,
    Preview,
    Process,
    Browser,
}

impl Default for RightPaneMode {
    fn default() -> Self {
        Self::FileTree
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ConversationListMode {
    #[default]
    Recents,
    ByProject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceSearchMode {
    #[default]
    Both,
    Filename,
    Content,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSearchHit {
    pub conversation_id: ConversationId,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FilenameIndexEntry {
    pub path: String,
    pub name: String,
    pub modified_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchHit {
    pub path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub snippet: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QueuedPrompt {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessNodeVm {
    pub pid: i32,
    pub ppid: i32,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub path: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Project {
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("Workspace")
            .to_string();
        let now = now_ms();
        Self {
            id: project_id_for_path(&path),
            name,
            path,
            created_at: now,
            updated_at: now,
        }
    }
}

fn project_id_for_path(path: &str) -> ProjectId {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    ProjectId::from_string(format!("project_{:016x}", hasher.finish()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: ConversationId,
    pub cursor_session_id: Option<String>,
    pub project_id: ProjectId,
    pub title: String,
    pub summary: Option<String>,
    pub last_message_preview: Option<String>,
    pub status: ConversationStatus,
    pub message_count: u32,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_opened_at: Option<i64>,
}

impl Conversation {
    pub fn new(project_id: ProjectId) -> Self {
        let now = now_ms();
        Self {
            id: ConversationId::new(),
            cursor_session_id: None,
            project_id,
            title: "New agent conversation".into(),
            summary: None,
            last_message_preview: None,
            status: ConversationStatus::Starting,
            message_count: 0,
            created_at: now,
            updated_at: now,
            last_opened_at: Some(now),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    pub role: String,
    pub kind: String,
    pub text: String,
    pub ordinal: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Message {
    pub fn new(
        conversation_id: ConversationId,
        role: &str,
        kind: &str,
        text: String,
        ordinal: u32,
    ) -> Self {
        let now = now_ms();
        Self {
            id: MessageId::new(),
            conversation_id,
            role: role.into(),
            kind: kind.into(),
            text,
            ordinal,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOption {
    pub option_id: String,
    pub label: String,
    pub description: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequest {
    pub request_id: PermissionRequestId,
    pub conversation_id: ConversationId,
    pub acp_request_id: Option<Value>,
    pub title: String,
    pub summary: String,
    pub tool_call_title: Option<String>,
    pub tool_kind: Option<String>,
    pub body: String,
    pub options: Vec<PermissionOption>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBudget {
    pub max_cpu_percent: f32,
    pub max_memory_bytes: u64,
    pub max_processes: usize,
    pub background_policy: bool,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_cpu_percent: 200.0,
            max_memory_bytes: 4 * 1024 * 1024 * 1024,
            max_processes: 64,
            background_policy: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSample {
    pub conversation_id: ConversationId,
    pub root_pid: i32,
    pub pgid: i32,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub process_count: usize,
    pub sampled_at_ms: i64,
    pub state: ProcessRuntimeState,
    #[serde(default)]
    pub nodes: Vec<ProcessNodeVm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentRuntime {
    pub conversation_id: ConversationId,
    pub cursor_session_id: Option<String>,
    pub root_pid: Option<i32>,
    pub pgid: Option<i32>,
    pub budget: ResourceBudget,
    pub latest_sample: Option<ProcessSample>,
    pub state: ProcessRuntimeState,
    #[serde(default)]
    pub process_nodes: Vec<ProcessNodeVm>,
}

impl AgentRuntime {
    pub fn new(conversation_id: ConversationId) -> Self {
        Self {
            conversation_id,
            cursor_session_id: None,
            root_pid: None,
            pgid: None,
            budget: ResourceBudget::default(),
            latest_sample: None,
            state: ProcessRuntimeState::Starting,
            process_nodes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum GitFileStatus {
    Clean,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    Conflicted,
    TypeChanged,
    Binary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum FileReviewView {
    #[default]
    Current,
    Before,
    InlineChanges,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiffRowKind {
    Context,
    Added,
    Removed,
    HunkHeader,
    FileHeader,
    Notice,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: Option<u64>,
    pub modified_at_ms: Option<i64>,
    pub ignored: bool,
    #[serde(default)]
    pub git_status: Option<GitFileStatus>,
    #[serde(default)]
    pub change_count: Option<u32>,
    #[serde(default)]
    pub synthetic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    pub project_id: ProjectId,
    pub path: String,
    pub text: Option<String>,
    pub highlighted_lines: Option<Vec<String>>,
    pub binary: bool,
    pub truncated: bool,
    pub size_bytes: u64,
    pub language_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangedFile {
    pub path: String,
    pub status: GitFileStatus,
    #[serde(default)]
    pub old_path: Option<String>,
    pub additions: Option<u32>,
    pub deletions: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusEntry {
    pub status: GitFileStatus,
    pub old_path: Option<String>,
    pub additions: Option<u32>,
    pub deletions: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GitOverlayState {
    pub entries: BTreeMap<String, GitStatusEntry>,
    pub synthetic_nodes: Vec<FileNode>,
    pub changed_files: Vec<ChangedFile>,
    pub base_revision: String,
    pub refreshed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionBaseRevision {
    pub conversation_id: ConversationId,
    pub project_id: ProjectId,
    pub revision: String,
    pub branch: Option<String>,
    pub captured_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConversationEditedFiles {
    pub count: u32,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConversationWorkspaceState {
    pub right_pane_mode: RightPaneMode,
    pub selected_path: Option<String>,
    pub selected_review_view: FileReviewView,
    pub expanded_directories: Vec<String>,
    pub browser_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiffRowVm {
    pub kind: DiffRowKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub highlighted_html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunkVm {
    pub header: String,
    pub rows: Vec<DiffRowVm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StructuredDiffVm {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: GitFileStatus,
    pub stat: String,
    pub hunks: Vec<DiffHunkVm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChangedFileGroupVm {
    pub status: GitFileStatus,
    pub label: String,
    pub files: Vec<ChangedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileReviewVm {
    pub path: String,
    pub file_name: String,
    pub git_status: Option<GitFileStatus>,
    pub status_label: String,
    pub change_summary: Option<String>,
    pub comparison_label: Option<String>,
    pub context_notice: Option<String>,
    pub selected_view: FileReviewView,
    pub available_views: Vec<FileReviewView>,
    pub loading: bool,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub preview: Option<FilePreview>,
    pub inline_changes: Option<StructuredDiffVm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub project_id: ProjectId,
    pub path: Option<String>,
    pub stat: String,
    pub text: String,
    pub generated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewStatus {
    pub preview_id: PreviewId,
    pub project_id: ProjectId,
    pub url: String,
    pub state: PreviewRuntimeState,
    pub dev_server_pid: Option<i32>,
    pub last_detected_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AcpMessage {
    pub id: Option<Value>,
    pub method: Option<String>,
    pub params: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<Value>,
    pub raw_json: Value,
}

impl AcpMessage {
    pub fn from_value(raw_json: Value) -> Self {
        Self {
            id: raw_json.get("id").cloned(),
            method: raw_json
                .get("method")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned),
            params: raw_json.get("params").cloned(),
            result: raw_json.get("result").cloned(),
            error: raw_json.get("error").cloned(),
            raw_json,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AppEvent {
    ProjectSelected {
        project_id: ProjectId,
    },
    ProjectAdded {
        path: String,
    },
    ConversationSelected {
        conversation_id: ConversationId,
    },
    ConversationCreated {
        project_id: ProjectId,
    },
    ConversationArchived {
        conversation_id: ConversationId,
    },
    UserPromptSubmitted {
        conversation_id: ConversationId,
        text: String,
    },
    ComposerModeSelected {
        conversation_id: ConversationId,
        mode_id: String,
    },
    ComposerModelSelected {
        conversation_id: ConversationId,
        model_id: String,
    },

    AgentPermissionApproved {
        request_id: PermissionRequestId,
    },
    AgentPermissionRejected {
        request_id: PermissionRequestId,
    },
    AgentPermissionSelected {
        request_id: PermissionRequestId,
        option_id: String,
    },
    AgentCancelled {
        conversation_id: ConversationId,
    },
    AgentPaused {
        conversation_id: ConversationId,
    },
    AgentResumed {
        conversation_id: ConversationId,
    },
    AgentKilled {
        conversation_id: ConversationId,
    },
    AgentCpuBudgetChanged {
        conversation_id: ConversationId,
        cpu_percent: f32,
    },

    FileTreeNodeExpanded {
        project_id: ProjectId,
        path: String,
    },
    FileTreeNodeCollapsed {
        project_id: ProjectId,
        path: String,
    },
    FileSelected {
        project_id: ProjectId,
        path: String,
    },
    DiffFileSelected {
        project_id: ProjectId,
        path: String,
    },
    ChangedFilesRefreshed {
        project_id: ProjectId,
    },
    ChangedFileSelected {
        project_id: ProjectId,
        path: String,
    },
    ReviewViewSelected {
        project_id: ProjectId,
        path: String,
        view: FileReviewView,
    },
    GitRefreshRequested {
        project_id: ProjectId,
    },
    FileReviewClosed,

    PreviewOpened {
        project_id: ProjectId,
        url: String,
    },
    PreviewSuspended {
        preview_id: PreviewId,
    },
    PreviewClosed {
        preview_id: PreviewId,
    },
    DevServerStarted {
        project_id: ProjectId,
        command: String,
        args: Vec<String>,
    },
    SearchSubmitted {
        query: String,
    },
    ConversationListModeSelected {
        mode: ConversationListMode,
    },
    WorkspaceSearchSubmitted {
        project_id: ProjectId,
        query: String,
        mode: WorkspaceSearchMode,
    },
    WorkspaceSearchCancelled,
    QuickOpenToggled {
        open: bool,
    },
    WorkspaceSearchResultSelected {
        project_id: ProjectId,
        path: String,
    },
    QueuedPromptRemoved {
        conversation_id: ConversationId,
        prompt_id: String,
    },
    QueuedPromptEdited {
        conversation_id: ConversationId,
        prompt_id: String,
        text: String,
    },
    RightPaneModeSelected {
        mode: RightPaneMode,
    },
    BrowserUrlChanged {
        url: String,
    },

    SystemAcpStarted {
        conversation_id: ConversationId,
        root_pid: i32,
        pgid: i32,
    },
    SystemAcpSessionReady {
        conversation_id: ConversationId,
        cursor_session_id: String,
        suppress_replay: bool,
    },
    SystemAcpStartFailed {
        conversation_id: ConversationId,
    },
    SystemAcpMessageReceived {
        conversation_id: ConversationId,
        message: AcpMessage,
    },
    SystemAcpPromptCompleted {
        conversation_id: ConversationId,
    },
    SystemAcpSessionMetaReceived {
        conversation_id: ConversationId,
        payload: Value,
    },
    SystemAgentExited {
        conversation_id: ConversationId,
        code: Option<i32>,
        signal: Option<i32>,
    },
    SystemProcessSampled {
        sample: ProcessSample,
    },
    SystemDirectoryLoaded {
        project_id: ProjectId,
        path: String,
        children: Vec<FileNode>,
    },
    SystemFileLoaded {
        preview: FilePreview,
    },
    SystemChangedFilesComputed {
        project_id: ProjectId,
        files: Vec<ChangedFile>,
    },
    SystemWorkspaceDirty {
        project_id: ProjectId,
        paths: Vec<String>,
    },
    SystemGitOverlayRefreshed {
        project_id: ProjectId,
        overlay: GitOverlayState,
    },
    SystemSessionBaseCaptured {
        session: SessionBaseRevision,
    },
    SystemPrevFileLoaded {
        preview: FilePreview,
    },
    SystemStructuredDiffComputed {
        path: String,
        diff: StructuredDiffVm,
    },
    SystemFileReviewLoadFailed {
        message: String,
    },
    SystemDiffComputed {
        diff: DiffResult,
    },
    SystemPreviewStatusChanged {
        status: PreviewStatus,
    },
    SystemStorageWriteCompleted {
        effect_id: EffectId,
    },
    SystemStorageWriteFailed {
        effect_id: EffectId,
        error: String,
    },
    SystemMessageSearchResults {
        hits: Vec<ConversationSearchHit>,
    },
    SystemFilenameIndexReady {
        project_id: ProjectId,
        entries: Vec<FilenameIndexEntry>,
    },
    SystemSearchResultsPartial {
        project_id: ProjectId,
        hits: Vec<WorkspaceSearchHit>,
        done: bool,
    },
    SystemConversationMessagesLoaded {
        conversation_id: ConversationId,
        messages: Vec<Message>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum EffectCommand {
    StartCursorAcp {
        effect_id: EffectId,
        conversation_id: ConversationId,
        project_path: String,
        resume_session_id: Option<String>,
    },
    SendAcpPrompt {
        effect_id: EffectId,
        conversation_id: ConversationId,
        cursor_session_id: Option<String>,
        text: String,
    },
    SetAcpMode {
        effect_id: EffectId,
        conversation_id: ConversationId,
        cursor_session_id: Option<String>,
        mode_id: String,
    },
    SetAcpConfigOption {
        effect_id: EffectId,
        conversation_id: ConversationId,
        cursor_session_id: Option<String>,
        config_id: String,
        value_id: String,
    },
    RespondAcpPermission {
        effect_id: EffectId,
        conversation_id: ConversationId,
        acp_request_id: Option<Value>,
        option_id: String,
    },
    CancelAcpSession {
        effect_id: EffectId,
        conversation_id: ConversationId,
        cursor_session_id: Option<String>,
    },
    WriteProject {
        effect_id: EffectId,
        project: Project,
    },
    WriteConversation {
        effect_id: EffectId,
        conversation: Conversation,
    },
    DeleteConversation {
        effect_id: EffectId,
        conversation_id: ConversationId,
    },
    WriteConversationMessages {
        effect_id: EffectId,
        conversation_id: ConversationId,
        messages: Vec<Message>,
    },
    LoadConversationMessages {
        effect_id: EffectId,
        conversation_id: ConversationId,
    },
    WriteAcpEvent {
        effect_id: EffectId,
        conversation_id: ConversationId,
        direction: String,
        method: Option<String>,
        raw_json: Value,
    },
    WriteSessionBase {
        effect_id: EffectId,
        session: SessionBaseRevision,
    },
    WriteConversationWorkspace {
        effect_id: EffectId,
        conversation_id: ConversationId,
        workspace: ConversationWorkspaceState,
    },
    WriteConversationEditedFiles {
        effect_id: EffectId,
        conversation_id: ConversationId,
        edited_files: ConversationEditedFiles,
    },
    LoadDirectory {
        effect_id: EffectId,
        project_id: ProjectId,
        project_path: String,
        path: String,
    },
    LoadFilePreview {
        effect_id: EffectId,
        project_id: ProjectId,
        project_path: String,
        path: String,
    },
    ComputeChangedFiles {
        effect_id: EffectId,
        project_id: ProjectId,
        project_path: String,
    },
    ComputeDiff {
        effect_id: EffectId,
        project_id: ProjectId,
        project_path: String,
        path: Option<String>,
    },
    RefreshGitOverlay {
        effect_id: EffectId,
        project_id: ProjectId,
        project_path: String,
        base_revision: String,
    },
    CaptureSessionBaseRevision {
        effect_id: EffectId,
        conversation_id: ConversationId,
        project_id: ProjectId,
        project_path: String,
    },
    LoadPrevFile {
        effect_id: EffectId,
        project_id: ProjectId,
        project_path: String,
        path: String,
        base_revision: String,
        old_path: Option<String>,
    },
    ComputeStructuredDiff {
        effect_id: EffectId,
        project_id: ProjectId,
        project_path: String,
        path: String,
        base_revision: String,
        old_path: Option<String>,
        status: GitFileStatus,
    },
    PauseProcessGroup {
        effect_id: EffectId,
        conversation_id: ConversationId,
    },
    ResumeProcessGroup {
        effect_id: EffectId,
        conversation_id: ConversationId,
    },
    KillProcessGroup {
        effect_id: EffectId,
        conversation_id: ConversationId,
    },
    UpdateCpuBudget {
        effect_id: EffectId,
        conversation_id: ConversationId,
        cpu_percent: f32,
    },
    OpenPreview {
        effect_id: EffectId,
        project_id: ProjectId,
        url: String,
    },
    SuspendPreview {
        effect_id: EffectId,
        preview_id: PreviewId,
    },
    DestroyPreview {
        effect_id: EffectId,
        preview_id: PreviewId,
    },
    StartDevServer {
        effect_id: EffectId,
        project_id: ProjectId,
        project_path: String,
        command: String,
        args: Vec<String>,
    },
    SearchMessages {
        effect_id: EffectId,
        query: String,
        limit: usize,
    },
    BuildFilenameIndex {
        effect_id: EffectId,
        project_id: ProjectId,
        project_path: String,
    },
    SearchWorkspace {
        effect_id: EffectId,
        project_id: ProjectId,
        project_path: String,
        query: String,
        mode: WorkspaceSearchMode,
    },
    SteerAcpPrompt {
        effect_id: EffectId,
        conversation_id: ConversationId,
        cursor_session_id: Option<String>,
        text: String,
    },
    StopAcpSession {
        effect_id: EffectId,
        conversation_id: ConversationId,
        cursor_session_id: Option<String>,
        root_pid: Option<i32>,
    },
    UnregisterProcessGroup {
        effect_id: EffectId,
        conversation_id: ConversationId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum ViewModelPatch {
    Replace { path: String, value: Value },
    Remove { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub projects: BTreeMap<ProjectId, Project>,
    pub project_order: Vec<ProjectId>,
    pub conversations: BTreeMap<ConversationId, Conversation>,
    pub conversation_order: Vec<ConversationId>,
    pub messages: BTreeMap<ConversationId, Vec<Message>>,
    pub agents: BTreeMap<ConversationId, AgentRuntime>,
    pub pending_permissions: BTreeMap<PermissionRequestId, PermissionRequest>,
    pub streaming_message_ids: BTreeMap<String, MessageId>,
    pub acp_sessions: BTreeMap<ConversationId, AcpSessionState>,
    pub process_samples: BTreeMap<ConversationId, ProcessSample>,
    pub file_tree: BTreeMap<String, Vec<FileNode>>,
    pub selected_file: Option<FilePreview>,
    pub changed_files: BTreeMap<ProjectId, Vec<ChangedFile>>,
    pub git_overlays: BTreeMap<ProjectId, GitOverlayState>,
    pub session_base_revisions: BTreeMap<ConversationId, SessionBaseRevision>,
    pub git_overlay_refreshing: BTreeMap<ProjectId, bool>,
    pub selected_review_view: FileReviewView,
    pub structured_diff: Option<StructuredDiffVm>,
    pub file_viewer_loading: bool,
    pub file_viewer_error: Option<String>,
    pub file_viewer_notice: Option<String>,
    pub prev_file_cache: BTreeMap<String, FilePreview>,
    pub structured_diff_cache: BTreeMap<String, StructuredDiffVm>,
    pub selected_diff: Option<DiffResult>,
    pub previews: BTreeMap<PreviewId, PreviewStatus>,
    pub active_preview_id: Option<PreviewId>,
    pub selected_project_id: Option<ProjectId>,
    pub selected_conversation_id: Option<ConversationId>,
    pub selected_path: Option<String>,
    pub right_pane_mode: RightPaneMode,
    pub search_query: String,
    pub conversation_list_mode: ConversationListMode,
    pub conversation_search_hits: Vec<ConversationSearchHit>,
    pub filename_indexes: BTreeMap<ProjectId, Vec<FilenameIndexEntry>>,
    pub workspace_search_hits: Vec<WorkspaceSearchHit>,
    pub workspace_search_done: bool,
    pub quick_open_open: bool,
    pub prompt_queues: BTreeMap<ConversationId, Vec<QueuedPrompt>>,
    pub steer_supported: bool,
    pub active_plan_text: BTreeMap<ConversationId, String>,
    pub toasts: Vec<ToastVm>,
    pub acp_connecting: BTreeSet<ConversationId>,
    pub acp_replay_suppressed: BTreeSet<ConversationId>,
    pub pending_connect_prompts: BTreeMap<ConversationId, String>,
    pub loaded_message_conversations: BTreeSet<ConversationId>,
    pub conversation_workspace: BTreeMap<ConversationId, ConversationWorkspaceState>,
    pub conversation_edited_files: BTreeMap<ConversationId, ConversationEditedFiles>,
    pub browser_url: Option<String>,
    pub dispatch_timing_history: Vec<DispatchTimingVm>,
}

impl AppState {
    pub fn with_project(path: impl Into<String>) -> Self {
        let mut state = Self::default();
        let project = Project::new(path);
        state.selected_project_id = Some(project.id.clone());
        state.project_order.push(project.id.clone());
        state.projects.insert(project.id.clone(), project);
        state
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ViewModel {
    pub left_pane: LeftPaneVm,
    pub center_pane: CenterPaneVm,
    pub right_pane: RightPaneVm,
    pub status_bar: StatusBarVm,
    pub modals: Vec<ModalVm>,
    pub toasts: Vec<ToastVm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LeftPaneVm {
    pub projects: Vec<ProjectVm>,
    pub conversations: Vec<ConversationRowVm>,
    pub project_groups: Vec<ProjectConversationGroupVm>,
    pub agents: Vec<AgentRowVm>,
    pub pressure: PressureVm,
    pub selected_project_id: Option<ProjectId>,
    pub selected_conversation_id: Option<ConversationId>,
    pub search_query: String,
    pub conversation_list_mode: ConversationListMode,
    pub search_hits: Vec<ConversationSearchHitVm>,
    pub quick_open_open: bool,
    pub workspace_search_hits: Vec<WorkspaceSearchHit>,
    pub workspace_search_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectVm {
    pub id: ProjectId,
    pub name: String,
    pub path: String,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConversationGroupVm {
    pub project: ProjectVm,
    pub conversations: Vec<ConversationRowVm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationRowVm {
    pub id: ConversationId,
    pub project_id: ProjectId,
    pub title: String,
    pub status: ConversationStatus,
    pub last_message_preview: Option<String>,
    pub message_count: u32,
    pub selected: bool,
    pub updated_at: i64,
    pub acp_connected: bool,
    pub acp_connecting: bool,
    pub cpu_percent: f32,
    pub process_state: Option<ProcessRuntimeState>,
    pub edited_file_count: u32,
    pub edited_file_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSearchHitVm {
    pub conversation_id: ConversationId,
    pub title: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentRowVm {
    pub id: ConversationId,
    pub title: String,
    pub state: ProcessRuntimeState,
    pub cpu_label: String,
    pub memory_label: String,
    pub process_label: String,
    pub budget_cpu_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PressureVm {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub process_count: usize,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AcpSessionState {
    pub current_mode: Option<String>,
    pub available_modes: Vec<String>,
    pub mode_config_id: Option<String>,
    pub mode_options: Vec<ModeOptionVm>,
    pub model_config_id: Option<String>,
    pub model_value_id: Option<String>,
    pub model_options: Vec<ModelOptionVm>,
    pub slash_commands: Vec<SlashCommandVm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModeOptionVm {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelOptionVm {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommandVm {
    pub name: String,
    pub description: Option<String>,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CenterPaneVm {
    pub project_name: String,
    pub selected_conversation_id: Option<ConversationId>,
    pub title: String,
    pub status: Option<ConversationStatus>,
    pub messages: Vec<MessageVm>,
    pub approvals: Vec<PermissionRequestVm>,
    pub tool_status: ToolStatusVm,
    pub composer_enabled: bool,
    pub slash_commands: Vec<SlashCommandVm>,
    pub mode_options: Vec<ModeOptionVm>,
    pub model_options: Vec<ModelOptionVm>,
    pub current_mode: Option<String>,
    pub current_mode_label: Option<String>,
    pub current_model_id: Option<String>,
    pub current_model_label: Option<String>,
    pub acp_connected: bool,
    pub acp_status_label: String,
    pub cpu_percent: f32,
    pub cpu_budget_percent: f32,
    pub plan_text: Option<String>,
    pub plan_visible: bool,
    pub queued_prompts: Vec<QueuedPrompt>,
    pub agent_running: bool,
    pub steer_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MessageVm {
    pub id: MessageId,
    pub role: String,
    pub kind: String,
    pub text: String,
    pub ordinal: u32,
    pub streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequestVm {
    pub request_id: PermissionRequestId,
    pub conversation_id: ConversationId,
    pub title: String,
    pub summary: String,
    pub tool_call_title: Option<String>,
    pub tool_kind: Option<String>,
    pub body: String,
    pub options: Vec<PermissionOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatusVm {
    pub running: u32,
    pub completed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RightPaneVm {
    pub project_name: String,
    pub mode: RightPaneMode,
    pub file_tree: FileTreeVm,
    pub selected_file: Option<FilePreview>,
    pub file_review: Option<FileReviewVm>,
    pub changed_files: Vec<ChangedFile>,
    pub changed_file_groups: Vec<ChangedFileGroupVm>,
    pub git_refreshing: bool,
    pub session_base_revision: Option<String>,
    pub selected_diff: Option<DiffResult>,
    pub preview: Option<PreviewStatus>,
    pub process: Option<ProcessDetailVm>,
    pub global_processes: Vec<GlobalProcessRowVm>,
    pub dispatch_timings: Vec<DispatchTimingVm>,
    pub browser_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GlobalProcessRowVm {
    pub conversation_id: ConversationId,
    pub title: String,
    pub state: ProcessRuntimeState,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub process_count: usize,
    pub root_pid: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileTreeVm {
    pub project_id: Option<ProjectId>,
    pub expanded: Vec<ExpandedDirectoryVm>,
    pub selected_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExpandedDirectoryVm {
    pub path: String,
    pub children: Vec<FileNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDetailVm {
    pub conversation_id: ConversationId,
    pub conversation_title: String,
    pub state: ProcessRuntimeState,
    pub root_pid: Option<i32>,
    pub pgid: Option<i32>,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub process_count: usize,
    pub cpu_budget_percent: f32,
    pub nodes: Vec<ProcessNodeVm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EffectTimingVm {
    pub name: String,
    pub duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DispatchTimingVm {
    pub event: String,
    pub reduce_ms: f64,
    pub initial_patch_ms: f64,
    pub effects: Vec<EffectTimingVm>,
    pub drain_io_ms: f64,
    pub finalize_patch_ms: f64,
    pub response_prep_ms: f64,
    pub patch_count: usize,
    pub patch_paths: Vec<String>,
    pub server_total_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusBarVm {
    pub selected_project_label: String,
    pub selected_conversation_label: String,
    pub agent_label: String,
    pub storage_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModalVm {
    pub id: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToastVm {
    pub id: ToastId,
    pub level: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EngineOutput {
    pub patches: Vec<ViewModelPatch>,
    pub effects: Vec<EffectCommand>,
    pub diagnostics: Vec<Diagnostic>,
}

pub type EngineResponse = EngineOutput;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct InitPayload {
    pub initial_project_path: Option<String>,
}

pub struct Engine {
    state: AppState,
    previous_view_model: Option<ViewModel>,
}

impl Engine {
    pub fn new(init: InitPayload) -> Result<Self, EngineError> {
        let state = init
            .initial_project_path
            .map(AppState::with_project)
            .unwrap_or_default();
        let previous_view_model = Some(select_view_model(&state));
        Ok(Self {
            state,
            previous_view_model,
        })
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn hydrate_projects(&mut self, projects: Vec<Project>) {
        for project in projects {
            if !self.state.project_order.contains(&project.id) {
                self.state.project_order.push(project.id.clone());
            }
            self.state.projects.insert(project.id.clone(), project);
        }
        if self.state.selected_project_id.is_none() {
            self.state.selected_project_id = self.state.project_order.first().cloned();
        }
        self.previous_view_model = Some(select_view_model(&self.state));
    }

    pub fn hydrate_persisted_conversation_state(
        &mut self,
        session_bases: Vec<SessionBaseRevision>,
        workspaces: Vec<(ConversationId, ConversationWorkspaceState)>,
        edited_files: Vec<(ConversationId, ConversationEditedFiles)>,
    ) {
        for session in session_bases {
            self.state
                .session_base_revisions
                .insert(session.conversation_id.clone(), session);
        }
        for (conversation_id, workspace) in workspaces {
            self.state
                .conversation_workspace
                .insert(conversation_id, workspace);
        }
        for (conversation_id, edited) in edited_files {
            self.state
                .conversation_edited_files
                .insert(conversation_id, edited);
        }
    }

    pub fn hydrate_conversations(
        &mut self,
        conversations: Vec<Conversation>,
        mut messages: BTreeMap<ConversationId, Vec<Message>>,
    ) -> Vec<EffectCommand> {
        let mut effects = Vec::new();
        for mut conversation in conversations {
            if is_agent_active(&conversation.status) {
                conversation.status = ConversationStatus::Idle;
            }
            if !self.state.conversation_order.contains(&conversation.id) {
                self.state.conversation_order.push(conversation.id.clone());
            }
            let loaded_messages = messages.remove(&conversation.id).unwrap_or_default();
            if !loaded_messages.is_empty() {
                self.state.messages.insert(conversation.id.clone(), loaded_messages);
                self.state
                    .loaded_message_conversations
                    .insert(conversation.id.clone());
            } else {
                self.state.messages.insert(conversation.id.clone(), Vec::new());
            }
            let mut agent = AgentRuntime::new(conversation.id.clone());
            agent.state = ProcessRuntimeState::Exited;
            self.state
                .agents
                .insert(conversation.id.clone(), agent);
            self.state
                .conversations
                .insert(conversation.id.clone(), conversation);
        }
        self.state.conversation_order.sort_by(|a, b| {
            let aa = self
                .state
                .conversations
                .get(a)
                .map(|c| c.updated_at)
                .unwrap_or_default();
            let bb = self
                .state
                .conversations
                .get(b)
                .map(|c| c.updated_at)
                .unwrap_or_default();
            bb.cmp(&aa)
        });
        if self.state.selected_conversation_id.is_none() {
            self.state.selected_conversation_id = self.state.conversation_order.first().cloned();
        }
        if let Some(conversation_id) = self.state.selected_conversation_id.clone() {
            restore_conversation_workspace(&mut self.state, &conversation_id, &mut effects);
            if !self.state.loaded_message_conversations.contains(&conversation_id) {
                effects.push(EffectCommand::LoadConversationMessages {
                    effect_id: EffectId::new(),
                    conversation_id,
                });
            }
        }
        self.previous_view_model = Some(select_view_model(&self.state));
        effects
    }

    pub fn snapshot_view_model(&mut self) -> Result<EngineOutput, EngineError> {
        let next = select_view_model(&self.state);
        self.previous_view_model = Some(next.clone());
        Ok(EngineOutput {
            patches: vec![ViewModelPatch::Replace {
                path: "".into(),
                value: serde_json::to_value(next)?,
            }],
            effects: vec![],
            diagnostics: vec![],
        })
    }

    pub fn previous_view_model(&self) -> ViewModel {
        self.previous_view_model
            .clone()
            .unwrap_or_else(|| select_view_model(&self.state))
    }

    /// Re-diff the view model after effect handlers have mutated state. `before` must be the
    /// view model the UI had when the dispatch started (before `handle_input`).
    pub fn finalize_after_effects(
        &mut self,
        before: &ViewModel,
    ) -> Result<Vec<ViewModelPatch>, EngineError> {
        let next = select_view_model(&self.state);
        let patches = diff_view_model(Some(before), &next)?;
        self.previous_view_model = Some(next);
        Ok(patches)
    }

    pub fn handle_input(&mut self, event: AppEvent) -> Result<EngineOutput, EngineError> {
        let (output, _, _) = self.handle_input_traced(event)?;
        Ok(output)
    }

    /// Apply an event to engine state without building or diffing the view model.
    /// Used while effect handlers run; `finalize_after_effects` projects patches once.
    pub fn reduce_event(&mut self, event: AppEvent) -> Result<Vec<EffectCommand>, EngineError> {
        let mut effects = Vec::new();
        self.reduce(event, &mut effects)?;
        Ok(effects)
    }

    pub fn handle_input_traced(
        &mut self,
        event: AppEvent,
    ) -> Result<(EngineOutput, f64, f64), EngineError> {
        let mut effects = Vec::new();
        let reduce_start = std::time::Instant::now();
        self.reduce(event, &mut effects)?;
        let reduce_ms = reduce_start.elapsed().as_secs_f64() * 1000.0;

        let patch_start = std::time::Instant::now();
        let patches = if effects.is_empty() {
            let next = select_view_model(&self.state);
            let patches = diff_view_model(self.previous_view_model.as_ref(), &next)?;
            self.previous_view_model = Some(next);
            patches
        } else {
            Vec::new()
        };
        let initial_patch_ms = patch_start.elapsed().as_secs_f64() * 1000.0;

        Ok((
            EngineOutput {
                patches,
                effects,
                diagnostics: vec![],
            },
            reduce_ms,
            initial_patch_ms,
        ))
    }

    pub fn record_dispatch_timing(&mut self, timing: DispatchTimingVm) {
        const MAX: usize = 10;
        self.state.dispatch_timing_history.insert(0, timing);
        self.state.dispatch_timing_history.truncate(MAX);
    }

    pub fn dispatch_timing_history(&self) -> &[DispatchTimingVm] {
        &self.state.dispatch_timing_history
    }

    fn reduce(
        &mut self,
        event: AppEvent,
        effects: &mut Vec<EffectCommand>,
    ) -> Result<(), EngineError> {
        match event {
            AppEvent::ProjectSelected { project_id } => {
                let project = self
                    .state
                    .projects
                    .get(&project_id)
                    .cloned()
                    .ok_or_else(|| EngineError::NotFound(project_id.to_string()))?;
                self.state.selected_project_id = Some(project_id.clone());
                if !self.state.file_tree.contains_key(".") {
                    effects.push(EffectCommand::LoadDirectory {
                        effect_id: EffectId::new(),
                        project_id: project_id.clone(),
                        project_path: project.path.clone(),
                        path: ".".into(),
                    });
                }
                self.queue_git_refresh(&project_id, &project.path, effects)?;
            }
            AppEvent::ProjectAdded { path } => {
                let path_buf = std::path::Path::new(&path);
                let path = path_buf
                    .canonicalize()
                    .unwrap_or_else(|_| path_buf.to_path_buf())
                    .to_string_lossy()
                    .to_string();
                if let Some(existing) = self
                    .state
                    .projects
                    .values()
                    .find(|p| p.path == path)
                    .cloned()
                {
                    self.state.selected_project_id = Some(existing.id.clone());
                    effects.push(EffectCommand::LoadDirectory {
                        effect_id: EffectId::new(),
                        project_id: existing.id,
                        project_path: path,
                        path: ".".into(),
                    });
                    return Ok(());
                }
                let project = Project::new(path.clone());
                let project_id = project.id.clone();
                self.state
                    .projects
                    .insert(project_id.clone(), project.clone());
                if !self.state.project_order.contains(&project_id) {
                    self.state.project_order.push(project_id.clone());
                }
                self.state.selected_project_id = Some(project_id.clone());
                effects.push(EffectCommand::WriteProject {
                    effect_id: EffectId::new(),
                    project,
                });
                effects.push(EffectCommand::LoadDirectory {
                    effect_id: EffectId::new(),
                    project_id: project_id.clone(),
                    project_path: path.clone(),
                    path: ".".into(),
                });
                effects.push(EffectCommand::BuildFilenameIndex {
                    effect_id: EffectId::new(),
                    project_id: project_id.clone(),
                    project_path: path.clone(),
                });
                self.queue_git_refresh(&project_id, &path, effects)?;
            }
            AppEvent::ConversationCreated { project_id } => {
                require(
                    self.state.projects.contains_key(&project_id),
                    &project_id.to_string(),
                )?;
                self.state.selected_project_id = Some(project_id.clone());
                let conversation = Conversation::new(project_id.clone());
                let conversation_id = conversation.id.clone();
                self.state
                    .conversation_order
                    .insert(0, conversation_id.clone());
                self.state
                    .conversations
                    .insert(conversation_id.clone(), conversation.clone());
                self.state
                    .messages
                    .insert(conversation_id.clone(), Vec::new());
                self.state.agents.insert(
                    conversation_id.clone(),
                    AgentRuntime::new(conversation_id.clone()),
                );
                self.state.selected_conversation_id = Some(conversation_id.clone());
                effects.push(EffectCommand::WriteConversation {
                    effect_id: EffectId::new(),
                    conversation,
                });
                if let Some(project) = self.state.projects.get(&project_id) {
                    effects.push(EffectCommand::CaptureSessionBaseRevision {
                        effect_id: EffectId::new(),
                        conversation_id: conversation_id.clone(),
                        project_id: project_id.clone(),
                        project_path: project.path.clone(),
                    });
                }
                self.request_acp_connection(&conversation_id, effects)?;
            }
            AppEvent::ConversationSelected { conversation_id } => {
                if let Some(previous_id) = self.state.selected_conversation_id.clone() {
                    if previous_id != conversation_id {
                        persist_conversation_workspace(&mut self.state, &previous_id, effects);
                    }
                }
                let c = self
                    .state
                    .conversations
                    .get_mut(&conversation_id)
                    .ok_or_else(|| EngineError::NotFound(conversation_id.to_string()))?;
                c.last_opened_at = Some(now_ms());
                self.state.selected_project_id = Some(c.project_id.clone());
                self.state.selected_conversation_id = Some(conversation_id.clone());
                restore_conversation_workspace(&mut self.state, &conversation_id, effects);
                self.ensure_conversation_messages_loaded(&conversation_id, effects);
                self.request_acp_connection(&conversation_id, effects)?;
            }
            AppEvent::ConversationArchived { conversation_id } => {
                let cursor_session_id = self
                    .state
                    .conversations
                    .get(&conversation_id)
                    .and_then(|c| c.cursor_session_id.clone());
                self.state
                    .conversation_order
                    .retain(|id| id != &conversation_id);
                self.state.conversations.remove(&conversation_id);
                self.state.messages.remove(&conversation_id);
                self.state.agents.remove(&conversation_id);
                self.state.acp_sessions.remove(&conversation_id);
                self.state.pending_connect_prompts.remove(&conversation_id);
                self.state.conversation_workspace.remove(&conversation_id);
                self.state.conversation_edited_files.remove(&conversation_id);
                self.state.loaded_message_conversations.remove(&conversation_id);
                self.state.pending_permissions
                    .retain(|_, req| req.conversation_id != conversation_id);
                if self.state.selected_conversation_id.as_ref() == Some(&conversation_id) {
                    self.state.selected_conversation_id =
                        self.state.conversation_order.first().cloned();
                }
                effects.push(EffectCommand::DeleteConversation {
                    effect_id: EffectId::new(),
                    conversation_id: conversation_id.clone(),
                });
                if cursor_session_id.is_some() {
                    effects.push(EffectCommand::CancelAcpSession {
                        effect_id: EffectId::new(),
                        conversation_id,
                        cursor_session_id,
                    });
                }
            }
            AppEvent::UserPromptSubmitted {
                conversation_id,
                text,
            } => {
                self.clear_active_stream_keys(&conversation_id);
                let cursor_session_id = self
                    .state
                    .conversations
                    .get(&conversation_id)
                    .and_then(|c| c.cursor_session_id.clone());
                let acp = self
                    .state
                    .acp_sessions
                    .get(&conversation_id)
                    .cloned()
                    .unwrap_or_default();
                let running = self
                    .state
                    .conversations
                    .get(&conversation_id)
                    .map(|c| is_agent_active(&c.status))
                    .unwrap_or(false);
                match parse_composer_input(&text, &acp) {
                    ComposerSubmit::Prompt(prompt) => {
                        let live_connected = self
                            .state
                            .agents
                            .get(&conversation_id)
                            .is_some_and(acp_is_connected_this_session);
                        if !live_connected {
                            let message = self.append_message(
                                conversation_id.clone(),
                                "user",
                                "text",
                                prompt.clone(),
                                ConversationStatus::Starting,
                            )?;
                            effects.push(EffectCommand::WriteConversationMessages {
                                effect_id: EffectId::new(),
                                conversation_id: conversation_id.clone(),
                                messages: vec![message],
                            });
                            self.state
                                .pending_connect_prompts
                                .insert(conversation_id.clone(), prompt);
                            self.request_acp_connection(&conversation_id, effects)?;
                            return Ok(());
                        }
                        if running {
                            if self.state.steer_supported {
                                let message = self.append_message(
                                    conversation_id.clone(),
                                    "user",
                                    "text",
                                    prompt.clone(),
                                    ConversationStatus::Running,
                                )?;
                                effects.push(EffectCommand::WriteConversationMessages {
                                    effect_id: EffectId::new(),
                                    conversation_id: conversation_id.clone(),
                                    messages: vec![message],
                                });
                                effects.push(EffectCommand::SteerAcpPrompt {
                                    effect_id: EffectId::new(),
                                    conversation_id: conversation_id.clone(),
                                    cursor_session_id,
                                    text: prompt,
                                });
                            } else {
                                let queue = self
                                    .state
                                    .prompt_queues
                                    .entry(conversation_id.clone())
                                    .or_default();
                                queue.push(QueuedPrompt {
                                    id: format!("qp_{}", uuid::Uuid::new_v4().simple()),
                                    text: prompt,
                                });
                            }
                            return Ok(());
                        }
                        let message = self.append_message(
                            conversation_id.clone(),
                            "user",
                            "text",
                            prompt.clone(),
                            ConversationStatus::Running,
                        )?;
                        effects.push(EffectCommand::WriteConversationMessages {
                            effect_id: EffectId::new(),
                            conversation_id: conversation_id.clone(),
                            messages: vec![message],
                        });
                        effects.push(EffectCommand::SendAcpPrompt {
                            effect_id: EffectId::new(),
                            conversation_id,
                            cursor_session_id,
                            text: prompt,
                        });
                    }
                }
            }
            AppEvent::ComposerModeSelected {
                conversation_id,
                mode_id,
            } => {
                let cursor_session_id = self
                    .state
                    .conversations
                    .get(&conversation_id)
                    .and_then(|c| c.cursor_session_id.clone());
                let acp = self
                    .state
                    .acp_sessions
                    .get(&conversation_id)
                    .cloned()
                    .unwrap_or_default();
                self.acp_session_mut(&conversation_id).current_mode = Some(mode_id.clone());
                if let Some(config_id) = acp.mode_config_id {
                    effects.push(EffectCommand::SetAcpConfigOption {
                        effect_id: EffectId::new(),
                        conversation_id,
                        cursor_session_id,
                        config_id,
                        value_id: mode_id,
                    });
                } else {
                    effects.push(EffectCommand::SetAcpMode {
                        effect_id: EffectId::new(),
                        conversation_id,
                        cursor_session_id,
                        mode_id,
                    });
                }
            }
            AppEvent::ComposerModelSelected {
                conversation_id,
                model_id,
            } => {
                let cursor_session_id = self
                    .state
                    .conversations
                    .get(&conversation_id)
                    .and_then(|c| c.cursor_session_id.clone());
                let acp = self
                    .state
                    .acp_sessions
                    .get(&conversation_id)
                    .cloned()
                    .unwrap_or_default();
                self.acp_session_mut(&conversation_id).model_value_id = Some(model_id.clone());
                let Some(config_id) = acp.model_config_id else {
                    return Ok(());
                };
                effects.push(EffectCommand::SetAcpConfigOption {
                    effect_id: EffectId::new(),
                    conversation_id,
                    cursor_session_id,
                    config_id,
                    value_id: model_id,
                });
            }
            AppEvent::AgentPermissionApproved { request_id } => {
                self.respond_permission_selection(request_id, "allow-once", effects)?;
            }
            AppEvent::AgentPermissionRejected { request_id } => {
                self.respond_permission_selection(request_id, "reject-once", effects)?;
            }
            AppEvent::AgentPermissionSelected {
                request_id,
                option_id,
            } => {
                self.respond_permission_selection(request_id, &option_id, effects)?;
            }
            AppEvent::AgentCancelled { conversation_id } => {
                self.clear_active_stream_keys(&conversation_id);
                let cursor_session_id = self
                    .state
                    .conversations
                    .get(&conversation_id)
                    .and_then(|c| c.cursor_session_id.clone());
                let root_pid = self
                    .state
                    .agents
                    .get(&conversation_id)
                    .and_then(|a| a.root_pid);
                self.state.prompt_queues.remove(&conversation_id);
                self.set_conversation_status(&conversation_id, ConversationStatus::Cancelled)?;
                effects.push(EffectCommand::StopAcpSession {
                    effect_id: EffectId::new(),
                    conversation_id: conversation_id.clone(),
                    cursor_session_id,
                    root_pid,
                });
                effects.push(EffectCommand::UnregisterProcessGroup {
                    effect_id: EffectId::new(),
                    conversation_id,
                });
            }
            AppEvent::AgentPaused { conversation_id } => {
                self.set_agent_state(&conversation_id, ProcessRuntimeState::Paused);
                self.set_conversation_status(&conversation_id, ConversationStatus::Paused)?;
                effects.push(EffectCommand::PauseProcessGroup {
                    effect_id: EffectId::new(),
                    conversation_id,
                });
            }
            AppEvent::AgentResumed { conversation_id } => {
                self.set_agent_state(&conversation_id, ProcessRuntimeState::Running);
                self.set_conversation_status(&conversation_id, ConversationStatus::Running)?;
                effects.push(EffectCommand::ResumeProcessGroup {
                    effect_id: EffectId::new(),
                    conversation_id,
                });
            }
            AppEvent::AgentKilled { conversation_id } => {
                self.clear_active_stream_keys(&conversation_id);
                let cursor_session_id = self
                    .state
                    .conversations
                    .get(&conversation_id)
                    .and_then(|c| c.cursor_session_id.clone());
                let root_pid = self
                    .state
                    .agents
                    .get(&conversation_id)
                    .and_then(|a| a.root_pid);
                self.state.prompt_queues.remove(&conversation_id);
                self.set_agent_state(&conversation_id, ProcessRuntimeState::Exited);
                self.set_conversation_status(&conversation_id, ConversationStatus::Cancelled)?;
                effects.push(EffectCommand::KillProcessGroup {
                    effect_id: EffectId::new(),
                    conversation_id: conversation_id.clone(),
                });
                effects.push(EffectCommand::StopAcpSession {
                    effect_id: EffectId::new(),
                    conversation_id: conversation_id.clone(),
                    cursor_session_id,
                    root_pid,
                });
                effects.push(EffectCommand::UnregisterProcessGroup {
                    effect_id: EffectId::new(),
                    conversation_id,
                });
            }
            AppEvent::AgentCpuBudgetChanged {
                conversation_id,
                cpu_percent,
            } => {
                if let Some(agent) = self.state.agents.get_mut(&conversation_id) {
                    agent.budget.max_cpu_percent = cpu_percent.max(10.0);
                }
                effects.push(EffectCommand::UpdateCpuBudget {
                    effect_id: EffectId::new(),
                    conversation_id,
                    cpu_percent,
                });
            }
            AppEvent::FileTreeNodeExpanded { project_id, path } => {
                let project = self
                    .state
                    .projects
                    .get(&project_id)
                    .cloned()
                    .ok_or_else(|| EngineError::NotFound(project_id.to_string()))?;
                self.state.right_pane_mode = RightPaneMode::FileTree;
                let normalized = normalize_workspace_path(&path);
                effects.push(EffectCommand::LoadDirectory {
                    effect_id: EffectId::new(),
                    project_id,
                    project_path: project.path,
                    path: normalized,
                });
                persist_current_conversation_workspace(&mut self.state, effects);
            }
            AppEvent::FileTreeNodeCollapsed { project_id: _, path } => {
                self.state.right_pane_mode = RightPaneMode::FileTree;
                let normalized = normalize_workspace_path(&path);
                let child_prefix = if normalized == "." {
                    String::new()
                } else {
                    format!("{}/", normalized)
                };
                self.state.file_tree.retain(|dir, _| {
                    let dir = normalize_workspace_path(dir);
                    dir != normalized && (child_prefix.is_empty() || !dir.starts_with(&child_prefix))
                });
                persist_current_conversation_workspace(&mut self.state, effects);
            }
            AppEvent::FileSelected { project_id, path } => {
                let project = self
                    .state
                    .projects
                    .get(&project_id)
                    .cloned()
                    .ok_or_else(|| EngineError::NotFound(project_id.to_string()))?;
                self.begin_file_view(project_id, &project.path, path, None, effects)?;
            }
            AppEvent::ChangedFileSelected { project_id, path } => {
                let project = self
                    .state
                    .projects
                    .get(&project_id)
                    .cloned()
                    .ok_or_else(|| EngineError::NotFound(project_id.to_string()))?;
                self.begin_file_view(
                    project_id,
                    &project.path,
                    path,
                    Some(FileReviewView::InlineChanges),
                    effects,
                )?;
            }
            AppEvent::ReviewViewSelected {
                project_id,
                path,
                view,
            } => {
                let project = self
                    .state
                    .projects
                    .get(&project_id)
                    .cloned()
                    .ok_or_else(|| EngineError::NotFound(project_id.to_string()))?;
                self.state.selected_review_view = view;
                self.state.file_viewer_notice = None;
                self.state.file_viewer_error = None;
                self.state.selected_path = Some(path.clone());
                self.state.right_pane_mode = RightPaneMode::FilePreview;
                self.schedule_review_load(project_id, &project.path, &path, view, effects)?;
            }
            AppEvent::GitRefreshRequested { project_id } => {
                let project = self
                    .state
                    .projects
                    .get(&project_id)
                    .cloned()
                    .ok_or_else(|| EngineError::NotFound(project_id.to_string()))?;
                self.queue_git_refresh(&project_id, &project.path, effects)?;
            }
            AppEvent::FileReviewClosed => {
                self.state.selected_path = None;
                self.state.selected_file = None;
                self.state.structured_diff = None;
                self.state.file_viewer_loading = false;
                self.state.file_viewer_error = None;
                self.state.file_viewer_notice = None;
                self.state.right_pane_mode = RightPaneMode::FileTree;
            }
            AppEvent::ChangedFilesRefreshed { project_id } => {
                let project = self
                    .state
                    .projects
                    .get(&project_id)
                    .cloned()
                    .ok_or_else(|| EngineError::NotFound(project_id.to_string()))?;
                self.state.right_pane_mode = RightPaneMode::ChangedFiles;
                self.queue_git_refresh(&project_id, &project.path, effects)?;
            }
            AppEvent::DiffFileSelected { project_id, path } => {
                let project = self
                    .state
                    .projects
                    .get(&project_id)
                    .cloned()
                    .ok_or_else(|| EngineError::NotFound(project_id.to_string()))?;
                self.state.right_pane_mode = RightPaneMode::Diff;
                effects.push(EffectCommand::ComputeDiff {
                    effect_id: EffectId::new(),
                    project_id,
                    project_path: project.path,
                    path: Some(path),
                });
            }
            AppEvent::PreviewOpened { project_id, url } => {
                self.state.right_pane_mode = RightPaneMode::Preview;
                effects.push(EffectCommand::OpenPreview {
                    effect_id: EffectId::new(),
                    project_id,
                    url,
                });
            }
            AppEvent::PreviewSuspended { preview_id } => {
                effects.push(EffectCommand::SuspendPreview {
                    effect_id: EffectId::new(),
                    preview_id,
                })
            }
            AppEvent::PreviewClosed { preview_id } => effects.push(EffectCommand::DestroyPreview {
                effect_id: EffectId::new(),
                preview_id,
            }),
            AppEvent::DevServerStarted {
                project_id,
                command,
                args,
            } => {
                let project = self
                    .state
                    .projects
                    .get(&project_id)
                    .cloned()
                    .ok_or_else(|| EngineError::NotFound(project_id.to_string()))?;
                effects.push(EffectCommand::StartDevServer {
                    effect_id: EffectId::new(),
                    project_id,
                    project_path: project.path,
                    command,
                    args,
                });
            }
            AppEvent::SearchSubmitted { query } => {
                self.state.search_query = query.clone();
                if query.trim().is_empty() {
                    self.state.conversation_search_hits.clear();
                } else {
                    effects.push(EffectCommand::SearchMessages {
                        effect_id: EffectId::new(),
                        query: query.trim().to_string(),
                        limit: 50,
                    });
                }
            }
            AppEvent::ConversationListModeSelected { mode } => {
                self.state.conversation_list_mode = mode;
            }
            AppEvent::WorkspaceSearchSubmitted {
                project_id,
                query,
                mode,
            } => {
                self.state.workspace_search_hits.clear();
                self.state.workspace_search_done = false;
                if let Some(project) = self.state.projects.get(&project_id) {
                    effects.push(EffectCommand::SearchWorkspace {
                        effect_id: EffectId::new(),
                        project_id,
                        project_path: project.path.clone(),
                        query,
                        mode,
                    });
                }
            }
            AppEvent::WorkspaceSearchCancelled => {
                self.state.workspace_search_hits.clear();
                self.state.workspace_search_done = true;
            }
            AppEvent::QuickOpenToggled { open } => {
                self.state.quick_open_open = open;
                if !open {
                    self.state.workspace_search_hits.clear();
                    self.state.workspace_search_done = true;
                }
            }
            AppEvent::WorkspaceSearchResultSelected { project_id, path } => {
                self.state.quick_open_open = false;
                self.state.selected_project_id = Some(project_id.clone());
                if let Some(project) = self.state.projects.get(&project_id) {
                    self.begin_file_view(project_id, &project.path.clone(), path, None, effects)?;
                }
            }
            AppEvent::QueuedPromptRemoved {
                conversation_id,
                prompt_id,
            } => {
                if let Some(queue) = self.state.prompt_queues.get_mut(&conversation_id) {
                    queue.retain(|p| p.id != prompt_id);
                }
            }
            AppEvent::QueuedPromptEdited {
                conversation_id,
                prompt_id,
                text,
            } => {
                if let Some(queue) = self.state.prompt_queues.get_mut(&conversation_id) {
                    if let Some(prompt) = queue.iter_mut().find(|p| p.id == prompt_id) {
                        prompt.text = text;
                    }
                }
            }
            AppEvent::RightPaneModeSelected { mode } => {
                self.state.right_pane_mode = mode;
                persist_current_conversation_workspace(&mut self.state, effects);
            }
            AppEvent::BrowserUrlChanged { url } => {
                let normalized = url.trim();
                if normalized.is_empty() {
                    return Ok(());
                }
                self.state.browser_url = Some(normalized.to_string());
                self.state.right_pane_mode = RightPaneMode::Browser;
                persist_current_conversation_workspace(&mut self.state, effects);
            }
            AppEvent::SystemAcpStarted {
                conversation_id,
                root_pid,
                pgid,
            } => {
                let agent = self
                    .state
                    .agents
                    .entry(conversation_id.clone())
                    .or_insert_with(|| AgentRuntime::new(conversation_id.clone()));
                agent.root_pid = Some(root_pid);
                agent.pgid = Some(pgid);
                agent.state = ProcessRuntimeState::Running;
                self.set_conversation_status(&conversation_id, ConversationStatus::Running)?;
            }
            AppEvent::SystemAcpSessionReady {
                conversation_id,
                cursor_session_id,
                suppress_replay: _,
            } => {
                self.state.acp_connecting.remove(&conversation_id);
                self.state.acp_replay_suppressed.remove(&conversation_id);
                self.state.steer_supported = true;
                if let Some(c) = self.state.conversations.get_mut(&conversation_id) {
                    c.cursor_session_id = Some(cursor_session_id.clone());
                    c.status = ConversationStatus::Idle;
                    c.updated_at = now_ms();
                    effects.push(EffectCommand::WriteConversation {
                        effect_id: EffectId::new(),
                        conversation: c.clone(),
                    });
                }
                if let Some(agent) = self.state.agents.get_mut(&conversation_id) {
                    agent.cursor_session_id = Some(cursor_session_id);
                    agent.state = ProcessRuntimeState::Running;
                }
                if let Some(prompt) = self
                    .state
                    .pending_connect_prompts
                    .remove(&conversation_id)
                {
                    effects.push(EffectCommand::SendAcpPrompt {
                        effect_id: EffectId::new(),
                        conversation_id: conversation_id.clone(),
                        cursor_session_id: self
                            .state
                            .conversations
                            .get(&conversation_id)
                            .and_then(|c| c.cursor_session_id.clone()),
                        text: prompt,
                    });
                    self.set_conversation_status(&conversation_id, ConversationStatus::Running)?;
                }
                resort_conversation_order(&mut self.state);
            }
            AppEvent::SystemAcpStartFailed { conversation_id } => {
                self.state.acp_connecting.remove(&conversation_id);
                self.state.acp_replay_suppressed.remove(&conversation_id);
                self.state.pending_connect_prompts.remove(&conversation_id);
                self.set_conversation_status(&conversation_id, ConversationStatus::Failed)?;
            }
            AppEvent::SystemAcpMessageReceived {
                conversation_id,
                message,
            } => {
                effects.push(EffectCommand::WriteAcpEvent {
                    effect_id: EffectId::new(),
                    conversation_id: conversation_id.clone(),
                    direction: "in".into(),
                    method: message.method.clone(),
                    raw_json: message.raw_json.clone(),
                });
                self.reduce_acp_message(conversation_id, message, effects)?;
            }
            AppEvent::SystemAcpPromptCompleted { conversation_id } => {
                self.set_conversation_status(&conversation_id, ConversationStatus::Idle)?;
                if let Some((project_id, project_path)) = self
                    .state
                    .conversations
                    .get(&conversation_id)
                    .and_then(|c| {
                        self.state
                            .projects
                            .get(&c.project_id)
                            .map(|p| (c.project_id.clone(), p.path.clone()))
                    })
                {
                    let _ = self.queue_git_refresh(&project_id, &project_path, effects);
                }
                if let Some(next) = self
                    .state
                    .prompt_queues
                    .get_mut(&conversation_id)
                    .and_then(|q| if q.is_empty() { None } else { Some(q.remove(0)) })
                {
                    let cursor_session_id = self
                        .state
                        .conversations
                        .get(&conversation_id)
                        .and_then(|c| c.cursor_session_id.clone());
                    let message = self.append_message(
                        conversation_id.clone(),
                        "user",
                        "text",
                        next.text.clone(),
                        ConversationStatus::Running,
                    )?;
                    effects.push(EffectCommand::WriteConversationMessages {
                        effect_id: EffectId::new(),
                        conversation_id: conversation_id.clone(),
                        messages: vec![message],
                    });
                    effects.push(EffectCommand::SendAcpPrompt {
                        effect_id: EffectId::new(),
                        conversation_id,
                        cursor_session_id,
                        text: next.text,
                    });
                }
            }
            AppEvent::SystemAcpSessionMetaReceived {
                conversation_id,
                payload,
            } => {
                self.apply_acp_session_payload(&conversation_id, &payload);
            }
            AppEvent::SystemAgentExited {
                conversation_id, ..
            } => {
                self.set_agent_state(&conversation_id, ProcessRuntimeState::Exited);
                self.set_conversation_status(&conversation_id, ConversationStatus::Completed)?;
            }
            AppEvent::SystemProcessSampled { sample } => {
                self.state
                    .process_samples
                    .insert(sample.conversation_id.clone(), sample.clone());
                if let Some(agent) = self.state.agents.get_mut(&sample.conversation_id) {
                    agent.latest_sample = Some(sample.clone());
                    agent.state = sample.state.clone();
                    agent.process_nodes = sample.nodes.clone();
                }
            }
            AppEvent::SystemMessageSearchResults { hits } => {
                self.state.conversation_search_hits = hits;
            }
            AppEvent::SystemFilenameIndexReady {
                project_id,
                entries,
            } => {
                self.state
                    .filename_indexes
                    .insert(project_id, entries);
            }
            AppEvent::SystemSearchResultsPartial { hits, done, .. } => {
                self.state.workspace_search_hits = hits;
                self.state.workspace_search_done = done;
            }
            AppEvent::SystemDirectoryLoaded {
                project_id: _,
                path,
                children,
            } => {
                self.state
                    .file_tree
                    .insert(normalize_workspace_path(&path), children);
                if !matches!(self.state.right_pane_mode, RightPaneMode::FilePreview) {
                    self.state.right_pane_mode = RightPaneMode::FileTree;
                }
            }
            AppEvent::SystemFileLoaded { preview } => {
                let selected = self
                    .state
                    .selected_path
                    .as_deref()
                    .map(normalize_workspace_path);
                let loaded = normalize_workspace_path(&preview.path);
                if selected.as_deref() != Some(loaded.as_str()) {
                    return Ok(());
                }
                self.state.selected_path = Some(preview.path.clone());
                self.state.selected_file = Some(preview);
                self.state.file_viewer_loading = false;
                self.state.right_pane_mode = RightPaneMode::FilePreview;
            }
            AppEvent::SystemChangedFilesComputed { project_id, files } => {
                self.state.changed_files.insert(project_id, files);
                self.state.right_pane_mode = RightPaneMode::ChangedFiles;
            }
            AppEvent::SystemWorkspaceDirty { project_id, paths } => {
                self.handle_workspace_dirty(project_id, paths, effects)?;
            }
            AppEvent::SystemGitOverlayRefreshed { project_id, overlay } => {
                self.state
                    .changed_files
                    .insert(project_id.clone(), overlay.changed_files.clone());
                self.state
                    .git_overlays
                    .insert(project_id.clone(), overlay);
                self.state.git_overlay_refreshing.insert(project_id, false);
            }
            AppEvent::SystemSessionBaseCaptured { session } => {
                self.state
                    .session_base_revisions
                    .insert(session.conversation_id.clone(), session.clone());
                effects.push(EffectCommand::WriteSessionBase {
                    effect_id: EffectId::new(),
                    session,
                });
            }
            AppEvent::SystemPrevFileLoaded { preview } => {
                let selected = self
                    .state
                    .selected_path
                    .as_deref()
                    .map(normalize_workspace_path);
                let loaded = normalize_workspace_path(&preview.path);
                if selected.as_deref() != Some(loaded.as_str()) {
                    return Ok(());
                }
                self.state
                    .prev_file_cache
                    .insert(preview.path.clone(), preview.clone());
                self.state.file_viewer_loading = false;
                self.state.right_pane_mode = RightPaneMode::FilePreview;
            }
            AppEvent::SystemStructuredDiffComputed { path, diff } => {
                let selected = self
                    .state
                    .selected_path
                    .as_deref()
                    .map(normalize_workspace_path);
                let loaded = normalize_workspace_path(&path);
                if selected.as_deref() != Some(loaded.as_str()) {
                    return Ok(());
                }
                self.state.structured_diff_cache.insert(path.clone(), diff.clone());
                self.state.structured_diff = Some(diff);
                self.state.file_viewer_loading = false;
                self.state.file_viewer_error = None;
                self.state.right_pane_mode = RightPaneMode::FilePreview;
            }
            AppEvent::SystemFileReviewLoadFailed { message } => {
                self.state.file_viewer_loading = false;
                self.state.file_viewer_error = Some(message);
            }
            AppEvent::SystemDiffComputed { diff } => {
                self.state.selected_diff = Some(diff);
                self.state.right_pane_mode = RightPaneMode::Diff;
            }
            AppEvent::SystemPreviewStatusChanged { status } => {
                self.state.active_preview_id = Some(status.preview_id.clone());
                self.state
                    .previews
                    .insert(status.preview_id.clone(), status);
                self.state.right_pane_mode = RightPaneMode::Preview;
            }
            AppEvent::SystemStorageWriteCompleted { .. } => {}
            AppEvent::SystemStorageWriteFailed { error, .. } => {
                self.push_toast("error", "Storage write failed", error)
            }
            AppEvent::SystemConversationMessagesLoaded {
                conversation_id,
                messages,
            } => {
                self.state
                    .messages
                    .insert(conversation_id.clone(), messages);
                self.state
                    .loaded_message_conversations
                    .insert(conversation_id);
            }
        }
        Ok(())
    }

    fn queue_git_refresh(
        &mut self,
        project_id: &ProjectId,
        project_path: &str,
        effects: &mut Vec<EffectCommand>,
    ) -> Result<(), EngineError> {
        if self
            .state
            .git_overlay_refreshing
            .get(project_id)
            .copied()
            .unwrap_or(false)
        {
            return Ok(());
        }
        let base_revision = base_revision_for_project(&self.state, project_id)
            .unwrap_or_else(|| "HEAD".into());
        self.state
            .git_overlay_refreshing
            .insert(project_id.clone(), true);
        effects.push(EffectCommand::RefreshGitOverlay {
            effect_id: EffectId::new(),
            project_id: project_id.clone(),
            project_path: project_path.into(),
            base_revision,
        });
        Ok(())
    }

    fn handle_workspace_dirty(
        &mut self,
        project_id: ProjectId,
        paths: Vec<String>,
        effects: &mut Vec<EffectCommand>,
    ) -> Result<(), EngineError> {
        if self.state.selected_project_id.as_ref() != Some(&project_id) {
            return Ok(());
        }
        let project = self
            .state
            .projects
            .get(&project_id)
            .cloned()
            .ok_or_else(|| EngineError::NotFound(project_id.to_string()))?;

        let mut dirs_to_reload = std::collections::BTreeSet::new();
        for path in &paths {
            for dir in expanded_parent_dirs(path) {
                if self.state.file_tree.contains_key(&dir) {
                    self.state.file_tree.remove(&dir);
                    dirs_to_reload.insert(dir);
                }
            }
        }
        for dir in dirs_to_reload {
            effects.push(EffectCommand::LoadDirectory {
                effect_id: EffectId::new(),
                project_id: project_id.clone(),
                project_path: project.path.clone(),
                path: dir,
            });
        }

        self.queue_git_refresh(&project_id, &project.path, effects)?;

        let Some(selected) = self.state.selected_path.clone() else {
            return Ok(());
        };
        let selected_norm = normalize_workspace_path(&selected);
        let selected_dirty = paths
            .iter()
            .any(|path| normalize_workspace_path(path) == selected_norm);
        if !selected_dirty {
            return Ok(());
        }

        self.invalidate_file_caches_for_path(&selected_norm);
        self.state.file_viewer_loading = true;
        self.schedule_review_load(
            project_id,
            &project.path,
            &selected_norm,
            self.state.selected_review_view,
            effects,
        )
    }

    fn invalidate_file_caches_for_path(&mut self, path: &str) {
        if self
            .state
            .selected_file
            .as_ref()
            .is_some_and(|preview| normalize_workspace_path(&preview.path) == path)
        {
            self.state.selected_file = None;
        }
        self.state.prev_file_cache.remove(path);
        self.state.structured_diff_cache.remove(path);
        self.state.structured_diff = None;
    }

    fn begin_file_view(
        &mut self,
        project_id: ProjectId,
        project_path: &str,
        path: String,
        forced_view: Option<FileReviewView>,
        effects: &mut Vec<EffectCommand>,
    ) -> Result<(), EngineError> {
        let normalized = normalize_workspace_path(&path);
        self.state.selected_project_id = Some(project_id.clone());
        let git_status = self
            .state
            .git_overlays
            .get(&project_id)
            .and_then(|overlay| overlay.entries.get(&normalized))
            .map(|entry| entry.status);
        let review_view = forced_view.unwrap_or_else(|| default_review_view(git_status));
        self.state.selected_path = Some(normalized.clone());
        self.state.selected_review_view = review_view;
        self.state.structured_diff = self.state.structured_diff_cache.get(&normalized).cloned();
        self.state.file_viewer_error = None;
        self.state.file_viewer_notice = None;
        self.state.right_pane_mode = RightPaneMode::FilePreview;

        let cached_current = self.state.selected_file.as_ref().filter(|preview| {
            normalize_workspace_path(&preview.path) == normalized
                && review_view == FileReviewView::Current
        });
        let cached_before = self.state.prev_file_cache.get(&normalized).cloned();
        let cached_inline = self.state.structured_diff_cache.get(&normalized).cloned();

        match review_view {
            FileReviewView::Current if cached_current.is_some() => {
                self.state.file_viewer_loading = false;
            }
            FileReviewView::Before if cached_before.is_some() => {
                self.state.file_viewer_loading = false;
            }
            FileReviewView::InlineChanges if cached_inline.is_some() => {
                self.state.structured_diff = cached_inline;
                self.state.file_viewer_loading = false;
            }
            _ => {
                self.state.selected_file = if review_view == FileReviewView::Current {
                    None
                } else {
                    self.state.selected_file.clone()
                };
                self.state.structured_diff = if review_view == FileReviewView::InlineChanges {
                    None
                } else {
                    self.state.structured_diff.clone()
                };
                self.state.file_viewer_loading = true;
                self.schedule_review_load(
                    project_id,
                    project_path,
                    &normalized,
                    review_view,
                    effects,
                )?;
            }
        }
        persist_current_conversation_workspace(&mut self.state, effects);
        Ok(())
    }

    fn schedule_review_load(
        &mut self,
        project_id: ProjectId,
        project_path: &str,
        path: &str,
        view: FileReviewView,
        effects: &mut Vec<EffectCommand>,
    ) -> Result<(), EngineError> {
        let overlay_entry = self
            .state
            .git_overlays
            .get(&project_id)
            .and_then(|overlay| overlay.entries.get(path).cloned());
        let base_revision = base_revision_for_project(&self.state, &project_id)
            .unwrap_or_else(|| "HEAD".into());
        match view {
            FileReviewView::Current => {
                effects.push(EffectCommand::LoadFilePreview {
                    effect_id: EffectId::new(),
                    project_id,
                    project_path: project_path.into(),
                    path: path.into(),
                });
            }
            FileReviewView::Before => {
                if matches!(
                    overlay_entry.as_ref().map(|e| e.status),
                    Some(GitFileStatus::Added | GitFileStatus::Untracked)
                ) {
                    self.state.file_viewer_loading = false;
                    self.state.file_viewer_notice = Some("No previous version".into());
                    return Ok(());
                }
                effects.push(EffectCommand::LoadPrevFile {
                    effect_id: EffectId::new(),
                    project_id,
                    project_path: project_path.into(),
                    path: path.into(),
                    base_revision,
                    old_path: overlay_entry.and_then(|e| e.old_path),
                });
            }
            FileReviewView::InlineChanges => {
                let status = self
                    .state
                    .git_overlays
                    .get(&project_id)
                    .and_then(|overlay| overlay.entries.get(path).map(|e| e.status))
                    .unwrap_or(GitFileStatus::Modified);
                effects.push(EffectCommand::ComputeStructuredDiff {
                    effect_id: EffectId::new(),
                    project_id,
                    project_path: project_path.into(),
                    path: path.into(),
                    base_revision,
                    old_path: overlay_entry.and_then(|e| e.old_path),
                    status,
                });
            }
        }
        Ok(())
    }

    fn reduce_acp_message(
        &mut self,
        conversation_id: ConversationId,
        message: AcpMessage,
        effects: &mut Vec<EffectCommand>,
    ) -> Result<(), EngineError> {
        match message.method.as_deref() {
            Some("session/request_permission") => {
                let params = message.params.unwrap_or(Value::Null);
                let (title, summary, tool_call_title, tool_kind, body) =
                    shape_permission_request(&params)?;
                let req = PermissionRequest {
                    request_id: PermissionRequestId::new(),
                    conversation_id: conversation_id.clone(),
                    acp_request_id: message.id,
                    title,
                    summary,
                    tool_call_title,
                    tool_kind,
                    body,
                    options: parse_permission_options(&params),
                    created_at: now_ms(),
                };
                self.state
                    .pending_permissions
                    .insert(req.request_id.clone(), req);
                self.set_conversation_status(
                    &conversation_id,
                    ConversationStatus::WaitingForPermission,
                )?;
            }
            Some("session/update") => {
                let params = message.params.unwrap_or(Value::Null);
                let update = params.get("update").unwrap_or(&params);
                let kind = update
                    .get("sessionUpdate")
                    .and_then(|v| v.as_str())
                    .unwrap_or("agent_message_chunk");

                match kind {
                    "session_info_update" => {
                        if let Some(title) = update.get("title").and_then(|v| v.as_str()) {
                            if let Some(c) = self.state.conversations.get_mut(&conversation_id) {
                                c.title = title.into();
                                effects.push(EffectCommand::WriteConversation {
                                    effect_id: EffectId::new(),
                                    conversation: c.clone(),
                                });
                            }
                        }
                    }
                    "available_commands_update" => {
                        if let Some(commands) =
                            update.get("availableCommands").and_then(|v| v.as_array())
                        {
                            let slash_commands = commands
                                .iter()
                                .filter_map(parse_available_command)
                                .collect::<Vec<_>>();
                            self.acp_session_mut(&conversation_id)
                                .slash_commands = slash_commands;
                        }
                    }
                    "current_mode_update" => {
                        if let Some(mode) = update
                            .get("currentModeId")
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                        {
                            self.acp_session_mut(&conversation_id).current_mode = Some(mode);
                        }
                    }
                    "config_option_update" => {
                        if let Some(options) = update
                            .get("configOptions")
                            .and_then(|v| v.as_array())
                            .or_else(|| update.get("options").and_then(|v| v.as_array()))
                        {
                            apply_config_options(
                                self.acp_session_mut(&conversation_id),
                                options,
                            );
                        }
                    }
                    "usage_update" => {}
                    "user_message_chunk" => {}
                    "agent_message_chunk" | "agent_thought_chunk" => {
                        if self.state.acp_replay_suppressed.contains(&conversation_id) {
                            return Ok(());
                        }
                        let Some(text) = extract_content_text(update) else {
                            return Ok(());
                        };
                        let (role, msg_kind) = if kind == "agent_thought_chunk" {
                            ("assistant", "thought")
                        } else {
                            ("assistant", "text")
                        };
                        let stream_key = streaming_key_for_update(update, kind);
                        self.push_stream_message(
                            conversation_id,
                            &stream_key,
                            role,
                            msg_kind,
                            text,
                            StreamMode::Append,
                            effects,
                        )?;
                    }
                    "tool_call" => {
                        if self.state.acp_replay_suppressed.contains(&conversation_id) {
                            return Ok(());
                        }
                        record_acp_edited_files(&mut self.state, &conversation_id, update, effects);
                        let tool_id = update
                            .get("toolCallId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let stream_key = format!("tool:{tool_id}");
                        self.push_stream_message(
                            conversation_id,
                            &stream_key,
                            "tool",
                            "tool",
                            format_tool_call(update),
                            StreamMode::Replace,
                            effects,
                        )?;
                    }
                    "tool_call_update" => {
                        if self.state.acp_replay_suppressed.contains(&conversation_id) {
                            return Ok(());
                        }
                        record_acp_edited_files(&mut self.state, &conversation_id, update, effects);
                        let Some(tool_id) = update.get("toolCallId").and_then(|v| v.as_str()) else {
                            return Ok(());
                        };
                        let stream_key = format!("tool:{tool_id}");
                        let text = format_tool_call_update(update);
                        if text.is_empty() {
                            return Ok(());
                        }
                        let mode = if update.get("appendContent").is_some() {
                            StreamMode::Append
                        } else {
                            StreamMode::Replace
                        };
                        self.push_stream_message(
                            conversation_id,
                            &stream_key,
                            "tool",
                            "tool",
                            text,
                            mode,
                            effects,
                        )?;
                    }
                    "plan" | "plan_update" => {
                        if self.state.acp_replay_suppressed.contains(&conversation_id) {
                            return Ok(());
                        }
                        let plan_text = format_plan(update);
                        if !plan_text.is_empty() {
                            self.state
                                .active_plan_text
                                .insert(conversation_id.clone(), plan_text.clone());
                        }
                        self.push_stream_message(
                            conversation_id,
                            "plan",
                            "assistant",
                            "plan",
                            plan_text,
                            StreamMode::Replace,
                            effects,
                        )?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn push_stream_message(
        &mut self,
        conversation_id: ConversationId,
        stream_key: &str,
        role: &str,
        kind: &str,
        text: String,
        mode: StreamMode,
        effects: &mut Vec<EffectCommand>,
    ) -> Result<(), EngineError> {
        if text.is_empty() {
            return Ok(());
        }
        let msg = self.upsert_stream_message(
            conversation_id.clone(),
            stream_key,
            role,
            kind,
            text,
            mode,
        )?;
        effects.push(EffectCommand::WriteConversationMessages {
            effect_id: EffectId::new(),
            conversation_id,
            messages: vec![msg],
        });
        Ok(())
    }

    fn clear_active_stream_keys(&mut self, conversation_id: &ConversationId) {
        let prefix = format!("{conversation_id}:");
        self.state
            .streaming_message_ids
            .retain(|key, _| !key.starts_with(&prefix));
    }

    fn acp_session_mut(&mut self, conversation_id: &ConversationId) -> &mut AcpSessionState {
        self.state
            .acp_sessions
            .entry(conversation_id.clone())
            .or_default()
    }

    fn apply_acp_session_payload(&mut self, conversation_id: &ConversationId, payload: &Value) {
        let session = self.acp_session_mut(conversation_id);
        let config_options = payload
            .get("configOptions")
            .and_then(|v| v.as_array())
            .or_else(|| payload.get("options").and_then(|v| v.as_array()));
        if let Some(options) = config_options {
            apply_config_options(session, options);
        }
        apply_models_payload(session, payload);
        if config_options.is_some() {
            if session.mode_options.is_empty() || session.model_options.is_empty() {
                apply_legacy_modes_payload(session, payload);
            }
        } else {
            apply_legacy_modes_payload(session, payload);
        }
    }

    fn append_message(
        &mut self,
        conversation_id: ConversationId,
        role: &str,
        kind: &str,
        text: String,
        status: ConversationStatus,
    ) -> Result<Message, EngineError> {
        let list = self
            .state
            .messages
            .entry(conversation_id.clone())
            .or_default();
        let ordinal = list.last().map(|m| m.ordinal + 1).unwrap_or(1);
        let msg = Message::new(conversation_id.clone(), role, kind, text.clone(), ordinal);
        list.push(msg.clone());
        let c = self
            .state
            .conversations
            .get_mut(&conversation_id)
            .ok_or_else(|| EngineError::NotFound(conversation_id.to_string()))?;
        c.status = status;
        c.message_count = list.len() as u32;
        c.updated_at = now_ms();
        c.last_message_preview = Some(text.chars().take(180).collect());
        if c.title == "New agent conversation" && role == "user" {
            c.title = text
                .split_whitespace()
                .take(8)
                .collect::<Vec<_>>()
                .join(" ");
        }
        resort_conversation_order(&mut self.state);
        Ok(msg)
    }

    fn upsert_stream_message(
        &mut self,
        conversation_id: ConversationId,
        stream_key: &str,
        role: &str,
        kind: &str,
        text: String,
        mode: StreamMode,
    ) -> Result<Message, EngineError> {
        let key = format!("{conversation_id}:{stream_key}");
        if let Some(existing) = self.state.streaming_message_ids.get(&key).cloned() {
            if let Some(list) = self.state.messages.get_mut(&conversation_id) {
                if let Some(msg) = list.iter_mut().find(|m| m.id == existing) {
                    match mode {
                        StreamMode::Append => msg.text.push_str(&text),
                        StreamMode::Replace => msg.text = text,
                    }
                    msg.updated_at = now_ms();
                    if let Some(c) = self.state.conversations.get_mut(&conversation_id) {
                        c.last_message_preview = Some(
                            msg.text.chars().take(180).collect::<String>(),
                        );
                        c.updated_at = now_ms();
                    }
                    return Ok(msg.clone());
                }
            }
        }
        let msg = self.append_message(
            conversation_id.clone(),
            role,
            kind,
            text,
            ConversationStatus::Running,
        )?;
        self.state.streaming_message_ids.insert(key, msg.id.clone());
        Ok(msg)
    }

    fn respond_permission_selection(
        &mut self,
        request_id: PermissionRequestId,
        option_id: &str,
        effects: &mut Vec<EffectCommand>,
    ) -> Result<(), EngineError> {
        let req = self
            .state
            .pending_permissions
            .remove(&request_id)
            .ok_or_else(|| EngineError::NotFound(request_id.to_string()))?;
        self.set_conversation_status(&req.conversation_id, ConversationStatus::Running)?;
        effects.push(EffectCommand::RespondAcpPermission {
            effect_id: EffectId::new(),
            conversation_id: req.conversation_id,
            acp_request_id: req.acp_request_id,
            option_id: option_id.to_string(),
        });
        Ok(())
    }

    fn set_conversation_status(
        &mut self,
        id: &ConversationId,
        status: ConversationStatus,
    ) -> Result<(), EngineError> {
        let c = self
            .state
            .conversations
            .get_mut(id)
            .ok_or_else(|| EngineError::NotFound(id.to_string()))?;
        c.status = status;
        c.updated_at = now_ms();
        resort_conversation_order(&mut self.state);
        Ok(())
    }

    fn set_agent_state(&mut self, id: &ConversationId, state: ProcessRuntimeState) {
        if let Some(agent) = self.state.agents.get_mut(id) {
            agent.state = state;
        }
    }

    fn push_toast(&mut self, level: &str, title: &str, body: String) {
        self.state.toasts.push(ToastVm {
            id: ToastId::new(),
            level: level.into(),
            title: title.into(),
            body,
        });
        if self.state.toasts.len() > 5 {
            self.state.toasts.remove(0);
        }
    }

    fn request_acp_connection(
        &mut self,
        conversation_id: &ConversationId,
        effects: &mut Vec<EffectCommand>,
    ) -> Result<(), EngineError> {
        if self.state.acp_connecting.contains(conversation_id) {
            return Ok(());
        }
        if let Some(agent) = self.state.agents.get(conversation_id) {
            if acp_is_connected_this_session(agent) {
                return Ok(());
            }
        }

        let conversation = self
            .state
            .conversations
            .get(conversation_id)
            .ok_or_else(|| EngineError::NotFound(conversation_id.to_string()))?;
        let project = self
            .state
            .projects
            .get(&conversation.project_id)
            .ok_or_else(|| EngineError::NotFound(conversation.project_id.to_string()))?;

        let resume_session_id = conversation.cursor_session_id.clone();
        if resume_session_id.is_some() {
            self.state
                .acp_replay_suppressed
                .insert(conversation_id.clone());
        }
        if let Some(c) = self.state.conversations.get_mut(conversation_id) {
            c.status = ConversationStatus::Starting;
            c.updated_at = now_ms();
        }
        if let Some(agent) = self.state.agents.get_mut(conversation_id) {
            agent.cursor_session_id = None;
            agent.root_pid = None;
            agent.pgid = None;
            agent.state = ProcessRuntimeState::Starting;
        }
        self.state.acp_sessions.remove(conversation_id);
        self.state
            .acp_connecting
            .insert(conversation_id.clone());
        effects.push(EffectCommand::StartCursorAcp {
            effect_id: EffectId::new(),
            conversation_id: conversation_id.clone(),
            project_path: project.path.clone(),
            resume_session_id,
        });
        Ok(())
    }

    fn ensure_conversation_messages_loaded(
        &mut self,
        conversation_id: &ConversationId,
        effects: &mut Vec<EffectCommand>,
    ) {
        if self
            .state
            .loaded_message_conversations
            .contains(conversation_id)
        {
            return;
        }
        effects.push(EffectCommand::LoadConversationMessages {
            effect_id: EffectId::new(),
            conversation_id: conversation_id.clone(),
        });
    }
}

fn require(ok: bool, id: &str) -> Result<(), EngineError> {
    if ok {
        Ok(())
    } else {
        Err(EngineError::NotFound(id.into()))
    }
}

fn default_permission_options() -> Vec<PermissionOption> {
    vec![
        PermissionOption {
            option_id: "allow-once".into(),
            label: "Allow once".into(),
            description: None,
            kind: Some("allow_once".into()),
        },
        PermissionOption {
            option_id: "allow-always".into(),
            label: "Allow always".into(),
            description: None,
            kind: Some("allow_always".into()),
        },
        PermissionOption {
            option_id: "reject-once".into(),
            label: "Reject".into(),
            description: None,
            kind: Some("reject_once".into()),
        },
    ]
}

fn parse_permission_options(params: &Value) -> Vec<PermissionOption> {
    let parsed = params
        .get("options")
        .and_then(|v| v.as_array())
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    let option_id = option.get("optionId").and_then(|v| v.as_str())?;
                    let label = option
                        .get("name")
                        .or_else(|| option.get("label"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(option_id)
                        .to_string();
                    Some(PermissionOption {
                        option_id: option_id.to_string(),
                        label,
                        description: option
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(ToOwned::to_owned),
                        kind: option
                            .get("kind")
                            .and_then(|v| v.as_str())
                            .map(ToOwned::to_owned),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if parsed.is_empty() {
        default_permission_options()
    } else {
        parsed
    }
}

fn shape_permission_request(
    params: &Value,
) -> Result<(String, String, Option<String>, Option<String>, String), EngineError> {
    let title = params
        .get("title")
        .or_else(|| params.get("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("Agent requests permission")
        .to_string();
    let tool_call = params.get("toolCall");
    let tool_kind = tool_call
        .and_then(|tc| tc.get("kind"))
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let tool_call_title = tool_call
        .and_then(|tc| tc.get("title"))
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let summary = extract_tool_call_reason(tool_call)
        .or(tool_call_title.clone())
        .unwrap_or_else(|| title.clone());
    let body = serde_json::to_string_pretty(params)?;
    Ok((title, summary, tool_call_title, tool_kind, body))
}

fn extract_tool_call_reason(tool_call: Option<&Value>) -> Option<String> {
    let content = tool_call?.get("content")?.as_array()?;
    for item in content {
        if let Some(text) = item
            .get("content")
            .and_then(|nested| nested.get("text"))
            .and_then(|v| v.as_str())
        {
            return Some(text.to_string());
        }
        if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
            return Some(text.to_string());
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamMode {
    Append,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ComposerSubmit {
    Prompt(String),
}

const BUILTIN_SLASH_COMMANDS: &[(&str, &str)] = &[];

fn parse_composer_input(text: &str, _acp: &AcpSessionState) -> ComposerSubmit {
    let trimmed = text.trim();
    ComposerSubmit::Prompt(trimmed.to_string())
}

fn merge_slash_commands(agent_commands: &[SlashCommandVm]) -> Vec<SlashCommandVm> {
    let mut merged = BUILTIN_SLASH_COMMANDS
        .iter()
        .map(|(name, description)| SlashCommandVm {
            name: (*name).into(),
            description: Some((*description).into()),
            hint: None,
        })
        .collect::<Vec<_>>();
    for command in agent_commands {
        if merged.iter().any(|c| c.name == command.name) {
            continue;
        }
        merged.push(command.clone());
    }
    merged
}

fn parse_available_command(value: &Value) -> Option<SlashCommandVm> {
    let name = value.get("name").and_then(|v| v.as_str())?;
    Some(SlashCommandVm {
        name: name.to_string(),
        description: value
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        hint: value
            .get("input")
            .and_then(|input| input.get("hint"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

fn is_agent_mode_id(id: &str) -> bool {
    matches!(
        id.to_ascii_lowercase().as_str(),
        "agent" | "plan" | "ask" | "debug" | "multitask" | "architect" | "code" | "chat"
    )
}

fn is_model_mode_id(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    lower.contains("composer")
        || lower.contains("gpt")
        || lower.contains("opus")
        || lower.contains("sonnet")
        || lower.contains("claude")
        || lower.contains("gemini")
        || lower.contains("haiku")
        || lower.contains("codex")
        || lower == "default"
        || lower == "auto"
}

fn apply_models_payload(session: &mut AcpSessionState, payload: &Value) {
    let Some(models_state) = payload.get("models") else {
        return;
    };
    if let Some(current) = models_state
        .get("currentModelId")
        .and_then(|v| v.as_str())
    {
        session.model_value_id = Some(current.to_string());
    }
    let Some(models) = models_state
        .get("availableModels")
        .and_then(|v| v.as_array())
    else {
        return;
    };
    let parsed = models
        .iter()
        .filter_map(parse_model_entry)
        .collect::<Vec<_>>();
    if session.model_options.is_empty() && !parsed.is_empty() {
        session.model_options = parsed;
    }
}

fn parse_model_entry(value: &Value) -> Option<ModelOptionVm> {
    let id = value
        .get("modelId")
        .or_else(|| value.get("id"))
        .or_else(|| value.get("value"))
        .and_then(|v| v.as_str())?;
    Some(ModelOptionVm {
        id: id.to_string(),
        label: value
            .get("name")
            .or_else(|| value.get("label"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format_model_display_label(id)),
        description: value
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

fn format_model_display_label(id: &str) -> String {
    if id == "default[]" || id == "default" {
        return "Auto".into();
    }
    id.split('[').next().unwrap_or(id).to_string()
}

fn apply_legacy_modes_payload(session: &mut AcpSessionState, payload: &Value) {
    let mode_state = payload
        .get("modes")
        .or_else(|| payload.get("sessionModeState"))
        .or_else(|| payload.get("modeState"));
    let current = payload
        .get("currentModeId")
        .and_then(|v| v.as_str())
        .or_else(|| mode_state.and_then(|m| m.get("currentModeId").and_then(|v| v.as_str())));
    if let Some(current) = current {
        if is_agent_mode_id(current) {
            session.current_mode = Some(current.to_string());
        } else if is_model_mode_id(current) {
            session.model_value_id = Some(current.to_string());
        }
    }
    let Some(modes) = mode_state
        .and_then(|m| m.get("availableModes"))
        .and_then(|v| v.as_array())
    else {
        return;
    };
    session.available_modes = modes
        .iter()
        .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    let mut agent_modes = Vec::new();
    let mut model_modes = Vec::new();
    for mode in modes {
        let Some(option) = parse_mode_as_option(mode) else {
            continue;
        };
        if is_agent_mode_id(&option.id) {
            agent_modes.push(option);
        } else if is_model_mode_id(&option.id) {
            model_modes.push(ModelOptionVm {
                id: option.id,
                label: option.label,
                description: option.description,
            });
        }
    }
    if session.mode_options.is_empty() && !agent_modes.is_empty() {
        session.mode_options = agent_modes;
    }
    if session.model_options.is_empty() && !model_modes.is_empty() {
        session.model_options = model_modes;
    }
}

fn parse_mode_as_option(value: &Value) -> Option<ModeOptionVm> {
    let id = value.get("id").and_then(|v| v.as_str())?;
    Some(ModeOptionVm {
        id: id.to_string(),
        label: value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(id)
            .to_string(),
        description: value
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

fn acp_is_connected_this_session(agent: &AgentRuntime) -> bool {
    agent.cursor_session_id.is_some()
        && !matches!(
            agent.state,
            ProcessRuntimeState::Exited | ProcessRuntimeState::Failed
        )
}

fn acp_status_label_for(
    conversation: Option<&Conversation>,
    agent: Option<&AgentRuntime>,
    acp_connecting: bool,
) -> String {
    let Some(_conversation) = conversation else {
        return "No conversation".into();
    };
    if agent.is_some_and(acp_is_connected_this_session) {
        return "ACP connected".into();
    }
    if acp_connecting {
        return "Connecting to ACP…".into();
    }
    "ACP disconnected".into()
}

fn default_mode_options() -> Vec<ModeOptionVm> {
    vec![
        ModeOptionVm {
            id: "agent".into(),
            label: "Agent".into(),
            description: Some("Build and edit".into()),
        },
        ModeOptionVm {
            id: "plan".into(),
            label: "Plan".into(),
            description: Some("Plan before coding".into()),
        },
        ModeOptionVm {
            id: "ask".into(),
            label: "Ask".into(),
            description: Some("Q&A without edits".into()),
        },
    ]
}

fn default_model_options() -> Vec<ModelOptionVm> {
    vec![
        ModelOptionVm {
            id: "composer-2.5".into(),
            label: "Composer 2.5".into(),
            description: None,
        },
        ModelOptionVm {
            id: "opus-4.8".into(),
            label: "Opus 4.8".into(),
            description: Some("High".into()),
        },
        ModelOptionVm {
            id: "gpt-5.5".into(),
            label: "GPT-5.5".into(),
            description: Some("Medium".into()),
        },
        ModelOptionVm {
            id: "sonnet-4.6".into(),
            label: "Sonnet 4.6".into(),
            description: Some("Medium".into()),
        },
        ModelOptionVm {
            id: "composer-2.5-fast".into(),
            label: "Composer 2.5 Fast".into(),
            description: None,
        },
        ModelOptionVm {
            id: "gpt-5.2".into(),
            label: "GPT 5.2".into(),
            description: Some("Medium".into()),
        },
    ]
}

fn config_option_id(option: &Value) -> Option<String> {
    option
        .get("id")
        .or_else(|| option.get("configId"))
        .or_else(|| option.get("optionId"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn config_option_current_value(option: &Value) -> Option<String> {
    option
        .get("currentValue")
        .or_else(|| option.get("value"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn apply_config_options(session: &mut AcpSessionState, options: &[Value]) {
    for option in options {
        let category = option
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let config_id = config_option_id(option);
        let is_mode = category == "mode"
            || config_id
                .as_deref()
                .is_some_and(|id| id.eq_ignore_ascii_case("mode"))
            || option
                .get("label")
                .and_then(|v| v.as_str())
                .is_some_and(|label| label.eq_ignore_ascii_case("mode"));
        let is_model = category == "model"
            || config_id
                .as_deref()
                .is_some_and(|id| id.eq_ignore_ascii_case("model"))
            || option
                .get("label")
                .and_then(|v| v.as_str())
                .is_some_and(|label| label.eq_ignore_ascii_case("model"));
        if is_mode {
            if let Some(id) = config_id.as_ref() {
                session.mode_config_id = Some(id.clone());
            }
            if let Some(value_id) = config_option_current_value(option) {
                session.current_mode = Some(value_id);
            }
            let parsed = extract_mode_options(option);
            if !parsed.is_empty() {
                session.mode_options = parsed;
            }
        }
        if is_model {
            if let Some(id) = config_id {
                session.model_config_id = Some(id);
            }
            if let Some(value_id) = config_option_current_value(option) {
                session.model_value_id = Some(value_id);
            }
            let parsed = extract_model_options(option);
            if !parsed.is_empty() {
                session.model_options = parsed;
            }
        }
    }
}

fn extract_mode_options(config: &Value) -> Vec<ModeOptionVm> {
    let mut out = Vec::new();
    push_mode_options(&mut out, extract_option_items(config));
    out
}

fn extract_model_options(config: &Value) -> Vec<ModelOptionVm> {
    let mut out = Vec::new();
    push_model_options(&mut out, extract_option_items(config));
    out
}

fn extract_option_items(config: &Value) -> Vec<&Value> {
    if let Some(items) = config.as_array() {
        return items.iter().collect();
    }
    let options = config.get("options");
    if let Some(items) = options.and_then(|o| o.as_array()) {
        return items.iter().collect();
    }
    if let Some(items) = options.and_then(|o| o.get("items")).and_then(|v| v.as_array()) {
        return items.iter().collect();
    }
    if let Some(groups) = options.and_then(|o| o.get("groups")).and_then(|v| v.as_array()) {
        let mut out = Vec::new();
        for group in groups {
            if let Some(items) = group.get("items").and_then(|v| v.as_array()) {
                out.extend(items.iter());
            }
        }
        if !out.is_empty() {
            return out;
        }
    }
    if let Some(values) = config.get("values").and_then(|v| v.as_array()) {
        return values.iter().collect();
    }
    Vec::new()
}

fn option_item_id(item: &Value) -> Option<&str> {
    item.get("modelId")
        .or_else(|| item.get("value"))
        .or_else(|| item.get("id"))
        .or_else(|| item.get("valueId"))
        .and_then(|v| v.as_str())
}

fn option_item_label(item: &Value, id: &str) -> String {
    item.get("label")
        .or_else(|| item.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or(id)
        .to_string()
}

fn push_mode_options(out: &mut Vec<ModeOptionVm>, items: Vec<&Value>) {
    for item in items {
        let Some(id) = option_item_id(item) else {
            continue;
        };
        out.push(ModeOptionVm {
            id: id.to_string(),
            label: option_item_label(item, id),
            description: item
                .get("description")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        });
    }
}

fn push_model_options(out: &mut Vec<ModelOptionVm>, items: Vec<&Value>) {
    for item in items {
        let Some(id) = option_item_id(item) else {
            continue;
        };
        out.push(ModelOptionVm {
            id: id.to_string(),
            label: option_item_label(item, id),
            description: item
                .get("description")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        });
    }
}

fn streaming_key_for_update(update: &Value, kind: &str) -> String {
    // Keep one bubble per chunk type per turn. Cursor may omit messageId on early
    // chunks then add it later — mixing keys split assistant output into two panes.
    match kind {
        "agent_message_chunk" | "agent_thought_chunk" => format!("active:{kind}"),
        _ => {
            if let Some(message_id) = update.get("messageId").and_then(|v| v.as_str()) {
                format!("msg:{message_id}")
            } else {
                format!("active:{kind}")
            }
        }
    }
}

fn extract_content_text(update: &Value) -> Option<String> {
    if let Some(text) = update.get("text").and_then(|v| v.as_str()) {
        return Some(text.to_string());
    }
    let content = update.get("content")?;
    match content.get("type").and_then(|v| v.as_str()) {
        Some("text") | None => content
            .get("text")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        Some("image") => content
            .get("mimeType")
            .and_then(|v| v.as_str())
            .map(|mime| format!("[image: {mime}]")),
        Some(other) => Some(format!("[{other} content]")),
    }
}

fn format_tool_call(update: &Value) -> String {
    let title = update
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Tool");
    let status = update
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("pending");
    let mut lines = vec![format!("{title} · {status}")];
    if let Some(raw) = update.get("rawInput") {
        if !raw.is_null() {
            lines.push(format!(
                "Input: {}",
                serde_json::to_string(raw).unwrap_or_else(|_| raw.to_string())
            ));
        }
    }
    lines.extend(tool_call_content_lines(update.get("content")));
    lines.join("\n")
}

fn format_tool_call_update(update: &Value) -> String {
    if let Some(append) = update.get("appendContent") {
        return tool_call_content_lines(Some(append)).join("\n");
    }
    format_tool_call(update)
}

fn tool_call_content_lines(content: Option<&Value>) -> Vec<String> {
    let Some(content) = content else {
        return Vec::new();
    };
    let blocks = if let Some(array) = content.as_array() {
        array.iter().collect::<Vec<_>>()
    } else {
        vec![content]
    };
    blocks
        .iter()
        .filter_map(|block| {
            block
                .get("text")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
                .or_else(|| {
                    block
                        .get("type")
                        .and_then(|v| v.as_str())
                        .map(|kind| format!("[{kind}]"))
                })
        })
        .collect()
}

fn acp_tool_kind_edits_files(kind: &str) -> bool {
    matches!(
        kind,
        "edit" | "write" | "delete" | "create" | "apply_patch" | "patch"
    )
}

fn should_record_acp_tool_edit(update: &Value) -> bool {
    if let Some(kind) = update.get("kind").and_then(|v| v.as_str()) {
        return acp_tool_kind_edits_files(kind);
    }
    if let Some(title) = update.get("title").and_then(|v| v.as_str()) {
        let title = title.to_lowercase();
        return title.contains("edit")
            || title.contains("write")
            || title.contains("create")
            || title.contains("delete")
            || title.contains("patch");
    }
    false
}

fn relativize_workspace_path(path: &str, project_path: &str) -> String {
    let path = normalize_workspace_path(path);
    let project = normalize_workspace_path(project_path);
    if project != "." {
        let prefix = format!("{project}/");
        if let Some(relative) = path.strip_prefix(&prefix) {
            return normalize_workspace_path(relative);
        }
        if path == project {
            return ".".into();
        }
    }
    path
}

fn push_acp_path(paths: &mut Vec<String>, path: &str, project_path: &str) {
    let relative = relativize_workspace_path(path, project_path);
    if relative == "." || paths.iter().any(|existing| existing == &relative) {
        return;
    }
    paths.push(relative);
}

fn extract_acp_tool_paths(update: &Value, project_path: &str) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(locations) = update.get("locations").and_then(|v| v.as_array()) {
        for location in locations {
            if let Some(path) = location.get("path").and_then(|v| v.as_str()) {
                push_acp_path(&mut paths, path, project_path);
            }
        }
    }
    if let Some(raw_input) = update.get("rawInput") {
        for key in [
            "path",
            "file_path",
            "filePath",
            "target_file",
            "old_path",
            "new_path",
        ] {
            if let Some(path) = raw_input.get(key).and_then(|v| v.as_str()) {
                push_acp_path(&mut paths, path, project_path);
            }
        }
    }
    paths
}

fn record_acp_edited_files(
    state: &mut AppState,
    conversation_id: &ConversationId,
    update: &Value,
    effects: &mut Vec<EffectCommand>,
) {
    if !should_record_acp_tool_edit(update) {
        return;
    }
    let project_path = state
        .conversations
        .get(conversation_id)
        .and_then(|conversation| state.projects.get(&conversation.project_id))
        .map(|project| project.path.as_str())
        .unwrap_or("");
    let paths = extract_acp_tool_paths(update, project_path);
    if paths.is_empty() {
        return;
    }
    let entry = state
        .conversation_edited_files
        .entry(conversation_id.clone())
        .or_default();
    for path in paths {
        entry.paths.retain(|existing| existing != &path);
        entry.paths.insert(0, path);
    }
    entry.paths.truncate(20);
    entry.count = entry.paths.len() as u32;
    effects.push(EffectCommand::WriteConversationEditedFiles {
        effect_id: EffectId::new(),
        conversation_id: conversation_id.clone(),
        edited_files: entry.clone(),
    });
}

fn format_plan(update: &Value) -> String {
    if let Some(markdown) = update
        .get("content")
        .or_else(|| update.get("markdown"))
        .and_then(|v| v.as_str())
    {
        if !markdown.is_empty() {
            return markdown.to_string();
        }
    }
    let Some(entries) = update.get("entries").and_then(|v| v.as_array()) else {
        return String::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let title = entry.get("title").and_then(|v| v.as_str())?;
            let status = entry
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");
            Some(format!("- [{status}] {title}"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn resort_conversation_order(state: &mut AppState) {
    state.conversation_order.sort_by(|a, b| {
        let aa = state
            .conversations
            .get(a)
            .map(|c| c.updated_at)
            .unwrap_or_default();
        let bb = state
            .conversations
            .get(b)
            .map(|c| c.updated_at)
            .unwrap_or_default();
        bb.cmp(&aa)
    });
}

fn is_agent_active(status: &ConversationStatus) -> bool {
    matches!(
        status,
        ConversationStatus::Running
            | ConversationStatus::WaitingForPermission
            | ConversationStatus::Starting
            | ConversationStatus::Throttling
            | ConversationStatus::Paused
    )
}

fn persist_current_conversation_workspace(
    state: &mut AppState,
    effects: &mut Vec<EffectCommand>,
) {
    let Some(conversation_id) = state.selected_conversation_id.clone() else {
        return;
    };
    persist_conversation_workspace(state, &conversation_id, effects);
}

fn persist_conversation_workspace(
    state: &mut AppState,
    conversation_id: &ConversationId,
    effects: &mut Vec<EffectCommand>,
) {
    let workspace = ConversationWorkspaceState {
        right_pane_mode: state.right_pane_mode.clone(),
        selected_path: state.selected_path.clone(),
        selected_review_view: state.selected_review_view,
        expanded_directories: state.file_tree.keys().cloned().collect(),
        browser_url: state.browser_url.clone(),
    };
    state
        .conversation_workspace
        .insert(conversation_id.clone(), workspace.clone());
    effects.push(EffectCommand::WriteConversationWorkspace {
        effect_id: EffectId::new(),
        conversation_id: conversation_id.clone(),
        workspace,
    });
}

fn restore_conversation_workspace(
    state: &mut AppState,
    conversation_id: &ConversationId,
    effects: &mut Vec<EffectCommand>,
) {
    let workspace = state
        .conversation_workspace
        .get(conversation_id)
        .cloned()
        .unwrap_or_default();
    state.right_pane_mode = workspace.right_pane_mode;
    state.selected_path = workspace.selected_path.clone();
    state.selected_review_view = workspace.selected_review_view;
    state.browser_url = workspace.browser_url;
    state.selected_file = None;
    state.structured_diff = None;
    state.selected_diff = None;
    state.file_viewer_loading = false;
    state.file_viewer_error = None;

    let Some(project_id) = state.selected_project_id.clone() else {
        return;
    };
    let Some(project) = state.projects.get(&project_id).cloned() else {
        return;
    };

    for dir in workspace.expanded_directories {
        if !state.file_tree.contains_key(&dir) {
            effects.push(EffectCommand::LoadDirectory {
                effect_id: EffectId::new(),
                project_id: project_id.clone(),
                project_path: project.path.clone(),
                path: dir,
            });
        }
    }
    if state.file_tree.is_empty() {
        effects.push(EffectCommand::LoadDirectory {
            effect_id: EffectId::new(),
            project_id: project_id.clone(),
            project_path: project.path.clone(),
            path: ".".into(),
        });
    }
    if let Some(path) = workspace.selected_path.clone() {
        if !matches!(state.right_pane_mode, RightPaneMode::Browser) {
            effects.push(EffectCommand::LoadFilePreview {
                effect_id: EffectId::new(),
                project_id,
                project_path: project.path,
                path,
            });
        }
    }
}

fn visible_conversation_ids(state: &AppState) -> Vec<ConversationId> {
    let query = state.search_query.trim().to_lowercase();
    let mut ids: Vec<ConversationId> = state.conversation_order.iter().cloned().collect();

    if !query.is_empty() {
        let fts_ids: BTreeSet<_> = state
            .conversation_search_hits
            .iter()
            .map(|h| h.conversation_id.clone())
            .collect();
        ids.retain(|id| {
            state
                .conversations
                .get(id)
                .map(|c| c.title.to_lowercase().contains(&query))
                .unwrap_or(false)
                || fts_ids.contains(id)
        });
        for hit in &state.conversation_search_hits {
            if !ids.contains(&hit.conversation_id) {
                ids.push(hit.conversation_id.clone());
            }
        }
    }

    ids
}

fn build_project_conversation_groups(
    state: &AppState,
    visible_ids: &[ConversationId],
) -> Vec<ProjectConversationGroupVm> {
    state
        .project_order
        .iter()
        .filter_map(|project_id| {
            let project = state.projects.get(project_id)?;
            let conversations = visible_ids
                .iter()
                .filter_map(|id| state.conversations.get(id))
                .filter(|conversation| &conversation.project_id == project_id)
                .map(|conversation| conversation_row_vm(state, conversation))
                .collect::<Vec<_>>();
            Some(ProjectConversationGroupVm {
                project: ProjectVm {
                    id: project.id.clone(),
                    name: project.name.clone(),
                    path: project.path.clone(),
                    selected: Some(project_id) == state.selected_project_id.as_ref(),
                },
                conversations,
            })
        })
        .collect()
}

fn conversation_row_vm(state: &AppState, c: &Conversation) -> ConversationRowVm {
    let agent = state.agents.get(&c.id);
    let sample = agent.and_then(|a| a.latest_sample.as_ref());
    let active = is_agent_active(&c.status);
    let edited = state
        .conversation_edited_files
        .get(&c.id)
        .cloned()
        .unwrap_or_default();
    ConversationRowVm {
        id: c.id.clone(),
        project_id: c.project_id.clone(),
        title: c.title.clone(),
        status: c.status.clone(),
        last_message_preview: c.last_message_preview.clone(),
        message_count: c.message_count,
        selected: Some(&c.id) == state.selected_conversation_id.as_ref(),
        updated_at: c.updated_at,
        acp_connected: agent.is_some_and(acp_is_connected_this_session),
        acp_connecting: state.acp_connecting.contains(&c.id),
        cpu_percent: if active {
            sample.map(|s| s.cpu_percent).unwrap_or(0.0)
        } else {
            0.0
        },
        process_state: if active {
            agent.map(|a| a.state.clone())
        } else {
            None
        },
        edited_file_count: edited.count,
        edited_file_paths: edited.paths,
    }
}

fn default_review_view(status: Option<GitFileStatus>) -> FileReviewView {
    match status {
        Some(GitFileStatus::Modified)
        | Some(GitFileStatus::Deleted)
        | Some(GitFileStatus::Renamed) => FileReviewView::InlineChanges,
        Some(GitFileStatus::Added)
        | Some(GitFileStatus::Untracked)
        | Some(GitFileStatus::Clean)
        | Some(GitFileStatus::Ignored)
        | Some(GitFileStatus::Binary)
        | None => FileReviewView::Current,
        _ => FileReviewView::InlineChanges,
    }
}

fn available_review_views(status: Option<GitFileStatus>) -> Vec<FileReviewView> {
    match status {
        Some(GitFileStatus::Added) => {
            vec![FileReviewView::Current, FileReviewView::InlineChanges]
        }
        Some(GitFileStatus::Untracked) | Some(GitFileStatus::Clean) | Some(GitFileStatus::Ignored) => {
            vec![FileReviewView::Current]
        }
        Some(GitFileStatus::Deleted) => {
            vec![FileReviewView::InlineChanges, FileReviewView::Before]
        }
        Some(GitFileStatus::Binary) => vec![FileReviewView::Current, FileReviewView::Before],
        None => vec![FileReviewView::Current],
        _ => vec![
            FileReviewView::InlineChanges,
            FileReviewView::Current,
            FileReviewView::Before,
        ],
    }
}

fn change_summary_for_entry(entry: &GitStatusEntry) -> Option<String> {
    match (entry.additions, entry.deletions) {
        (Some(additions), Some(deletions)) if additions > 0 || deletions > 0 => {
            Some(format!("+{additions} −{deletions}"))
        }
        (Some(additions), None) if additions > 0 => Some(format!("+{additions}")),
        (None, Some(deletions)) if deletions > 0 => Some(format!("−{deletions}")),
        _ => None,
    }
}

fn file_name_for_path(path: &str) -> String {
    let normalized = normalize_workspace_path(path);
    if normalized == "." {
        return normalized;
    }
    normalized
        .rsplit('/')
        .next()
        .unwrap_or(normalized.as_str())
        .to_string()
}

fn git_status_label(status: GitFileStatus) -> &'static str {
    match status {
        GitFileStatus::Clean => "Clean",
        GitFileStatus::Modified => "Modified",
        GitFileStatus::Added => "Added",
        GitFileStatus::Deleted => "Deleted",
        GitFileStatus::Renamed => "Renamed",
        GitFileStatus::Copied => "Copied",
        GitFileStatus::Untracked => "Untracked",
        GitFileStatus::Ignored => "Ignored",
        GitFileStatus::Conflicted => "Conflicted",
        GitFileStatus::TypeChanged => "Type changed",
        GitFileStatus::Binary => "Binary",
    }
}

fn group_changed_files(files: &[ChangedFile]) -> Vec<ChangedFileGroupVm> {
    let order = [
        GitFileStatus::Conflicted,
        GitFileStatus::Modified,
        GitFileStatus::Added,
        GitFileStatus::Deleted,
        GitFileStatus::Renamed,
        GitFileStatus::Untracked,
        GitFileStatus::Copied,
        GitFileStatus::TypeChanged,
        GitFileStatus::Binary,
    ];
    let mut groups = Vec::new();
    for status in order {
        let matched: Vec<ChangedFile> = files
            .iter()
            .filter(|f| f.status == status)
            .cloned()
            .collect();
        if matched.is_empty() {
            continue;
        }
        groups.push(ChangedFileGroupVm {
            status,
            label: git_status_label(status).into(),
            files: matched,
        });
    }
    groups
}

fn expanded_parent_dirs(path: &str) -> Vec<String> {
    let mut dirs = Vec::new();
    let mut current = normalize_workspace_path(path);
    loop {
        let parent = parent_dir_for_path(&current);
        dirs.push(parent.clone());
        if parent == "." {
            break;
        }
        current = parent;
    }
    dirs
}

fn parent_dir_for_path(path: &str) -> String {
    let normalized = normalize_workspace_path(path);
    if normalized == "." {
        return ".".into();
    }
    normalized
        .rsplit_once('/')
        .map(|(parent, _)| {
            if parent.is_empty() {
                ".".into()
            } else {
                parent.into()
            }
        })
        .unwrap_or_else(|| ".".into())
}

fn count_dir_changes(
    dir_path: &str,
    entries: Option<&BTreeMap<String, GitStatusEntry>>,
    synthetic: &[FileNode],
) -> u32 {
    let prefix = if dir_path == "." {
        String::new()
    } else {
        format!("{}/", normalize_workspace_path(dir_path))
    };
    let mut count = 0_u32;
    if let Some(entries) = entries {
        for (path, entry) in entries {
            if entry.status == GitFileStatus::Clean {
                continue;
            }
            let in_dir = if prefix.is_empty() {
                !path.contains('/')
            } else {
                path.starts_with(&prefix)
            };
            if in_dir {
                count += 1;
            }
        }
    }
    for node in synthetic {
        let in_dir = if prefix.is_empty() {
            !node.path.contains('/')
        } else {
            node.path.starts_with(&prefix)
        };
        if in_dir {
            count += 1;
        }
    }
    count
}

fn build_annotated_file_tree(state: &AppState) -> Vec<ExpandedDirectoryVm> {
    let overlay = state
        .selected_project_id
        .as_ref()
        .and_then(|id| state.git_overlays.get(id));
    let entries = overlay.map(|o| &o.entries);
    let synthetic = overlay
        .map(|o| o.synthetic_nodes.as_slice())
        .unwrap_or(&[]);

    state
        .file_tree
        .iter()
        .map(|(path, children)| {
            let dir = normalize_workspace_path(path);
            let mut merged = children.clone();
            for node in synthetic {
                if parent_dir_for_path(&node.path) == dir
                    && !merged.iter().any(|c| c.path == node.path)
                {
                    merged.push(node.clone());
                }
            }
            merged.sort_by(|a, b| {
                b.is_dir
                    .cmp(&a.is_dir)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            let annotated = merged
                .into_iter()
                .map(|mut node| {
                    if let Some(entries) = entries {
                        if let Some(entry) = entries.get(&node.path) {
                            node.git_status = Some(entry.status);
                        }
                    }
                    if node.is_dir {
                        node.change_count = Some(count_dir_changes(&node.path, entries, synthetic));
                    }
                    node
                })
                .collect();
            ExpandedDirectoryVm {
                path: path.clone(),
                children: annotated,
            }
        })
        .collect()
}

fn base_revision_for_project(state: &AppState, project_id: &ProjectId) -> Option<String> {
    if let Some(conv_id) = &state.selected_conversation_id {
        if let Some(session) = state.session_base_revisions.get(conv_id) {
            if &session.project_id == project_id {
                return Some(session.revision.clone());
            }
        }
    }
    state
        .git_overlays
        .get(project_id)
        .map(|overlay| overlay.base_revision.clone())
}

fn file_preview_for_vm(preview: FilePreview) -> FilePreview {
    preview
}

fn build_file_review_vm(state: &AppState) -> Option<FileReviewVm> {
    let path = state.selected_path.clone()?;
    let project_id = state.selected_project_id.as_ref()?;
    let overlay = state.git_overlays.get(project_id);
    let overlay_entry = overlay.and_then(|o| o.entries.get(&path));
    let git_status = overlay_entry.map(|e| e.status);
    let available_views = available_review_views(git_status);
    let selected_view = if available_views.contains(&state.selected_review_view) {
        state.selected_review_view
    } else {
        default_review_view(git_status)
    };
    let status_label = git_status
        .map(git_status_label)
        .unwrap_or("Current")
        .to_string();
    let change_summary = overlay_entry.and_then(change_summary_for_entry);
    let comparison_label = base_revision_for_project(state, project_id)
        .map(|_| "Compared to agent start".into());
    let context_notice = match git_status {
        Some(GitFileStatus::Added) if selected_view == FileReviewView::Current => {
            Some("New file".into())
        }
        _ => None,
    };
    let notice = state.file_viewer_notice.clone().or_else(|| {
        if selected_view == FileReviewView::Before
            && matches!(git_status, Some(GitFileStatus::Added | GitFileStatus::Untracked))
        {
            Some("No previous version".into())
        } else if selected_view == FileReviewView::Current
            && matches!(git_status, Some(GitFileStatus::Deleted))
        {
            Some("File deleted".into())
        } else {
            None
        }
    });
    let preview = match selected_view {
        FileReviewView::Current => state
            .selected_file
            .clone()
            .map(file_preview_for_vm),
        FileReviewView::Before => state
            .prev_file_cache
            .get(&path)
            .cloned()
            .map(file_preview_for_vm)
            .or_else(|| {
                if notice.is_some() {
                    None
                } else {
                    state.selected_file.clone().map(file_preview_for_vm)
                }
            }),
        FileReviewView::InlineChanges => None,
    };
    let inline_changes = if selected_view == FileReviewView::InlineChanges {
        state.structured_diff.clone()
    } else {
        None
    };
    Some(FileReviewVm {
        path: path.clone(),
        file_name: file_name_for_path(&path),
        git_status,
        status_label,
        change_summary,
        comparison_label,
        context_notice,
        selected_view,
        available_views,
        loading: state.file_viewer_loading,
        error: state.file_viewer_error.clone(),
        notice,
        preview,
        inline_changes,
    })
}

fn select_view_model(state: &AppState) -> ViewModel {
    let projects = state
        .project_order
        .iter()
        .filter_map(|id| state.projects.get(id))
        .map(|p| ProjectVm {
            id: p.id.clone(),
            name: p.name.clone(),
            path: p.path.clone(),
            selected: Some(&p.id) == state.selected_project_id.as_ref(),
        })
        .collect::<Vec<_>>();

    let visible_conversation_ids = visible_conversation_ids(state);
    let conversations = if state.conversation_list_mode == ConversationListMode::Recents {
        visible_conversation_ids
            .iter()
            .filter_map(|id| state.conversations.get(id))
            .map(|c| conversation_row_vm(state, c))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let project_groups = if state.conversation_list_mode == ConversationListMode::ByProject {
        build_project_conversation_groups(state, &visible_conversation_ids)
    } else {
        Vec::new()
    };

    let search_hits = state
        .conversation_search_hits
        .iter()
        .filter_map(|hit| {
            let title = state
                .conversations
                .get(&hit.conversation_id)
                .map(|c| c.title.clone())
                .unwrap_or_else(|| hit.conversation_id.to_string());
            Some(ConversationSearchHitVm {
                conversation_id: hit.conversation_id.clone(),
                title,
                snippet: hit.snippet.clone(),
            })
        })
        .collect::<Vec<_>>();

    let agents = state
        .agents
        .values()
        .filter(|a| {
            state
                .conversations
                .get(&a.conversation_id)
                .map(|c| is_agent_active(&c.status))
                .unwrap_or(false)
        })
        .map(|a| {
            let title = state
                .conversations
                .get(&a.conversation_id)
                .map(|c| c.title.clone())
                .unwrap_or_else(|| a.conversation_id.to_string());
            let sample = a.latest_sample.as_ref();
            AgentRowVm {
                id: a.conversation_id.clone(),
                title,
                state: a.state.clone(),
                cpu_label: format!(
                    "{:.0}% / {:.0}%",
                    sample.map(|s| s.cpu_percent).unwrap_or(0.0),
                    a.budget.max_cpu_percent
                ),
                memory_label: format_bytes(sample.map(|s| s.memory_bytes).unwrap_or(0)),
                process_label: format!(
                    "{} / {}",
                    sample.map(|s| s.process_count).unwrap_or(0),
                    a.budget.max_processes
                ),
                budget_cpu_percent: a.budget.max_cpu_percent,
            }
        })
        .collect::<Vec<_>>();

    let cpu = state
        .process_samples
        .values()
        .map(|s| s.cpu_percent)
        .sum::<f32>();
    let mem = state
        .process_samples
        .values()
        .map(|s| s.memory_bytes)
        .sum::<u64>();
    let pc = state
        .process_samples
        .values()
        .map(|s| s.process_count)
        .sum::<usize>();
    let pressure = PressureVm {
        cpu_percent: cpu,
        memory_bytes: mem,
        process_count: pc,
        label: format!(
            "CPU {:.0}% - Memory {} - Processes {}",
            cpu,
            format_bytes(mem),
            pc
        ),
    };

    let selected_conversation = state
        .selected_conversation_id
        .as_ref()
        .and_then(|id| state.conversations.get(id));
    let selected_conversation_is_active = selected_conversation
        .map(|c| is_agent_active(&c.status))
        .unwrap_or(false);
    let streaming_message_ids = state
        .selected_conversation_id
        .as_ref()
        .filter(|_| selected_conversation_is_active)
        .map(|conversation_id| {
            let prefix = format!("{conversation_id}:");
            state
                .streaming_message_ids
                .iter()
                .filter(|(key, _)| key.starts_with(&prefix))
                .map(|(_, id)| id.clone())
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();
    let conversation_messages = state
        .selected_conversation_id
        .as_ref()
        .and_then(|id| state.messages.get(id))
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|m| MessageVm {
            id: m.id.clone(),
            role: m.role,
            kind: m.kind,
            text: m.text,
            ordinal: m.ordinal,
            streaming: streaming_message_ids.contains(&m.id),
        })
        .collect::<Vec<_>>();
    let approvals = state
        .pending_permissions
        .values()
        .filter(|r| {
            state
                .selected_conversation_id
                .as_ref()
                .map(|id| &r.conversation_id == id)
                .unwrap_or(true)
        })
        .map(|r| PermissionRequestVm {
            request_id: r.request_id.clone(),
            conversation_id: r.conversation_id.clone(),
            title: r.title.clone(),
            summary: r.summary.clone(),
            tool_call_title: r.tool_call_title.clone(),
            tool_kind: r.tool_kind.clone(),
            body: r.body.clone(),
            options: r.options.clone(),
        })
        .collect::<Vec<_>>();
    let tool_status = ToolStatusVm {
        running: conversation_messages
            .iter()
            .filter(|m| m.kind == "tool" && streaming_message_ids.contains(&m.id))
            .count() as u32,
        completed: conversation_messages
            .iter()
            .filter(|m| m.kind == "tool" && !streaming_message_ids.contains(&m.id))
            .count() as u32,
    };
    let messages: Vec<MessageVm> = conversation_messages
        .into_iter()
        .filter(|m| m.kind != "tool" && m.role != "tool")
        .collect();

    let changed_files = state
        .selected_project_id
        .as_ref()
        .and_then(|id| state.git_overlays.get(id))
        .map(|overlay| overlay.changed_files.clone())
        .or_else(|| {
            state
                .selected_project_id
                .as_ref()
                .and_then(|id| state.changed_files.get(id))
                .cloned()
        })
        .unwrap_or_default();
    let changed_file_groups = group_changed_files(&changed_files);
    let git_refreshing = state
        .selected_project_id
        .as_ref()
        .and_then(|id| state.git_overlay_refreshing.get(id).copied())
        .unwrap_or(false);
    let session_base_revision = state
        .selected_project_id
        .as_ref()
        .and_then(|id| base_revision_for_project(state, id));
    let annotated_tree = build_annotated_file_tree(state);
    let preview = state
        .active_preview_id
        .as_ref()
        .and_then(|id| state.previews.get(id))
        .cloned();
    let process = state
        .selected_conversation_id
        .as_ref()
        .and_then(|id| state.agents.get(id))
        .map(|a| {
            let s = a.latest_sample.as_ref();
            let title = state
                .conversations
                .get(&a.conversation_id)
                .map(|c| c.title.clone())
                .unwrap_or_else(|| a.conversation_id.to_string());
            ProcessDetailVm {
                conversation_id: a.conversation_id.clone(),
                conversation_title: title,
                state: a.state.clone(),
                root_pid: a.root_pid,
                pgid: a.pgid,
                cpu_percent: s.map(|s| s.cpu_percent).unwrap_or(0.0),
                memory_bytes: s.map(|s| s.memory_bytes).unwrap_or(0),
                process_count: s.map(|s| s.process_count).unwrap_or(0),
                cpu_budget_percent: a.budget.max_cpu_percent,
                nodes: if !a.process_nodes.is_empty() {
                    a.process_nodes.clone()
                } else {
                    s.map(|s| s.nodes.clone()).unwrap_or_default()
                },
            }
        });

    let mut global_processes: Vec<GlobalProcessRowVm> = state
        .agents
        .values()
        .filter_map(|a| {
            let conv = state.conversations.get(&a.conversation_id)?;
            if !is_agent_active(&conv.status) {
                return None;
            }
            let s = a.latest_sample.as_ref();
            Some(GlobalProcessRowVm {
                conversation_id: a.conversation_id.clone(),
                title: conv.title.clone(),
                state: a.state.clone(),
                cpu_percent: s.map(|s| s.cpu_percent).unwrap_or(0.0),
                memory_bytes: s.map(|s| s.memory_bytes).unwrap_or(0),
                process_count: s.map(|s| s.process_count).unwrap_or(0),
                root_pid: a.root_pid,
            })
        })
        .collect();
    global_processes.sort_by(|a, b| {
        b.cpu_percent
            .partial_cmp(&a.cpu_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let agent_label = process
        .as_ref()
        .map(|p| format!("Agent {:?} CPU {:.0}%", p.state, p.cpu_percent))
        .unwrap_or_else(|| "No agent".into());
    let file_review = build_file_review_vm(state);

    ViewModel {
        left_pane: LeftPaneVm {
            projects,
            conversations,
            project_groups,
            agents,
            pressure,
            selected_project_id: state.selected_project_id.clone(),
            selected_conversation_id: state.selected_conversation_id.clone(),
            search_query: state.search_query.clone(),
            conversation_list_mode: state.conversation_list_mode.clone(),
            search_hits,
            quick_open_open: state.quick_open_open,
            workspace_search_hits: state.workspace_search_hits.clone(),
            workspace_search_done: state.workspace_search_done,
        },
        center_pane: {
            let acp = state
                .selected_conversation_id
                .as_ref()
                .and_then(|id| state.acp_sessions.get(id))
                .cloned()
                .unwrap_or_default();
            let selected_agent = state
                .selected_conversation_id
                .as_ref()
                .and_then(|id| state.agents.get(id));
            let acp_connecting = state
                .selected_conversation_id
                .as_ref()
                .is_some_and(|id| state.acp_connecting.contains(id));
            let acp_connected = selected_agent.is_some_and(acp_is_connected_this_session);
            let acp_status_label = acp_status_label_for(
                selected_conversation,
                selected_agent,
                acp_connecting,
            );
            let mode_options = if !acp.mode_options.is_empty() {
                acp.mode_options.clone()
            } else if acp_connected {
                Vec::new()
            } else {
                default_mode_options()
            };
            let model_options = if !acp.model_options.is_empty() {
                acp.model_options.clone()
            } else if acp_connected {
                Vec::new()
            } else {
                default_model_options()
            };
            let current_mode = acp
                .current_mode
                .clone()
                .or_else(|| mode_options.first().map(|mode| mode.id.clone()));
            let current_model_id = acp
                .model_value_id
                .clone()
                .or_else(|| model_options.first().map(|model| model.id.clone()));
            let current_model_label = current_model_id.as_ref().map(|id| {
                model_options
                    .iter()
                    .find(|o| &o.id == id)
                    .map(|o| o.label.clone())
                    .unwrap_or_else(|| format_model_display_label(id))
            });
            let current_mode_label = current_mode.as_ref().and_then(|id| {
                mode_options
                    .iter()
                    .find(|o| &o.id == id)
                    .map(|o| o.label.clone())
                    .or_else(|| Some(id.clone()))
            });
            let plan_visible = selected_conversation
                .map(|c| {
                    current_mode.as_deref() == Some("plan")
                        || state
                            .active_plan_text
                            .get(&c.id)
                            .map(|t| !t.is_empty())
                            .unwrap_or(false)
                })
                .unwrap_or(false);
            CenterPaneVm {
                project_name: state
                    .selected_project_id
                    .as_ref()
                    .and_then(|id| state.projects.get(id))
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Workspace".into()),
                selected_conversation_id: state.selected_conversation_id.clone(),
                title: selected_conversation
                    .map(|c| c.title.clone())
                    .unwrap_or_else(|| "Start or select a conversation".into()),
                status: selected_conversation.map(|c| c.status.clone()),
                messages,
                approvals,
                tool_status,
                composer_enabled: state.selected_conversation_id.is_some(),
                slash_commands: merge_slash_commands(&acp.slash_commands),
                mode_options,
                model_options,
                current_mode,
                current_mode_label,
                current_model_id,
                current_model_label,
                acp_connected,
                acp_status_label,
                cpu_percent: process.as_ref().map(|p| p.cpu_percent).unwrap_or(0.0),
                cpu_budget_percent: process.as_ref().map(|p| p.cpu_budget_percent).unwrap_or(0.0),
                plan_text: state
                    .selected_conversation_id
                    .as_ref()
                    .and_then(|id| state.active_plan_text.get(id).cloned()),
                plan_visible,
                queued_prompts: state
                    .selected_conversation_id
                    .as_ref()
                    .and_then(|id| state.prompt_queues.get(id).cloned())
                    .unwrap_or_default(),
                agent_running: selected_conversation
                    .map(|c| is_agent_active(&c.status))
                    .unwrap_or(false),
                steer_supported: state.steer_supported,
            }
        },
        right_pane: RightPaneVm {
            project_name: state
                .selected_project_id
                .as_ref()
                .and_then(|id| state.projects.get(id))
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "Workspace".into()),
            mode: state.right_pane_mode.clone(),
            file_tree: FileTreeVm {
                project_id: state.selected_project_id.clone(),
                expanded: annotated_tree,
                selected_path: state.selected_path.clone(),
            },
            selected_file: if file_review.is_some() {
                None
            } else {
                state
                    .selected_file
                    .clone()
                    .map(file_preview_for_vm)
            },
            file_review,
            changed_files,
            changed_file_groups,
            git_refreshing,
            session_base_revision,
            selected_diff: state.selected_diff.clone(),
            preview,
            process,
            global_processes,
            dispatch_timings: state.dispatch_timing_history.clone(),
            browser_url: state.browser_url.clone(),
        },
        status_bar: StatusBarVm {
            selected_project_label: state
                .selected_project_id
                .as_ref()
                .and_then(|id| state.projects.get(id))
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "No project".into()),
            selected_conversation_label: selected_conversation
                .map(|c| c.title.clone())
                .unwrap_or_else(|| "No conversation".into()),
            agent_label,
            storage_label: "SQLite WAL ready".into(),
        },
        modals: Vec::new(),
        toasts: state.toasts.clone(),
    }
}

pub fn event_type_name(event: &AppEvent) -> &'static str {
    match event {
        AppEvent::ProjectSelected { .. } => "projectSelected",
        AppEvent::ProjectAdded { .. } => "projectAdded",
        AppEvent::ConversationSelected { .. } => "conversationSelected",
        AppEvent::ConversationCreated { .. } => "conversationCreated",
        AppEvent::ConversationArchived { .. } => "conversationArchived",
        AppEvent::UserPromptSubmitted { .. } => "userPromptSubmitted",
        AppEvent::ComposerModeSelected { .. } => "composerModeSelected",
        AppEvent::ComposerModelSelected { .. } => "composerModelSelected",
        AppEvent::AgentPermissionApproved { .. } => "agentPermissionApproved",
        AppEvent::AgentPermissionRejected { .. } => "agentPermissionRejected",
        AppEvent::AgentPermissionSelected { .. } => "agentPermissionSelected",
        AppEvent::AgentCancelled { .. } => "agentCancelled",
        AppEvent::AgentPaused { .. } => "agentPaused",
        AppEvent::AgentResumed { .. } => "agentResumed",
        AppEvent::AgentKilled { .. } => "agentKilled",
        AppEvent::AgentCpuBudgetChanged { .. } => "agentCpuBudgetChanged",
        AppEvent::FileTreeNodeExpanded { .. } => "fileTreeNodeExpanded",
        AppEvent::FileTreeNodeCollapsed { .. } => "fileTreeNodeCollapsed",
        AppEvent::FileSelected { .. } => "fileSelected",
        AppEvent::DiffFileSelected { .. } => "diffFileSelected",
        AppEvent::ChangedFilesRefreshed { .. } => "changedFilesRefreshed",
        AppEvent::ChangedFileSelected { .. } => "changedFileSelected",
        AppEvent::ReviewViewSelected { .. } => "reviewViewSelected",
        AppEvent::GitRefreshRequested { .. } => "gitRefreshRequested",
        AppEvent::FileReviewClosed => "fileReviewClosed",
        AppEvent::PreviewOpened { .. } => "previewOpened",
        AppEvent::PreviewSuspended { .. } => "previewSuspended",
        AppEvent::PreviewClosed { .. } => "previewClosed",
        AppEvent::DevServerStarted { .. } => "devServerStarted",
        AppEvent::SearchSubmitted { .. } => "searchSubmitted",
        AppEvent::ConversationListModeSelected { .. } => "conversationListModeSelected",
        AppEvent::WorkspaceSearchSubmitted { .. } => "workspaceSearchSubmitted",
        AppEvent::WorkspaceSearchCancelled { .. } => "workspaceSearchCancelled",
        AppEvent::QuickOpenToggled { .. } => "quickOpenToggled",
        AppEvent::WorkspaceSearchResultSelected { .. } => "workspaceSearchResultSelected",
        AppEvent::QueuedPromptRemoved { .. } => "queuedPromptRemoved",
        AppEvent::QueuedPromptEdited { .. } => "queuedPromptEdited",
        AppEvent::RightPaneModeSelected { .. } => "rightPaneModeSelected",
        AppEvent::BrowserUrlChanged { .. } => "browserUrlChanged",
        AppEvent::SystemAcpStarted { .. } => "systemAcpStarted",
        AppEvent::SystemAcpSessionReady { .. } => "systemAcpSessionReady",
        AppEvent::SystemAcpStartFailed { .. } => "systemAcpStartFailed",
        AppEvent::SystemAcpMessageReceived { .. } => "systemAcpMessageReceived",
        AppEvent::SystemAcpPromptCompleted { .. } => "systemAcpPromptCompleted",
        AppEvent::SystemAcpSessionMetaReceived { .. } => "systemAcpSessionMetaReceived",
        AppEvent::SystemAgentExited { .. } => "systemAgentExited",
        AppEvent::SystemDirectoryLoaded { .. } => "systemDirectoryLoaded",
        AppEvent::SystemFileLoaded { .. } => "systemFileLoaded",
        AppEvent::SystemChangedFilesComputed { .. } => "systemChangedFilesComputed",
        AppEvent::SystemWorkspaceDirty { .. } => "systemWorkspaceDirty",
        AppEvent::SystemGitOverlayRefreshed { .. } => "systemGitOverlayRefreshed",
        AppEvent::SystemSessionBaseCaptured { .. } => "systemSessionBaseCaptured",
        AppEvent::SystemPrevFileLoaded { .. } => "systemPrevFileLoaded",
        AppEvent::SystemStructuredDiffComputed { .. } => "systemStructuredDiffComputed",
        AppEvent::SystemFileReviewLoadFailed { .. } => "systemFileReviewLoadFailed",
        AppEvent::SystemDiffComputed { .. } => "systemDiffComputed",
        AppEvent::SystemProcessSampled { .. } => "systemProcessSampled",
        AppEvent::SystemPreviewStatusChanged { .. } => "systemPreviewStatusChanged",
        AppEvent::SystemConversationMessagesLoaded { .. } => "systemConversationMessagesLoaded",
        AppEvent::SystemStorageWriteCompleted { .. } => "systemStorageWriteCompleted",
        AppEvent::SystemStorageWriteFailed { .. } => "systemStorageWriteFailed",
        AppEvent::SystemMessageSearchResults { .. } => "systemMessageSearchResults",
        AppEvent::SystemFilenameIndexReady { .. } => "systemFilenameIndexReady",
        AppEvent::SystemSearchResultsPartial { .. } => "systemSearchResultsPartial",
    }
}

pub fn effect_command_name(effect: &EffectCommand) -> &'static str {
    match effect {
        EffectCommand::StartCursorAcp { .. } => "startCursorAcp",
        EffectCommand::SendAcpPrompt { .. } => "sendAcpPrompt",
        EffectCommand::SetAcpMode { .. } => "setAcpMode",
        EffectCommand::SetAcpConfigOption { .. } => "setAcpConfigOption",
        EffectCommand::RespondAcpPermission { .. } => "respondAcpPermission",
        EffectCommand::CancelAcpSession { .. } => "cancelAcpSession",
        EffectCommand::WriteProject { .. } => "writeProject",
        EffectCommand::WriteConversation { .. } => "writeConversation",
        EffectCommand::DeleteConversation { .. } => "deleteConversation",
        EffectCommand::WriteConversationMessages { .. } => "writeConversationMessages",
        EffectCommand::LoadConversationMessages { .. } => "loadConversationMessages",
        EffectCommand::WriteAcpEvent { .. } => "writeAcpEvent",
        EffectCommand::WriteSessionBase { .. } => "writeSessionBase",
        EffectCommand::WriteConversationWorkspace { .. } => "writeConversationWorkspace",
        EffectCommand::WriteConversationEditedFiles { .. } => "writeConversationEditedFiles",
        EffectCommand::LoadDirectory { .. } => "loadDirectory",
        EffectCommand::LoadFilePreview { .. } => "loadFilePreview",
        EffectCommand::ComputeChangedFiles { .. } => "computeChangedFiles",
        EffectCommand::ComputeDiff { .. } => "computeDiff",
        EffectCommand::RefreshGitOverlay { .. } => "refreshGitOverlay",
        EffectCommand::CaptureSessionBaseRevision { .. } => "captureSessionBaseRevision",
        EffectCommand::LoadPrevFile { .. } => "loadPrevFile",
        EffectCommand::ComputeStructuredDiff { .. } => "computeStructuredDiff",
        EffectCommand::PauseProcessGroup { .. } => "pauseProcessGroup",
        EffectCommand::ResumeProcessGroup { .. } => "resumeProcessGroup",
        EffectCommand::KillProcessGroup { .. } => "killProcessGroup",
        EffectCommand::UpdateCpuBudget { .. } => "updateCpuBudget",
        EffectCommand::OpenPreview { .. } => "openPreview",
        EffectCommand::SuspendPreview { .. } => "suspendPreview",
        EffectCommand::DestroyPreview { .. } => "destroyPreview",
        EffectCommand::StartDevServer { .. } => "startDevServer",
        EffectCommand::SearchMessages { .. } => "searchMessages",
        EffectCommand::BuildFilenameIndex { .. } => "buildFilenameIndex",
        EffectCommand::SearchWorkspace { .. } => "searchWorkspace",
        EffectCommand::SteerAcpPrompt { .. } => "steerAcpPrompt",
        EffectCommand::StopAcpSession { .. } => "stopAcpSession",
        EffectCommand::UnregisterProcessGroup { .. } => "unregisterProcessGroup",
    }
}

pub fn patch_paths_for(patches: &[ViewModelPatch]) -> Vec<String> {
    patches
        .iter()
        .map(|patch| match patch {
            ViewModelPatch::Replace { path, .. } => path.clone(),
            ViewModelPatch::Remove { path } => format!("-{path}"),
        })
        .collect()
}

fn diff_view_model(
    previous: Option<&ViewModel>,
    next: &ViewModel,
) -> Result<Vec<ViewModelPatch>, EngineError> {
    let Some(prev) = previous else {
        return Ok(vec![ViewModelPatch::Replace {
            path: "".into(),
            value: serde_json::to_value(next)?,
        }]);
    };
    let mut patches = Vec::new();
    push_struct_patch_if_changed(&mut patches, "leftPane", &prev.left_pane, &next.left_pane)?;
    push_struct_patch_if_changed(&mut patches, "centerPane", &prev.center_pane, &next.center_pane)?;
    push_struct_patch_if_changed(&mut patches, "statusBar", &prev.status_bar, &next.status_bar)?;
    push_struct_patch_if_changed(&mut patches, "modals", &prev.modals, &next.modals)?;
    push_struct_patch_if_changed(&mut patches, "toasts", &prev.toasts, &next.toasts)?;
    diff_right_pane_patches(&mut patches, &prev.right_pane, &next.right_pane)?;
    Ok(patches)
}

fn push_struct_patch_if_changed<T: PartialEq + Serialize>(
    patches: &mut Vec<ViewModelPatch>,
    path: &str,
    prev: &T,
    next: &T,
) -> Result<(), EngineError> {
    if prev != next {
        patches.push(ViewModelPatch::Replace {
            path: path.into(),
            value: serde_json::to_value(next)?,
        });
    }
    Ok(())
}

fn diff_right_pane_patches(
    patches: &mut Vec<ViewModelPatch>,
    prev: &RightPaneVm,
    next: &RightPaneVm,
) -> Result<(), EngineError> {
    const KEY: &str = "rightPane";
    push_struct_patch_if_changed(patches, &format!("{KEY}.mode"), &prev.mode, &next.mode)?;
    push_struct_patch_if_changed(patches, &format!("{KEY}.fileTree"), &prev.file_tree, &next.file_tree)?;
    push_struct_patch_if_changed(patches, &format!("{KEY}.selectedFile"), &prev.selected_file, &next.selected_file)?;
    push_struct_patch_if_changed(patches, &format!("{KEY}.fileReview"), &prev.file_review, &next.file_review)?;
    push_struct_patch_if_changed(patches, &format!("{KEY}.changedFiles"), &prev.changed_files, &next.changed_files)?;
    push_struct_patch_if_changed(patches, &format!("{KEY}.changedFileGroups"), &prev.changed_file_groups, &next.changed_file_groups)?;
    push_struct_patch_if_changed(patches, &format!("{KEY}.gitRefreshing"), &prev.git_refreshing, &next.git_refreshing)?;
    push_struct_patch_if_changed(patches, &format!("{KEY}.sessionBaseRevision"), &prev.session_base_revision, &next.session_base_revision)?;
    push_struct_patch_if_changed(patches, &format!("{KEY}.selectedDiff"), &prev.selected_diff, &next.selected_diff)?;
    push_struct_patch_if_changed(patches, &format!("{KEY}.preview"), &prev.preview, &next.preview)?;
    push_struct_patch_if_changed(patches, &format!("{KEY}.process"), &prev.process, &next.process)?;
    push_struct_patch_if_changed(patches, &format!("{KEY}.dispatchTimings"), &prev.dispatch_timings, &next.dispatch_timings)?;
    Ok(())
}

fn normalize_workspace_path(path: &str) -> String {
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

pub fn format_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} GB", b / 1024.0 / 1024.0 / 1024.0)
    } else if b >= 1024.0 * 1024.0 {
        format!("{:.1} MB", b / 1024.0 / 1024.0)
    } else if b >= 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_engine_with_conversation() -> (Engine, ConversationId) {
        let mut engine = Engine::new(InitPayload {
            initial_project_path: Some("/tmp/test".into()),
        })
        .unwrap();
        let project_id = engine.state().selected_project_id.clone().unwrap();
        let vm_before = engine.previous_view_model();
        engine
            .handle_input(AppEvent::ConversationCreated { project_id })
            .unwrap();
        engine.finalize_after_effects(&vm_before).unwrap();
        let conversation_id = engine.state().selected_conversation_id.clone().unwrap();
        (engine, conversation_id)
    }

    fn acp_update(conversation_id: &ConversationId, update: Value) -> AppEvent {
        AppEvent::SystemAcpMessageReceived {
            conversation_id: conversation_id.clone(),
            message: AcpMessage::from_value(json!({
                "jsonrpc": "2.0",
                "method": "session/update",
                "params": { "update": update }
            })),
        }
    }

    #[test]
    fn agent_chunks_with_mixed_message_ids_stay_in_one_pane() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        for (text, message_id) in [
            ("Hi", None),
            (" there", Some("msg-1")),
            ("!", Some("msg-1")),
        ] {
            let mut update = json!({
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": text }
            });
            if let Some(id) = message_id {
                update["messageId"] = json!(id);
            }
            engine
                .handle_input(acp_update(&conversation_id, update))
                .unwrap();
        }
        let messages = engine.state().messages.get(&conversation_id).unwrap();
        let assistant = messages
            .iter()
            .filter(|m| m.kind == "text" && m.role == "assistant")
            .collect::<Vec<_>>();
        assert_eq!(assistant.len(), 1);
        assert_eq!(assistant[0].text, "Hi there!");
    }

    #[test]
    fn aggregates_agent_chunks_without_message_id() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        for text in ["Hi", " —", " how", " can", " I help?"] {
            engine
                .handle_input(acp_update(
                    &conversation_id,
                    json!({
                        "sessionUpdate": "agent_message_chunk",
                        "content": { "type": "text", "text": text }
                    }),
                ))
                .unwrap();
        }
        let messages = engine.state().messages.get(&conversation_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "Hi — how can I help?");
        assert_eq!(messages[0].role, "assistant");
    }

    #[test]
    fn ignores_available_commands_update() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        engine
            .handle_input(acp_update(
                &conversation_id,
                json!({
                    "sessionUpdate": "available_commands_update",
                    "availableCommands": [{
                        "name": "copy-request-id",
                        "description": "Copy the last request ID to clipboard"
                    }]
                }),
            ))
            .unwrap();
        let messages = engine.state().messages.get(&conversation_id).unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn composer_model_selection_updates_state_without_config_id() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        engine
            .handle_input(AppEvent::ComposerModelSelected {
                conversation_id: conversation_id.clone(),
                model_id: "composer-2.5-fast".into(),
            })
            .unwrap();
        let session = engine.state().acp_sessions.get(&conversation_id).unwrap();
        assert_eq!(
            session.model_value_id.as_deref(),
            Some("composer-2.5-fast")
        );
        let center = select_view_model(engine.state()).center_pane;
        assert_eq!(center.current_model_id.as_deref(), Some("composer-2.5-fast"));
    }

    #[test]
    fn archive_removes_conversation_from_view_model() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        let before = select_view_model(engine.state()).left_pane.conversations.len();
        assert_eq!(before, 1);

        let vm_before = engine.previous_view_model();
        let output = engine
            .handle_input(AppEvent::ConversationArchived {
                conversation_id: conversation_id.clone(),
            })
            .unwrap();
        assert!(!output.effects.is_empty());
        let patches = engine.finalize_after_effects(&vm_before).unwrap();
        assert!(patches.iter().any(|patch| {
            matches!(patch, ViewModelPatch::Replace { path, .. } if path == "leftPane")
        }));

        let after = select_view_model(engine.state()).left_pane.conversations.len();
        assert_eq!(after, 0);
        assert!(!engine.state().conversations.contains_key(&conversation_id));
    }

    #[test]
    fn legacy_modes_payload_routes_models_separately_from_agent_modes() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        engine
            .handle_input(AppEvent::SystemAcpSessionMetaReceived {
                conversation_id: conversation_id.clone(),
                payload: json!({
                    "sessionId": "sess_test",
                    "modes": {
                        "currentModeId": "gpt-5.2",
                        "availableModes": [
                            { "id": "agent", "name": "Agent" },
                            { "id": "plan", "name": "Plan" },
                            { "id": "gpt-5.2", "name": "GPT 5.2" },
                            { "id": "composer-2.5-fast", "name": "Composer 2.5 Fast" }
                        ]
                    }
                }),
            })
            .unwrap();
        let session = engine.state().acp_sessions.get(&conversation_id).unwrap();
        assert_eq!(session.mode_options.len(), 2);
        assert_eq!(session.model_options.len(), 2);
        assert_eq!(session.model_value_id.as_deref(), Some("gpt-5.2"));
    }

    #[test]
    fn config_options_payload_populates_mode_and_model_selectors() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        engine
            .handle_input(AppEvent::SystemAcpSessionMetaReceived {
                conversation_id: conversation_id.clone(),
                payload: json!({
                    "sessionId": "sess_test",
                    "configOptions": [
                        {
                            "type": "select",
                            "category": "mode",
                            "id": "mode",
                            "currentValue": "agent",
                            "options": {
                                "type": "flat",
                                "items": [
                                    { "value": "agent", "label": "Agent" },
                                    { "value": "plan", "label": "Plan" },
                                    { "value": "ask", "label": "Ask" }
                                ]
                            }
                        },
                        {
                            "type": "select",
                            "category": "model",
                            "id": "model",
                            "currentValue": "composer-2.5",
                            "options": {
                                "type": "flat",
                                "items": [
                                    { "value": "composer-2.5", "label": "Composer 2.5" },
                                    { "value": "gpt-5.2", "label": "GPT 5.2" }
                                ]
                            }
                        }
                    ]
                }),
            })
            .unwrap();
        let session = engine.state().acp_sessions.get(&conversation_id).unwrap();
        assert_eq!(session.mode_options.len(), 3);
        assert_eq!(session.model_options.len(), 2);
        assert_eq!(session.current_mode.as_deref(), Some("agent"));
        assert_eq!(session.model_value_id.as_deref(), Some("composer-2.5"));
    }

    #[test]
    fn cursor_cli_flat_config_options_and_models_payload() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        engine
            .handle_input(AppEvent::SystemAcpSessionMetaReceived {
                conversation_id: conversation_id.clone(),
                payload: json!({
                    "sessionId": "sess_test",
                    "modes": {
                        "currentModeId": "agent",
                        "availableModes": [
                            { "id": "agent", "name": "Agent" },
                            { "id": "plan", "name": "Plan" },
                            { "id": "ask", "name": "Ask" }
                        ]
                    },
                    "models": {
                        "currentModelId": "default[]",
                        "availableModels": [
                            { "modelId": "default[]", "name": "Auto" },
                            { "modelId": "composer-2.5[fast=true]", "name": "composer-2.5" },
                            { "modelId": "gpt-5.5[context=272k,reasoning=medium,fast=false]", "name": "gpt-5.5" }
                        ]
                    },
                    "configOptions": [
                        {
                            "id": "mode",
                            "category": "mode",
                            "currentValue": "agent",
                            "options": [
                                { "value": "agent", "name": "Agent" },
                                { "value": "plan", "name": "Plan" },
                                { "value": "ask", "name": "Ask" }
                            ]
                        },
                        {
                            "id": "model",
                            "category": "model",
                            "currentValue": "default[]",
                            "options": [
                                { "value": "default[]", "name": "Auto" },
                                { "value": "composer-2.5[fast=true]", "name": "composer-2.5" },
                                { "value": "gpt-5.5[context=272k,reasoning=medium,fast=false]", "name": "gpt-5.5" }
                            ]
                        }
                    ]
                }),
            })
            .unwrap();
        let session = engine.state().acp_sessions.get(&conversation_id).unwrap();
        assert_eq!(session.mode_options.len(), 3);
        assert_eq!(session.model_options.len(), 3);
        assert_eq!(session.model_value_id.as_deref(), Some("default[]"));
        let center = select_view_model(engine.state()).center_pane;
        assert_eq!(center.model_options.len(), 3);
        assert_eq!(center.current_model_label.as_deref(), Some("Auto"));
    }

    #[test]
    fn keeps_slash_mode_text_as_prompt() {
        let acp = AcpSessionState::default();
        match parse_composer_input("/plan refactor auth", &acp) {
            ComposerSubmit::Prompt(text) => assert_eq!(text, "/plan refactor auth"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn conversation_selected_passes_stored_session_for_resume() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        engine.state.acp_connecting.clear();
        if let Some(agent) = engine.state.agents.get_mut(&conversation_id) {
            agent.cursor_session_id = None;
            agent.state = ProcessRuntimeState::Exited;
        }
        if let Some(c) = engine.state.conversations.get_mut(&conversation_id) {
            c.cursor_session_id = Some("cursor-session-123".into());
            c.status = ConversationStatus::Idle;
        }

        let output = engine
            .handle_input(AppEvent::ConversationSelected {
                conversation_id: conversation_id.clone(),
            })
            .unwrap();

        assert!(output.effects.iter().any(|effect| {
            matches!(
                effect,
                EffectCommand::StartCursorAcp {
                    conversation_id: id,
                    resume_session_id: Some(session_id),
                    ..
                } if *id == conversation_id && session_id == "cursor-session-123"
            )
        }));
        assert!(
            engine
                .state()
                .conversations
                .get(&conversation_id)
                .unwrap()
                .cursor_session_id
                .as_deref()
                == Some("cursor-session-123")
        );
    }

    #[test]
    fn session_ready_persists_cursor_session_id() {
        let (mut engine, conversation_id) = test_engine_with_conversation();

        let output = engine
            .handle_input(AppEvent::SystemAcpSessionReady {
                conversation_id: conversation_id.clone(),
                cursor_session_id: "cursor-session-456".into(),
                suppress_replay: false,
            })
            .unwrap();

        assert!(output.effects.iter().any(|effect| {
            matches!(
                effect,
                EffectCommand::WriteConversation { conversation, .. }
                    if conversation.cursor_session_id.as_deref() == Some("cursor-session-456")
            )
        }));
    }

    #[test]
    fn conversation_selected_requests_acp_when_disconnected() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        engine.state.acp_connecting.clear();
        if let Some(agent) = engine.state.agents.get_mut(&conversation_id) {
            agent.cursor_session_id = None;
            agent.state = ProcessRuntimeState::Exited;
        }
        if let Some(c) = engine.state.conversations.get_mut(&conversation_id) {
            c.cursor_session_id = None;
            c.status = ConversationStatus::Idle;
        }

        let output = engine
            .handle_input(AppEvent::ConversationSelected {
                conversation_id: conversation_id.clone(),
            })
            .unwrap();

        assert!(output.effects.iter().any(|effect| {
            matches!(
                effect,
                EffectCommand::StartCursorAcp { conversation_id: id, .. }
                    if *id == conversation_id
            )
        }));
        assert_eq!(
            engine
                .state()
                .conversations
                .get(&conversation_id)
                .unwrap()
                .status,
            ConversationStatus::Starting
        );
    }

    #[test]
    fn hydrate_selected_conversation_does_not_request_acp() {
        let mut engine = Engine::new(InitPayload {
            initial_project_path: Some("/tmp/test".into()),
        })
        .unwrap();
        let project_id = engine.state().selected_project_id.clone().unwrap();
        let conversation = Conversation::new(project_id);
        let conversation_id = conversation.id.clone();

        let effects = engine.hydrate_conversations(vec![conversation], BTreeMap::new());

        assert!(!effects.iter().any(|effect| {
            matches!(
                effect,
                EffectCommand::StartCursorAcp { conversation_id: id, .. }
                    if *id == conversation_id
            )
        }));
        assert!(effects.iter().any(|effect| {
            matches!(
                effect,
                EffectCommand::LoadConversationMessages { conversation_id: id, .. }
                    if *id == conversation_id
            )
        }));
        assert_eq!(
            engine.state().selected_conversation_id.as_ref(),
            Some(&conversation_id)
        );
    }

    #[test]
    fn aggregates_chunks_with_message_id() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        let update = |text: &str| {
            json!({
                "sessionUpdate": "agent_message_chunk",
                "messageId": "msg-123",
                "content": { "type": "text", "text": text }
            })
        };
        engine
            .handle_input(acp_update(&conversation_id, update("Hello")))
            .unwrap();
        engine
            .handle_input(acp_update(&conversation_id, update(" world")))
            .unwrap();
        let messages = engine.state().messages.get(&conversation_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "Hello world");
    }

    #[test]
    fn late_chunks_after_prompt_completed_stay_in_same_message() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        let update = |text: &str| {
            json!({
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": text }
            })
        };
        engine
            .handle_input(acp_update(&conversation_id, update("Project status\n\n|")))
            .unwrap();
        engine
            .handle_input(AppEvent::SystemAcpPromptCompleted {
                conversation_id: conversation_id.clone(),
            })
            .unwrap();
        engine
            .handle_input(acp_update(
                &conversation_id,
                update("\nPhase\tStatus\n1 — Data\tComplete"),
            ))
            .unwrap();

        let messages = engine.state().messages.get(&conversation_id).unwrap();
        let assistant = messages
            .iter()
            .filter(|m| m.kind == "text" && m.role == "assistant")
            .collect::<Vec<_>>();
        assert_eq!(assistant.len(), 1);
        assert_eq!(
            assistant[0].text,
            "Project status\n\n|\nPhase\tStatus\n1 — Data\tComplete"
        );
    }

    #[test]
    fn steer_follow_up_is_persisted_as_user_message() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        engine.state.steer_supported = true;
        if let Some(c) = engine.state.conversations.get_mut(&conversation_id) {
            c.cursor_session_id = Some("cursor-session-test".into());
        }
        if let Some(agent) = engine.state.agents.get_mut(&conversation_id) {
            agent.cursor_session_id = Some("cursor-session-test".into());
            agent.state = ProcessRuntimeState::Running;
        }
        engine
            .set_conversation_status(&conversation_id, ConversationStatus::Running)
            .unwrap();

        let output = engine
            .handle_input(AppEvent::UserPromptSubmitted {
                conversation_id: conversation_id.clone(),
                text: "Keep going".into(),
            })
            .unwrap();

        let messages = engine.state().messages.get(&conversation_id).unwrap();
        let user_messages = messages
            .iter()
            .filter(|m| m.role == "user" && m.kind == "text")
            .collect::<Vec<_>>();
        assert_eq!(user_messages.len(), 1);
        assert_eq!(user_messages[0].text, "Keep going");
        assert!(output.effects.iter().any(|effect| {
            matches!(
                effect,
                EffectCommand::SteerAcpPrompt {
                    conversation_id: id,
                    text,
                    ..
                } if id == &conversation_id && text == "Keep going"
            )
        }));
    }

    #[test]
    fn finalize_after_effects_includes_directory_load() {
        let mut engine = Engine::new(InitPayload {
            initial_project_path: Some("/tmp/test".into()),
        })
        .unwrap();
        let project_id = engine.state().selected_project_id.clone().unwrap();
        let before = engine.previous_view_model();
        let output = engine
            .handle_input(AppEvent::FileTreeNodeExpanded {
                project_id: project_id.clone(),
                path: ".".into(),
            })
            .unwrap();
        assert!(output.effects.iter().any(|effect| {
            matches!(effect, EffectCommand::LoadDirectory { .. })
        }));
        engine
            .handle_input(AppEvent::SystemDirectoryLoaded {
                project_id,
                path: ".".into(),
                children: vec![FileNode {
                    path: "src".into(),
                    name: "src".into(),
                    is_dir: true,
                    size_bytes: None,
                    modified_at_ms: None,
                    ignored: false,
                    git_status: None,
                    change_count: None,
                    synthetic: false,
                }],
            })
            .unwrap();
        let patches = engine.finalize_after_effects(&before).unwrap();
        assert!(patches.iter().any(|patch| {
            matches!(
                patch,
                ViewModelPatch::Replace { path, .. } if path == "rightPane.fileTree"
            )
        }));
    }

    #[test]
    fn tool_status_counts_running_and_completed_tools() {
        let mut engine = Engine::new(InitPayload {
            initial_project_path: Some("/tmp/test".into()),
        })
        .unwrap();
        let project_id = engine.state().selected_project_id.clone().unwrap();
        engine
            .handle_input(AppEvent::ConversationCreated { project_id })
            .unwrap();
        let conversation_id = engine.state().selected_conversation_id.clone().unwrap();
        engine
            .handle_input(AppEvent::SystemDirectoryLoaded {
                project_id: engine.state().selected_project_id.clone().unwrap(),
                path: ".".into(),
                children: vec![],
            })
            .unwrap();
        engine
            .handle_input(acp_update(
                &conversation_id,
                json!({
                    "sessionUpdate": "tool_call",
                    "toolCallId": "tool-1",
                    "title": "Read",
                    "kind": "read",
                    "status": "in_progress"
                }),
            ))
            .unwrap();
        let center = select_view_model(engine.state()).center_pane;
        assert_eq!(center.tool_status.running, 1);
        assert_eq!(center.tool_status.completed, 0);
        engine
            .handle_input(AppEvent::SystemAcpPromptCompleted { conversation_id })
            .unwrap();
        let center = select_view_model(engine.state()).center_pane;
        assert_eq!(center.tool_status.running, 0);
        assert_eq!(center.tool_status.completed, 1);
    }

    #[test]
    fn acp_edit_tool_call_records_edited_files() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        let output = engine
            .handle_input(acp_update(
                &conversation_id,
                json!({
                    "sessionUpdate": "tool_call",
                    "toolCallId": "tool-edit-1",
                    "title": "Edit src/lib.rs",
                    "kind": "edit",
                    "status": "in_progress",
                    "locations": [{ "path": "src/lib.rs" }]
                }),
            ))
            .unwrap();

        assert!(output.effects.iter().any(|effect| {
            matches!(
                effect,
                EffectCommand::WriteConversationEditedFiles { edited_files, .. }
                    if edited_files.count == 1
                        && edited_files.paths == vec!["src/lib.rs".to_string()]
            )
        }));

        let row = select_view_model(engine.state())
            .left_pane
            .conversations
            .into_iter()
            .find(|row| row.id == conversation_id)
            .unwrap();
        assert_eq!(row.edited_file_count, 1);
        assert_eq!(row.edited_file_paths, vec!["src/lib.rs".to_string()]);
    }

    #[test]
    fn acp_read_tool_call_does_not_record_edited_files() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        engine
            .handle_input(acp_update(
                &conversation_id,
                json!({
                    "sessionUpdate": "tool_call",
                    "toolCallId": "tool-read-1",
                    "title": "Read src/lib.rs",
                    "kind": "read",
                    "status": "in_progress",
                    "locations": [{ "path": "src/lib.rs" }]
                }),
            ))
            .unwrap();

        let row = select_view_model(engine.state())
            .left_pane
            .conversations
            .into_iter()
            .find(|row| row.id == conversation_id)
            .unwrap();
        assert_eq!(row.edited_file_count, 0);
        assert!(row.edited_file_paths.is_empty());
    }

    #[test]
    fn file_tree_expand_always_reloads_directory() {
        let mut engine = Engine::new(InitPayload {
            initial_project_path: Some("/tmp/test".into()),
        })
        .unwrap();
        let project_id = engine.state().selected_project_id.clone().unwrap();
        engine
            .handle_input(AppEvent::SystemDirectoryLoaded {
                project_id: project_id.clone(),
                path: ".".into(),
                children: vec![],
            })
            .unwrap();
        let output = engine
            .handle_input(AppEvent::FileTreeNodeExpanded {
                project_id,
                path: ".".into(),
            })
            .unwrap();
        assert!(output.effects.iter().any(|effect| {
            matches!(effect, EffectCommand::LoadDirectory { .. })
        }));
    }

    fn acp_permission_request(conversation_id: &ConversationId, params: Value) -> AppEvent {
        AppEvent::SystemAcpMessageReceived {
            conversation_id: conversation_id.clone(),
            message: AcpMessage::from_value(json!({
                "jsonrpc": "2.0",
                "id": 42,
                "method": "session/request_permission",
                "params": params
            })),
        }
    }

    #[test]
    fn permission_request_uses_acp_options_and_structured_summary() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        engine
            .handle_input(acp_permission_request(
                &conversation_id,
                json!({
                    "sessionId": "sess_test",
                    "options": [
                        { "kind": "allow_once", "name": "Allow once", "optionId": "allow-once" },
                        { "kind": "allow_always", "name": "Allow always", "optionId": "allow-always" },
                        { "kind": "reject_once", "name": "Reject", "optionId": "reject-once" }
                    ],
                    "toolCall": {
                        "kind": "execute",
                        "title": "`cargo test`",
                        "content": [{
                            "type": "content",
                            "content": { "type": "text", "text": "Not in allowlist: cargo test" }
                        }]
                    }
                }),
            ))
            .unwrap();

        let approvals = select_view_model(engine.state()).center_pane.approvals;
        assert_eq!(approvals.len(), 1);
        let approval = &approvals[0];
        assert_eq!(approval.summary, "Not in allowlist: cargo test");
        assert_eq!(approval.tool_call_title.as_deref(), Some("`cargo test`"));
        assert_eq!(approval.tool_kind.as_deref(), Some("execute"));
        assert_eq!(approval.options.len(), 3);
        assert_eq!(approval.options[1].option_id, "allow-always");
        assert_eq!(approval.options[1].kind.as_deref(), Some("allow_always"));
    }

    #[test]
    fn permission_selected_sends_exact_option_id() {
        let (mut engine, conversation_id) = test_engine_with_conversation();
        engine
            .handle_input(acp_permission_request(
                &conversation_id,
                json!({
                    "options": [
                        { "kind": "allow_always", "name": "Allow always", "optionId": "allow-always" }
                    ]
                }),
            ))
            .unwrap();
        let request_id = engine
            .state()
            .pending_permissions
            .keys()
            .next()
            .cloned()
            .unwrap();

        let output = engine
            .handle_input(AppEvent::AgentPermissionSelected {
                request_id: request_id.clone(),
                option_id: "allow-always".into(),
            })
            .unwrap();

        assert!(engine.state().pending_permissions.is_empty());
        assert!(output.effects.iter().any(|effect| {
            matches!(
                effect,
                EffectCommand::RespondAcpPermission {
                    conversation_id: id,
                    option_id,
                    ..
                } if id == &conversation_id && option_id == "allow-always"
            )
        }));
    }
}
