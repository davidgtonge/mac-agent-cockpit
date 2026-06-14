import type { DiffResult } from "../../../generated/engine-types";

type DiffViewProps = {
  input: DiffResult | null;
};

export function DiffView({ input: diff }: DiffViewProps) {
  if (!diff) return <p class="empty">Select a changed file.</p>;
  return <pre class="diff-view mac-scrollbar">{diff.text || "No unstaged diff for this file."}</pre>;
}
