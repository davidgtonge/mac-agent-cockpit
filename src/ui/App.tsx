import { useEffect } from "preact/hooks";
import { useViewModel } from "../state/use-view-model";
import { PaneResizeHandle } from "./components/PaneResizeHandle";
import { CenterPaneHost } from "./hosts/CenterPaneHost";
import { LeftPaneHost } from "./hosts/LeftPaneHost";
import { RightPaneHost } from "./hosts/RightPaneHost";
import { ToastsHost } from "./hosts/ToastsHost";
import { useAppDispatch } from "./hosts/use-app-dispatch";
import { PaneLayoutProvider, usePaneLayoutContext } from "./layout/PaneLayoutContext";
import { QuickOpenView } from "./views/quick-open/QuickOpenView";

function AppShell() {
  const leftPane = useViewModel((vm) => vm.leftPane);
  const onEvent = useAppDispatch();
  const { shellStyle } = usePaneLayoutContext();

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "p") {
        e.preventDefault();
        onEvent({ type: "quickOpenToggled", open: true });
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onEvent]);

  return (
    <main class="app-shell" style={shellStyle}>
      <LeftPaneHost />
      <PaneResizeHandle pane="left" />
      <CenterPaneHost />
      <PaneResizeHandle pane="center" />
      <RightPaneHost />
      <ToastsHost />
      {leftPane && (
        <QuickOpenView
          input={{
            quickOpenOpen: leftPane.quickOpenOpen,
            workspaceSearchHits: leftPane.workspaceSearchHits,
            workspaceSearchDone: leftPane.workspaceSearchDone,
            selectedProjectId: leftPane.selectedProjectId,
            projects: leftPane.projects,
          }}
          onEvent={onEvent}
        />
      )}
    </main>
  );
}

export function App() {
  return (
    <PaneLayoutProvider>
      <AppShell />
    </PaneLayoutProvider>
  );
}
