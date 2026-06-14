import type { StructuredDiffVm } from "../../../generated/engine-types";

type StructuredDiffViewProps = {
  diff: StructuredDiffVm;
};

export function StructuredDiffView({ diff }: StructuredDiffViewProps) {
  return (
    <section class="structured-diff mac-scrollbar">
      <header class="structured-diff-header">
        <span class="structured-diff-stat">{diff.stat}</span>
        {diff.oldPath && diff.newPath && diff.oldPath !== diff.newPath ? (
          <span class="structured-diff-rename">
            {diff.oldPath} → {diff.newPath}
          </span>
        ) : null}
      </header>
      {diff.hunks.map((hunk, hunkIndex) => (
        <div class="structured-diff-hunk" key={`hunk-${hunkIndex}`}>
          {hunk.rows.map((row, rowIndex) => (
            <div class={`structured-diff-row structured-diff-row-${row.kind}`} key={`row-${hunkIndex}-${rowIndex}`}>
              <span class="structured-diff-gutter structured-diff-gutter-old">{row.oldLine ?? ""}</span>
              <span class="structured-diff-gutter structured-diff-gutter-new">{row.newLine ?? ""}</span>
              <span
                class="structured-diff-code"
                dangerouslySetInnerHTML={{ __html: row.highlightedHtml || "&nbsp;" }}
              />
            </div>
          ))}
        </div>
      ))}
    </section>
  );
}
