import type { AppEvent, RightPaneVm } from "../../../generated/engine-types";
import { PaneResizeHandle } from "../../components/PaneResizeHandle";
import { usePaneLayoutContext } from "../../layout/PaneLayoutContext";
import {
  ChevronDownIcon,
  ChevronRightIcon,
  DotsIcon,
  ExternalIcon,
  FileRustIcon,
  GitBranchIcon,
  GlobeIcon,
  PlusIcon,
  SplitIcon,
  TerminalIcon,
} from "../../components/icons";
import { SlickFileExplorerView } from "../file-explorer/SlickFileExplorerView";
import { ChangedFilesView } from "./ChangedFilesView";
import { DiffView } from "./DiffView";
import { FileReviewPanel } from "./FileReviewPanel";
import { PreviewView } from "./PreviewView";
import { GlobalProcessView } from "./GlobalProcessView";
import { BrowserView } from "./BrowserView";
import { StatsPanelView, type StatsPanelInput } from "./StatsPanelView";

export type RightPaneInput = RightPaneVm & {
  stats: StatsPanelInput;
};

export type RightPaneEvent = Extract<
  AppEvent,
  | { type: "changedFilesRefreshed" }
  | { type: "fileTreeNodeExpanded" }
  | { type: "fileTreeNodeCollapsed" }
  | { type: "fileSelected" }
  | { type: "diffFileSelected" }
  | { type: "changedFileSelected" }
  | { type: "reviewViewSelected" }
  | { type: "gitRefreshRequested" }
  | { type: "fileReviewClosed" }
  | { type: "previewSuspended" }
  | { type: "previewClosed" }
  | { type: "conversationSelected" }
  | { type: "rightPaneModeSelected" }
  | { type: "browserUrlChanged" }
>;

type RightPaneViewProps = {
  input: RightPaneInput;
  onEvent: (event: RightPaneEvent) => void;
  onShowPreview: () => void;
  canShowPreview: boolean;
  onOpenExternally: () => void;
  canOpenExternally: boolean;
  onRefreshFiles: () => void;
};

export function RightPaneView({
  input,
  onEvent,
  onShowPreview,
  canShowPreview,
  onOpenExternally,
  canOpenExternally,
  onRefreshFiles,
}: RightPaneViewProps) {
  const { editorWorkspaceStyle } = usePaneLayoutContext();
  const { stats, ...vm } = input;
  const projectId = vm.fileTree.projectId ?? null;
  const selectedPath = vm.fileTree.selectedPath ?? null;
  const fileReview = vm.fileReview;
  const fileName = selectedPath ? (selectedPath.split("/").pop() ?? selectedPath) : null;
  const breadcrumb = selectedPath ? selectedPath.split("/").filter(Boolean) : [];

  return (
    <section class="pane right-pane">
      <header class="editor-toolbar">
        <div class="editor-toolbar-left">
          <button
            type="button"
            class="icon-btn ghost"
            aria-label="Source control"
            onClick={() => projectId && onEvent({ type: "changedFilesRefreshed", projectId })}
          >
            <GitBranchIcon size={14} />
          </button>
          <button
            type="button"
            class={vm.mode === "fileTree" || vm.mode === "filePreview" ? "icon-btn ghost active" : "icon-btn ghost"}
            aria-label="Files"
            onClick={() => onEvent({ type: "rightPaneModeSelected", mode: "fileTree" })}
          >
            <FileRustIcon size={14} />
          </button>
          <button
            type="button"
            class={vm.mode === "browser" ? "icon-btn ghost active" : "icon-btn ghost"}
            aria-label="Browser"
            onClick={() => onEvent({ type: "rightPaneModeSelected", mode: "browser" })}
          >
            <GlobeIcon size={14} />
          </button>
          <button
            type="button"
            class={vm.mode === "preview" ? "icon-btn ghost active" : "icon-btn ghost"}
            aria-label="Dev preview"
            disabled={!canShowPreview}
            onClick={onShowPreview}
          >
            <SplitIcon size={14} />
          </button>
          <button
            type="button"
            class={vm.mode === "process" ? "icon-btn ghost active" : "icon-btn ghost"}
            aria-label="Processes"
            onClick={() => onEvent({ type: "rightPaneModeSelected", mode: "process" })}
          >
            <TerminalIcon size={14} />
          </button>
        </div>
        <div class="editor-tabs">
          {fileName && (
            <div class="editor-tab active">
              <FileRustIcon size={13} />
              <span>{fileName}</span>
            </div>
          )}
        </div>
        <div class="editor-toolbar-right">
          <button type="button" class="icon-btn ghost" aria-label="New tab">
            <PlusIcon size={14} />
          </button>
          <button
            type="button"
            class="icon-btn ghost"
            aria-label="Open externally"
            disabled={!canOpenExternally}
            onClick={onOpenExternally}
          >
            <ExternalIcon size={14} />
          </button>
          <button type="button" class="icon-btn ghost" aria-label="Split editor">
            <SplitIcon size={14} />
          </button>
          <button type="button" class="icon-btn ghost" aria-label="More">
            <DotsIcon size={14} />
          </button>
        </div>
      </header>

      <div class="editor-workspace" style={editorWorkspaceStyle}>
        <aside class="file-tree-panel mac-scrollbar">
          <div class="file-tree-header">
            <span>{vm.projectName}</span>
            <button
              type="button"
              class="icon-btn ghost tiny"
              aria-label="Refresh files"
              disabled={!projectId}
              onClick={onRefreshFiles}
            >
              <ChevronDownIcon size={12} />
            </button>
          </div>
          {vm.mode === "fileTree" || vm.mode === "filePreview" ? (
            <SlickFileExplorerView
              input={{ fileTree: vm.fileTree, projectId }}
              onEvent={onEvent}
              variant="sidebar"
            />
          ) : vm.mode === "changedFiles" ? (
            <ChangedFilesView
              input={{ groups: vm.changedFileGroups, projectId, refreshing: vm.gitRefreshing }}
              onEvent={onEvent}
            />
          ) : vm.mode === "diff" ? (
            <DiffView input={vm.selectedDiff ?? null} />
          ) : vm.mode === "preview" ? (
            <PreviewView input={{ preview: vm.preview ?? null }} onEvent={onEvent} />
          ) : vm.mode === "process" ? (
            <GlobalProcessView rows={vm.globalProcesses} onEvent={onEvent} />
          ) : null}
        </aside>

        <PaneResizeHandle pane="fileTree" class="editor-resize-handle" />

        <div class="editor-panel">
          {vm.mode === "browser" ? (
            <BrowserView url={vm.browserUrl} onEvent={onEvent} />
          ) : (
            <>
          {selectedPath && (
            <nav class="breadcrumbs" aria-label="File path">
              <span class="crumb-root">{vm.projectName}</span>
              {breadcrumb.map((part, i) => (
                <span class="crumb" key={`${part}-${i}`}>
                  <ChevronRightIcon size={10} />
                  <span>{part}</span>
                </span>
              ))}
            </nav>
          )}
          {fileReview && selectedPath ? (
            <FileReviewPanel projectId={projectId} review={fileReview} onEvent={onEvent} />
          ) : vm.mode === "diff" && vm.selectedDiff ? (
            <pre class="diff-view mac-scrollbar">{vm.selectedDiff.text || "No unstaged diff for this file."}</pre>
          ) : selectedPath && vm.selectedFile ? (
            <FileReviewPanel
              projectId={projectId}
              review={{
                path: selectedPath,
                fileName: selectedPath.split("/").pop() ?? selectedPath,
                statusLabel: "Current",
                selectedView: "current",
                availableViews: ["current"],
                loading: false,
                preview: vm.selectedFile,
              }}
              onEvent={onEvent}
            />
          ) : (
            <div class="editor-empty">
              <p>{selectedPath ? "Loading file…" : "Select a file from the tree"}</p>
            </div>
          )}
          <StatsPanelView input={stats} onEvent={onEvent} />
            </>
          )}
        </div>
      </div>
    </section>
  );
}
