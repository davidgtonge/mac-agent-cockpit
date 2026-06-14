import { useCallback } from "preact/hooks";
import { invoke } from "@tauri-apps/api/core";
import { useViewModel } from "../../state/use-view-model";
import { CenterPaneView } from "../views/center-pane/CenterPaneView";
import { useAppDispatch } from "./use-app-dispatch";

export function CenterPaneHost() {
  const state = useViewModel((vm) => ({
    centerPane: vm.centerPane,
    selectedProjectPath: vm.leftPane.projects.find((project) => project.selected)?.path ?? null,
  }));
  const onEvent = useAppDispatch();
  const onOpenProjectFolder = useCallback(async () => {
    if (!state?.selectedProjectPath) return;
    await invoke("open_external_target", { target: state.selectedProjectPath });
  }, [state?.selectedProjectPath]);

  if (!state) return null;
  return (
    <CenterPaneView
      input={state.centerPane}
      onEvent={onEvent}
      onOpenProjectFolder={() => void onOpenProjectFolder()}
      canOpenProjectFolder={Boolean(state.selectedProjectPath)}
    />
  );
}
