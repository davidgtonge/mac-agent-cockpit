import type { AppEvent, FileReviewViewModel, FileReviewView, ProjectId } from "../../../generated/engine-types";
import { CodeEditor } from "../../components/CodeEditor";
import { isMarkdownPreview, MarkdownFilePreview } from "../../components/MarkdownFilePreview";
import { StructuredDiffView } from "./StructuredDiffView";

export type FileReviewEvent = Extract<
  AppEvent,
  { type: "reviewViewSelected" } | { type: "fileReviewClosed" }
>;

type FileReviewPanelProps = {
  projectId: ProjectId | null;
  review: FileReviewViewModel;
  onEvent: (event: FileReviewEvent) => void;
};

const VIEW_LABELS: Record<FileReviewView, string> = {
  inlineChanges: "Inline Changes",
  current: "Current",
  before: "Before",
};

export function FileReviewPanel({ projectId, review, onEvent }: FileReviewPanelProps) {
  const secondaryViews = review.availableViews.filter((view) => view !== review.selectedView);

  return (
    <section class="file-review-panel">
      <header class="file-review-header">
        <div class="file-review-heading">
          <h2 class="file-review-title">{review.fileName}</h2>
          <p class="file-review-meta">
            <span>{review.statusLabel}</span>
            {review.changeSummary ? <span> · {review.changeSummary}</span> : null}
          </p>
          {review.comparisonLabel ? (
            <p class="file-review-comparison">{review.comparisonLabel}</p>
          ) : null}
          {review.contextNotice ? (
            <p class="file-review-context">{review.contextNotice}</p>
          ) : null}
        </div>
        {secondaryViews.length > 0 ? (
          <div class="file-review-secondary" aria-label="Other review views">
            {secondaryViews.map((view) => (
              <button
                key={view}
                type="button"
                class="file-review-secondary-btn"
                disabled={!projectId}
                onClick={() =>
                  projectId &&
                  onEvent({
                    type: "reviewViewSelected",
                    projectId,
                    path: review.path,
                    view,
                  })
                }
              >
                {VIEW_LABELS[view]}
              </button>
            ))}
          </div>
        ) : null}
      </header>

      {review.loading ? <div class="file-review-state">Loading…</div> : null}
      {review.error ? <div class="file-review-state file-review-error">{review.error}</div> : null}
      {review.notice ? <div class="file-review-state file-review-notice">{review.notice}</div> : null}

      {!review.loading && review.selectedView === "inlineChanges" && review.inlineChanges ? (
        <StructuredDiffView diff={review.inlineChanges} />
      ) : null}

      {!review.loading && review.selectedView !== "inlineChanges" && review.preview ? (
        isMarkdownPreview(review.preview.path, review.preview.languageHint) ? (
          <MarkdownFilePreview
            text={review.preview.text ?? ""}
            truncated={review.preview.truncated}
          />
        ) : (
          <CodeEditor
            text={review.preview.text ?? ""}
            highlightedLines={review.preview.highlightedLines}
            path={review.preview.path}
            languageHint={review.preview.languageHint}
            truncated={review.preview.truncated}
          />
        )
      ) : null}
    </section>
  );
}
