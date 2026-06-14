import type { ToolStatusVm } from "../../generated/engine-types";

type ToolStatusBarProps = {
  status: ToolStatusVm;
};

export function ToolStatusBar({ status }: ToolStatusBarProps) {
  const { running, completed } = status;
  if (running === 0 && completed === 0) return null;

  const parts: string[] = [];
  if (running > 0) {
    parts.push(`${running} running`);
  }
  if (completed > 0) {
    parts.push(`${completed} completed`);
  }

  return (
    <div class="tool-status" role="status" aria-live="polite">
      <span class="tool-status-dot" data-active={running > 0 ? "true" : "false"} aria-hidden="true" />
      <span class="tool-status-label">Tools</span>
      <span class="tool-status-count">{parts.join(" · ")}</span>
    </div>
  );
}
