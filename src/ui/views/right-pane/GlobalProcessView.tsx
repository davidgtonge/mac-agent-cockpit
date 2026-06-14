import type { AppEvent, GlobalProcessRowVm } from "../../../generated/engine-types";
import { formatBytes } from "../../utils/format";

export type GlobalProcessEvent = Extract<
  AppEvent,
  { type: "conversationSelected" } | { type: "rightPaneModeSelected" }
>;

type GlobalProcessViewProps = {
  rows: GlobalProcessRowVm[];
  onEvent: (event: GlobalProcessEvent) => void;
  embedded?: boolean;
};

export function GlobalProcessView({ rows, onEvent, embedded = false }: GlobalProcessViewProps) {
  if (rows.length === 0) {
    return <div class="process-empty">No active agent processes</div>;
  }

  const list = (
    <div class="global-process-list">
      {rows.map((row) => (
        <button
          key={row.conversationId}
          type="button"
          class="global-process-row"
          onClick={() => {
            onEvent({ type: "conversationSelected", conversationId: row.conversationId });
            onEvent({ type: "rightPaneModeSelected", mode: "process" });
          }}
        >
          <span class="global-process-dot" data-state={row.state} />
          <span class="global-process-title">{row.title}</span>
          <span class="global-process-meta">
            CPU {Math.round(row.cpuPercent)}% · {formatBytes(row.memoryBytes)} · {row.processCount} proc
          </span>
        </button>
      ))}
    </div>
  );

  if (embedded) {
    return (
      <div class="global-process-view embedded mac-scrollbar" aria-label="Global processes">
        {list}
      </div>
    );
  }

  return (
    <section class="global-process-view mac-scrollbar" aria-label="Global processes">
      <header class="global-process-header">
        <b>Active processes</b>
        <span>{rows.length} agents</span>
      </header>
      {list}
    </section>
  );
}
