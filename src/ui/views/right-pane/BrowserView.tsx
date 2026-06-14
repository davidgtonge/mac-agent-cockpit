import { useState } from "preact/hooks";
import type { AppEvent } from "../../../generated/engine-types";

export type BrowserViewEvent = Extract<AppEvent, { type: "browserUrlChanged" }>;

type BrowserViewProps = {
  url?: string | null;
  onEvent: (event: BrowserViewEvent) => void;
};

export function BrowserView({ url, onEvent }: BrowserViewProps) {
  const [draft, setDraft] = useState(url ?? "http://localhost:5173");

  const navigate = () => {
    const value = draft.trim();
    if (!value) return;
    onEvent({ type: "browserUrlChanged", url: value });
  };

  return (
    <div class="browser-pane">
      <form
        class="browser-toolbar"
        onSubmit={(e) => {
          e.preventDefault();
          navigate();
        }}
      >
        <input
          class="browser-url-input"
          type="url"
          value={draft}
          placeholder="https://…"
          onInput={(e) => setDraft((e.target as HTMLInputElement).value)}
        />
        <button type="submit" class="browser-go-btn">
          Go
        </button>
      </form>
      {url ? (
        <iframe class="browser-frame" src={url} title="Browser preview" sandbox="allow-scripts allow-same-origin allow-forms allow-popups" />
      ) : (
        <div class="browser-empty">Enter a URL to browse</div>
      )}
    </div>
  );
}
