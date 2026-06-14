import { useCallback, useEffect, useMemo, useRef, useState } from "preact/hooks";
import { open } from "@tauri-apps/plugin-dialog";
import { dispatchAppEvent } from "../../engine/ipc-client";
import type { RightPaneMode } from "../../generated/engine-types";
import { useViewModel } from "../../state/use-view-model";
import { LeftPaneView } from "../views/left-pane/LeftPaneView";
import { useAppDispatch } from "./use-app-dispatch";

type NavigationSnapshot = {
  projectId: string | null;
  conversationId: string | null;
  selectedPath: string | null;
  rightPaneMode: RightPaneMode;
};

function snapshotsMatch(a: NavigationSnapshot, b: NavigationSnapshot) {
  return (
    a.projectId === b.projectId &&
    a.conversationId === b.conversationId &&
    a.selectedPath === b.selectedPath &&
    a.rightPaneMode === b.rightPaneMode
  );
}

export function LeftPaneHost() {
  const state = useViewModel((vm) => ({
    leftPane: vm.leftPane,
    rightPaneMode: vm.rightPane.mode,
    selectedPath: vm.rightPane.fileTree.selectedPath ?? null,
  }));
  const onEvent = useAppDispatch();
  const historyRef = useRef<NavigationSnapshot[]>([]);
  const pendingReplayRef = useRef<NavigationSnapshot | null>(null);
  const [historyIndex, setHistoryIndex] = useState(0);

  const currentSnapshot = useMemo<NavigationSnapshot | null>(() => {
    if (!state) return null;
    return {
      projectId: state.leftPane.selectedProjectId ?? null,
      conversationId: state.leftPane.selectedConversationId ?? null,
      selectedPath: state.selectedPath,
      rightPaneMode: state.rightPaneMode,
    };
  }, [state]);

  useEffect(() => {
    if (!currentSnapshot) return;
    if (historyRef.current.length === 0) {
      historyRef.current = [currentSnapshot];
      setHistoryIndex(0);
      return;
    }
    const pending = pendingReplayRef.current;
    if (pending) {
      if (snapshotsMatch(currentSnapshot, pending)) {
        pendingReplayRef.current = null;
      }
      return;
    }
    const currentEntry = historyRef.current[historyIndex];
    if (currentEntry && snapshotsMatch(currentEntry, currentSnapshot)) return;
    const next = [...historyRef.current.slice(0, historyIndex + 1), currentSnapshot];
    historyRef.current = next;
    setHistoryIndex(next.length - 1);
  }, [currentSnapshot, historyIndex]);

  const replaySnapshot = useCallback(async (snapshot: NavigationSnapshot) => {
    const { projectId, conversationId, selectedPath, rightPaneMode } = snapshot;
    if (!projectId) return;

    pendingReplayRef.current = snapshot;

    if (conversationId) {
      await dispatchAppEvent({ type: "conversationSelected", conversationId });
    } else {
      await dispatchAppEvent({ type: "projectSelected", projectId });
    }

    switch (rightPaneMode) {
      case "filePreview":
        if (selectedPath) {
          await dispatchAppEvent({ type: "fileSelected", projectId, path: selectedPath });
        } else {
          await dispatchAppEvent({ type: "rightPaneModeSelected", mode: "fileTree" });
        }
        break;
      case "fileTree":
        await dispatchAppEvent({ type: "rightPaneModeSelected", mode: "fileTree" });
        break;
      case "changedFiles":
        await dispatchAppEvent({ type: "changedFilesRefreshed", projectId });
        break;
      case "diff":
        if (selectedPath) {
          await dispatchAppEvent({ type: "diffFileSelected", projectId, path: selectedPath });
        } else {
          await dispatchAppEvent({ type: "changedFilesRefreshed", projectId });
        }
        break;
      case "preview":
      case "process":
      case "browser":
        await dispatchAppEvent({ type: "rightPaneModeSelected", mode: rightPaneMode });
        break;
    }
  }, []);

  const onAddRepository = useCallback(async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select repository folder",
    });
    if (typeof selected === "string") {
      onEvent({ type: "projectAdded", path: selected });
    }
  }, [onEvent]);

  const goBack = useCallback(async () => {
    if (historyIndex <= 0) return;
    const nextIndex = historyIndex - 1;
    setHistoryIndex(nextIndex);
    await replaySnapshot(historyRef.current[nextIndex]);
  }, [historyIndex, replaySnapshot]);

  const goForward = useCallback(async () => {
    if (historyIndex >= historyRef.current.length - 1) return;
    const nextIndex = historyIndex + 1;
    setHistoryIndex(nextIndex);
    await replaySnapshot(historyRef.current[nextIndex]);
  }, [historyIndex, replaySnapshot]);

  if (!state) return null;
  return (
    <LeftPaneView
      input={state.leftPane}
      onEvent={onEvent}
      onAddRepository={onAddRepository}
      canGoBack={historyIndex > 0}
      canGoForward={historyIndex < historyRef.current.length - 1}
      onGoBack={() => void goBack()}
      onGoForward={() => void goForward()}
    />
  );
}
