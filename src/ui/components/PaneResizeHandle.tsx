import type { PaneKey } from "../layout/use-pane-layout";
import { usePaneLayoutContext } from "../layout/PaneLayoutContext";

type PaneResizeHandleProps = {
  pane: PaneKey;
  class?: string;
};

export function PaneResizeHandle({ pane, class: className }: PaneResizeHandleProps) {
  const { startResize, resetPane } = usePaneLayoutContext();

  return (
    <button
      type="button"
      class={className ? `pane-resize-handle ${className}` : "pane-resize-handle"}
      aria-label={`Resize ${pane} pane`}
      onPointerDown={(event) => startResize(pane, event)}
      onDblClick={() => resetPane(pane)}
    />
  );
}
