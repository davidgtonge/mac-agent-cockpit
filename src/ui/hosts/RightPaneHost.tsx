import { useCallback, useMemo } from "preact/hooks";
import { invoke } from "@tauri-apps/api/core";
import { dispatchAppEvent } from "../../engine/ipc-client";
import { useDispatchTimingHistory } from "../../state/use-dispatch-timing-history";
import { useViewModel } from "../../state/use-view-model";
import { RightPaneView } from "../views/right-pane/RightPaneView";
import { useAppDispatch } from "./use-app-dispatch";

function joinProjectPath(projectPath: string, relativePath: string | null) {
  if (!relativePath || relativePath === ".") return projectPath;
  return `${projectPath.replace(/\/+$/, "")}/${relativePath.replace(/^\.\//, "")}`;
}

export function RightPaneHost() {
  const state = useViewModel((vm) => ({
    rightPane: vm.rightPane,
    selectedProjectPath: vm.leftPane.projects.find((project) => project.selected)?.path ?? null,
  }));
  const timings = useDispatchTimingHistory();
  const onEvent = useAppDispatch();

  const externalTarget = useMemo(() => {
    if (!state) return null;
    if (state.rightPane.preview?.url && state.rightPane.mode === "preview") {
      return state.rightPane.preview.url;
    }
    if (!state.selectedProjectPath) return null;
    return joinProjectPath(state.selectedProjectPath, state.rightPane.fileTree.selectedPath ?? null);
  }, [state]);

  const onShowPreview = useCallback(async () => {
    if (!state?.rightPane.preview) return;
    await dispatchAppEvent({ type: "rightPaneModeSelected", mode: "preview" });
  }, [state]);

  const onOpenExternally = useCallback(async () => {
    if (!externalTarget) return;
    await invoke("open_external_target", { target: externalTarget });
  }, [externalTarget]);

  const onRefreshFiles = useCallback(async () => {
    const projectId = state?.rightPane.fileTree.projectId;
    if (!projectId) return;
    const rootLoaded = state.rightPane.fileTree.expanded.some((entry) => entry.path === ".");
    if (rootLoaded) {
      await dispatchAppEvent({ type: "fileTreeNodeCollapsed", projectId, path: "." });
    }
    await dispatchAppEvent({ type: "fileTreeNodeExpanded", projectId, path: "." });
    await dispatchAppEvent({ type: "gitRefreshRequested", projectId });
  }, [state]);

  if (!state) return null;
  const rightPane = state.rightPane;

  const stats = {
    process: rightPane.mode === "process" ? (rightPane.process ?? null) : null,
    globalProcesses: rightPane.globalProcesses,
    timings,
  };

  return (
    <RightPaneView
      input={{ ...rightPane, stats }}
      onEvent={onEvent}
      onShowPreview={() => void onShowPreview()}
      canShowPreview={Boolean(rightPane.preview)}
      onOpenExternally={() => void onOpenExternally()}
      canOpenExternally={Boolean(externalTarget)}
      onRefreshFiles={() => void onRefreshFiles()}
    />
  );
}
