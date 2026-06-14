import type { DispatchTimingRecord } from "../../../state/dispatch-timing-overlay";
import { formatMs } from "../../utils/format";

type DispatchTimingViewProps = {
  record: DispatchTimingRecord;
  isLatest?: boolean;
};

export function DispatchTimingView({ record, isLatest = false }: DispatchTimingViewProps) {
  return (
    <article class={`timing-card${isLatest ? " timing-card-latest" : ""}`}>
      <b>{isLatest ? "Latest dispatch" : "Dispatch"}</b>
      <span class="timing-event">{record.event}</span>
      <span>reduce {formatMs(record.reduceMs)}</span>
      <span>initial patch {formatMs(record.initialPatchMs)}</span>
      {record.effects.map((effect) => (
        <span key={effect.name}>
          effect {effect.name} {formatMs(effect.durationMs)}
        </span>
      ))}
      {record.drainIoMs > 0 && <span>drain io {formatMs(record.drainIoMs)}</span>}
      {record.finalizePatchMs > 0 && <span>finalize patch {formatMs(record.finalizePatchMs)}</span>}
      {record.responsePrepMs > 0 && <span>response prep {formatMs(record.responsePrepMs)}</span>}
      <span>server total {formatMs(record.serverTotalMs)}</span>
      <span>ipc round-trip {formatMs(record.clientInvokeMs)}</span>
      {record.clientInvokeMs > record.serverTotalMs && (
        <span>ipc overhead {formatMs(record.clientInvokeMs - record.serverTotalMs)}</span>
      )}
      <span>client apply patches {formatMs(record.clientApplyPatchesMs)}</span>
      {record.patchPaths.length > 0 && (
        <details class="timing-patches">
          <summary>
            {record.patchCount} patch{record.patchCount === 1 ? "" : "es"}
          </summary>
          <ul>
            {record.patchPaths.map((path) => (
              <li key={path}>{path || "(root)"}</li>
            ))}
          </ul>
        </details>
      )}
    </article>
  );
}
