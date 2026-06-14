export type ProjectId = string;
export type ConversationId = string;
export type MessageId = string;
export type PermissionRequestId = string;
export type PreviewId = string;
export type EffectId = string;
export type ToastId = string;

export type ConversationStatus = "idle" | "starting" | "running" | "waitingForPermission" | "paused" | "throttling" | "completed" | "cancelled" | "failed";
export type ProcessRuntimeState = "starting" | "running" | "paused" | "throttling" | "exited" | "failed";
export type PreviewRuntimeState = "opening" | "open" | "suspended" | "destroyed" | "failed";
export type RightPaneMode = "fileTree" | "filePreview" | "changedFiles" | "diff" | "preview" | "process" | "browser";
export type ConversationListMode = "recents" | "byProject";
export type WorkspaceSearchMode = "both" | "filename" | "content";

export type ProjectVm = { id: ProjectId; name: string; path: string; selected: boolean };
export type ProjectConversationGroupVm = { project: ProjectVm; conversations: ConversationRowVm[] };
export type ConversationRowVm = {
  id: ConversationId;
  projectId: ProjectId;
  title: string;
  status: ConversationStatus;
  lastMessagePreview?: string | null;
  messageCount: number;
  selected: boolean;
  updatedAt: number;
  acpConnected: boolean;
  acpConnecting: boolean;
  cpuPercent: number;
  processState?: ProcessRuntimeState | null;
  editedFileCount: number;
  editedFilePaths: string[];
};
export type ConversationSearchHitVm = { conversationId: ConversationId; title: string; snippet: string };
export type AgentRowVm = { id: ConversationId; title: string; state: ProcessRuntimeState; cpuLabel: string; memoryLabel: string; processLabel: string; budgetCpuPercent: number };
export type PressureVm = { cpuPercent: number; memoryBytes: number; processCount: number; label: string };
export type LeftPaneVm = {
  projects: ProjectVm[];
  conversations: ConversationRowVm[];
  projectGroups: ProjectConversationGroupVm[];
  agents: AgentRowVm[];
  pressure: PressureVm;
  selectedProjectId?: ProjectId | null;
  selectedConversationId?: ConversationId | null;
  searchQuery: string;
  conversationListMode: ConversationListMode;
  searchHits: ConversationSearchHitVm[];
  quickOpenOpen: boolean;
  workspaceSearchHits: WorkspaceSearchHit[];
  workspaceSearchDone: boolean;
};

export type MessageVm = { id: MessageId; role: string; kind: string; text: string; ordinal: number; streaming: boolean };
export type PermissionOption = {
  optionId: string;
  label: string;
  description?: string | null;
  kind?: string | null;
};
export type PermissionRequestVm = {
  requestId: PermissionRequestId;
  conversationId: ConversationId;
  title: string;
  summary: string;
  toolCallTitle?: string | null;
  toolKind?: string | null;
  body: string;
  options: PermissionOption[];
};
export type ToolStatusVm = { running: number; completed: number };
export type SlashCommandVm = { name: string; description?: string | null; hint?: string | null };
export type ModeOptionVm = { id: string; label: string; description?: string | null };
export type ModelOptionVm = { id: string; label: string; description?: string | null };
export type QueuedPrompt = { id: string; text: string };
export type CenterPaneVm = {
  projectName: string;
  selectedConversationId?: ConversationId | null;
  title: string;
  status?: ConversationStatus | null;
  messages: MessageVm[];
  approvals: PermissionRequestVm[];
  toolStatus: ToolStatusVm;
  composerEnabled: boolean;
  slashCommands: SlashCommandVm[];
  modeOptions: ModeOptionVm[];
  modelOptions: ModelOptionVm[];
  currentMode?: string | null;
  currentModeLabel?: string | null;
  currentModelId?: string | null;
  currentModelLabel?: string | null;
  acpConnected: boolean;
  acpStatusLabel: string;
  cpuPercent: number;
  cpuBudgetPercent: number;
  planText?: string | null;
  planVisible: boolean;
  queuedPrompts: QueuedPrompt[];
  agentRunning: boolean;
  steerSupported: boolean;
};

export type GitFileStatus = "clean" | "modified" | "added" | "deleted" | "renamed" | "copied" | "untracked" | "ignored" | "conflicted" | "typeChanged" | "binary";
export type FileReviewView = "inlineChanges" | "current" | "before";
export type DiffRowKind = "context" | "added" | "removed" | "hunkHeader" | "fileHeader" | "notice";
export type FileNode = { path: string; name: string; isDir: boolean; sizeBytes?: number | null; modifiedAtMs?: number | null; ignored: boolean; gitStatus?: GitFileStatus | null; changeCount?: number | null; synthetic?: boolean };
export type FilePreview = { projectId: ProjectId; path: string; text?: string | null; highlightedLines?: string[] | null; binary: boolean; truncated: boolean; sizeBytes: number; languageHint?: string | null };
export type ChangedFile = { path: string; status: GitFileStatus; oldPath?: string | null; additions?: number | null; deletions?: number | null };
export type DiffRowVm = { kind: DiffRowKind; oldLine?: number | null; newLine?: number | null; highlightedHtml: string };
export type DiffHunkVm = { header: string; rows: DiffRowVm[] };
export type StructuredDiffVm = { oldPath?: string | null; newPath?: string | null; status: GitFileStatus; stat: string; hunks: DiffHunkVm[] };
export type ChangedFileGroupVm = { status: GitFileStatus; label: string; files: ChangedFile[] };
export type FileReviewViewModel = {
  path: string;
  fileName: string;
  gitStatus?: GitFileStatus | null;
  statusLabel: string;
  changeSummary?: string | null;
  comparisonLabel?: string | null;
  contextNotice?: string | null;
  selectedView: FileReviewView;
  availableViews: FileReviewView[];
  loading: boolean;
  error?: string | null;
  notice?: string | null;
  preview?: FilePreview | null;
  inlineChanges?: StructuredDiffVm | null;
};
export type DiffResult = { projectId: ProjectId; path?: string | null; stat: string; text: string; generatedAtMs: number };
export type PreviewStatus = { previewId: PreviewId; projectId: ProjectId; url: string; state: PreviewRuntimeState; devServerPid?: number | null; lastDetectedPort?: number | null };
export type ExpandedDirectoryVm = { path: string; children: FileNode[] };
export type FileTreeVm = { projectId?: ProjectId | null; expanded: ExpandedDirectoryVm[]; selectedPath?: string | null };
export type ProcessNodeVm = { pid: number; ppid: number; cpuPercent: number; memoryBytes: number; command?: string | null };
export type ProcessDetailVm = {
  conversationId: ConversationId;
  conversationTitle: string;
  state: ProcessRuntimeState;
  rootPid?: number | null;
  pgid?: number | null;
  cpuPercent: number;
  memoryBytes: number;
  processCount: number;
  cpuBudgetPercent: number;
  nodes: ProcessNodeVm[];
};
export type GlobalProcessRowVm = {
  conversationId: ConversationId;
  title: string;
  state: ProcessRuntimeState;
  cpuPercent: number;
  memoryBytes: number;
  processCount: number;
  rootPid?: number | null;
};
export type WorkspaceSearchHit = { path: string; line?: number | null; column?: number | null; snippet: string; kind: string };
export type EffectTimingVm = { name: string; durationMs: number };
export type DispatchTimingVm = { event: string; reduceMs: number; initialPatchMs: number; effects: EffectTimingVm[]; drainIoMs: number; finalizePatchMs: number; responsePrepMs: number; patchCount: number; patchPaths: string[]; serverTotalMs: number };
export type RightPaneVm = {
  projectName: string;
  mode: RightPaneMode;
  fileTree: FileTreeVm;
  selectedFile?: FilePreview | null;
  fileReview?: FileReviewViewModel | null;
  changedFiles: ChangedFile[];
  changedFileGroups: ChangedFileGroupVm[];
  gitRefreshing: boolean;
  sessionBaseRevision?: string | null;
  selectedDiff?: DiffResult | null;
  preview?: PreviewStatus | null;
  process?: ProcessDetailVm | null;
  globalProcesses: GlobalProcessRowVm[];
  dispatchTimings: DispatchTimingVm[];
  browserUrl?: string | null;
};

export type StatusBarVm = { selectedProjectLabel: string; selectedConversationLabel: string; agentLabel: string; storageLabel: string };
export type ModalVm = { id: string; title: string; body: string };
export type ToastVm = { id: ToastId; level: string; title: string; body: string };
export type ViewModel = { leftPane: LeftPaneVm; centerPane: CenterPaneVm; rightPane: RightPaneVm; statusBar: StatusBarVm; modals: ModalVm[]; toasts: ToastVm[] };

export type ViewModelPatch = { op: "replace"; path: string; value: unknown } | { op: "remove"; path: string };
export type Diagnostic = { level: string; message: string };
export type EngineResponse = { patches: ViewModelPatch[]; effects: unknown[]; diagnostics: Diagnostic[] };

export type AppEvent =
  | { type: "projectSelected"; projectId: ProjectId }
  | { type: "projectAdded"; path: string }
  | { type: "conversationSelected"; conversationId: ConversationId }
  | { type: "conversationCreated"; projectId: ProjectId }
  | { type: "conversationArchived"; conversationId: ConversationId }
  | { type: "userPromptSubmitted"; conversationId: ConversationId; text: string }
  | { type: "composerModeSelected"; conversationId: ConversationId; modeId: string }
  | { type: "composerModelSelected"; conversationId: ConversationId; modelId: string }
  | { type: "agentPermissionApproved"; requestId: PermissionRequestId }
  | { type: "agentPermissionRejected"; requestId: PermissionRequestId }
  | { type: "agentPermissionSelected"; requestId: PermissionRequestId; optionId: string }
  | { type: "agentCancelled"; conversationId: ConversationId }
  | { type: "agentPaused"; conversationId: ConversationId }
  | { type: "agentResumed"; conversationId: ConversationId }
  | { type: "agentKilled"; conversationId: ConversationId }
  | { type: "agentCpuBudgetChanged"; conversationId: ConversationId; cpuPercent: number }
  | { type: "fileTreeNodeExpanded"; projectId: ProjectId; path: string }
  | { type: "fileTreeNodeCollapsed"; projectId: ProjectId; path: string }
  | { type: "fileSelected"; projectId: ProjectId; path: string }
  | { type: "diffFileSelected"; projectId: ProjectId; path: string }
  | { type: "changedFileSelected"; projectId: ProjectId; path: string }
  | { type: "reviewViewSelected"; projectId: ProjectId; path: string; view: FileReviewView }
  | { type: "gitRefreshRequested"; projectId: ProjectId }
  | { type: "fileReviewClosed" }
  | { type: "changedFilesRefreshed"; projectId: ProjectId }
  | { type: "previewOpened"; projectId: ProjectId; url: string }
  | { type: "previewSuspended"; previewId: PreviewId }
  | { type: "previewClosed"; previewId: PreviewId }
  | { type: "devServerStarted"; projectId: ProjectId; command: string; args: string[] }
  | { type: "searchSubmitted"; query: string }
  | { type: "conversationListModeSelected"; mode: ConversationListMode }
  | { type: "workspaceSearchSubmitted"; projectId: ProjectId; query: string; mode: WorkspaceSearchMode }
  | { type: "workspaceSearchCancelled" }
  | { type: "quickOpenToggled"; open: boolean }
  | { type: "workspaceSearchResultSelected"; projectId: ProjectId; path: string }
  | { type: "queuedPromptRemoved"; conversationId: ConversationId; promptId: string }
  | { type: "queuedPromptEdited"; conversationId: ConversationId; promptId: string; text: string }
  | { type: "rightPaneModeSelected"; mode: RightPaneMode }
  | { type: "browserUrlChanged"; url: string };
