import type { GlobalProcessRowVm, ProcessDetailVm } from "../../../generated/engine-types";
import { CollapsiblePanel } from "../../components/CollapsiblePanel";
import { formatMs } from "../../utils/format";
import { GlobalProcessView } from "./GlobalProcessView";
import type { GlobalProcessEvent } from "./GlobalProcessView";
import type { DispatchTimingRecord } from "../../../state/dispatch-timing-overlay";
import { DispatchTimingView } from "./DispatchTimingView";
import { ProcessView } from "./ProcessView";

export type StatsPanelInput = {
  process: ProcessDetailVm | null;
  globalProcesses: GlobalProcessRowVm[];
  timings: DispatchTimingRecord[];
};

type StatsPanelViewProps = {
  input: StatsPanelInput;
  onEvent?: (event: GlobalProcessEvent) => void;
};

function processSummary(process: ProcessDetailVm) {
  return `${String(process.state)} · CPU ${Math.round(process.cpuPercent)}% · ${process.processCount} proc`;
}

function activeProcessesSummary(rows: GlobalProcessRowVm[]) {
  const totalCpu = rows.reduce((sum, row) => sum + row.cpuPercent, 0);
  return `${rows.length} agent${rows.length === 1 ? "" : "s"} · CPU ${Math.round(totalCpu)}%`;
}

function dispatchTimingSummary(timings: DispatchTimingRecord[]) {
  const latest = timings[0];
  if (!latest) return undefined;
  return `${latest.event} · ${formatMs(latest.serverTotalMs)} server`;
}

export function StatsPanelView({ input, onEvent }: StatsPanelViewProps) {
  const { process, globalProcesses, timings } = input;
  if (!process && globalProcesses.length === 0 && timings.length === 0) return null;

  return (
    <div class="stats-panel">
      {process && (
        <CollapsiblePanel
          id="process-detail"
          title="Process activity"
          summary={processSummary(process)}
          defaultOpen
        >
          <ProcessView input={process} embedded />
        </CollapsiblePanel>
      )}
      {!process && globalProcesses.length > 0 && onEvent && (
        <CollapsiblePanel
          id="active-processes"
          title="Active processes"
          summary={activeProcessesSummary(globalProcesses)}
        >
          <GlobalProcessView rows={globalProcesses} onEvent={onEvent} embedded />
        </CollapsiblePanel>
      )}
      {timings.length > 0 && (
        <CollapsiblePanel
          id="dispatch-timing"
          title="Dispatch timing"
          summary={dispatchTimingSummary(timings)}
          bodyClass="timing-history-body"
        >
          <div class="timing-history-list mac-scrollbar">
            {timings.map((record, index) => (
              <DispatchTimingView
                key={`${record.event}-${record.serverTotalMs}-${index}`}
                record={record}
                isLatest={index === 0}
              />
            ))}
          </div>
        </CollapsiblePanel>
      )}
    </div>
  );
}
