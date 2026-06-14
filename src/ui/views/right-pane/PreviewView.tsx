import type { AppEvent, PreviewStatus } from "../../../generated/engine-types";

export type PreviewInput = {
  preview: PreviewStatus | null;
};

export type PreviewEvent = Extract<AppEvent, { type: "previewSuspended" } | { type: "previewClosed" }>;

type PreviewViewProps = {
  input: PreviewInput;
  onEvent: (event: PreviewEvent) => void;
};

export function PreviewView({ input, onEvent }: PreviewViewProps) {
  const { preview } = input;
  if (!preview) return <p class="empty">No preview open.</p>;
  return (
    <article class="preview-card">
      <b>{preview.url}</b>
      <small>
        {preview.state}
        {preview.lastDetectedPort ? ` · :${preview.lastDetectedPort}` : ""}
      </small>
      <div class="button-row">
        <button type="button" onClick={() => onEvent({ type: "previewSuspended", previewId: preview.previewId })}>
          Suspend
        </button>
        <button type="button" onClick={() => onEvent({ type: "previewClosed", previewId: preview.previewId })}>
          Destroy
        </button>
      </div>
    </article>
  );
}
