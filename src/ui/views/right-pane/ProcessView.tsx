import type { ProcessDetailVm } from "../../../generated/engine-types";
import { formatBytes } from "../../utils/format";

type ProcessViewProps = {
  input: ProcessDetailVm;
  embedded?: boolean;
};

export function ProcessView({ input: process, embedded = false }: ProcessViewProps) {
  const content = (
    <>
      {!embedded && (
        <header class="process-card-header">
          <b>{process.conversationTitle}</b>
          <span data-state={process.state}>{String(process.state)}</span>
        </header>
      )}
      <div class="process-summary">
        <span>PID {process.rootPid ?? "-"}</span>
        <span>
          CPU {Math.round(process.cpuPercent)}% / {Math.round(process.cpuBudgetPercent)}%
        </span>
        <span>Memory {formatBytes(process.memoryBytes)}</span>
        <span>Processes {process.processCount}</span>
      </div>
      {process.nodes.length > 0 && (
        <table class="process-tree">
          <thead>
            <tr>
              <th>PID</th>
              <th>PPID</th>
              <th>CPU</th>
              <th>Memory</th>
            </tr>
          </thead>
          <tbody>
            {process.nodes.map((node) => (
              <tr key={node.pid} class={node.pid === process.rootPid ? "process-root" : undefined}>
                <td>{node.pid}</td>
                <td>{node.ppid}</td>
                <td>{Math.round(node.cpuPercent)}%</td>
                <td>{formatBytes(node.memoryBytes)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </>
  );

  if (embedded) {
    return <div class="process-card embedded">{content}</div>;
  }

  return <article class="process-card">{content}</article>;
}
