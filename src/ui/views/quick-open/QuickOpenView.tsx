import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import type { AppEvent, LeftPaneVm, ProjectId } from "../../../generated/engine-types";

export type QuickOpenInput = Pick<
  LeftPaneVm,
  "quickOpenOpen" | "workspaceSearchHits" | "workspaceSearchDone" | "selectedProjectId" | "projects"
>;

export type QuickOpenEvent = Extract<
  AppEvent,
  | { type: "quickOpenToggled" }
  | { type: "workspaceSearchSubmitted" }
  | { type: "workspaceSearchCancelled" }
  | { type: "workspaceSearchResultSelected" }
>;

type QuickOpenViewProps = {
  input: QuickOpenInput;
  onEvent: (event: QuickOpenEvent) => void;
};

export function QuickOpenView({ input, onEvent }: QuickOpenViewProps) {
  const [query, setQuery] = useState("");
  const [index, setIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const projectId = input.selectedProjectId ?? input.projects[0]?.id;

  useEffect(() => {
    if (input.quickOpenOpen) {
      setQuery("");
      setIndex(0);
      inputRef.current?.focus();
    }
  }, [input.quickOpenOpen]);

  useEffect(() => {
    if (!input.quickOpenOpen || !projectId) return;
    const handle = window.setTimeout(() => {
      if (!query.trim()) return;
      onEvent({
        type: "workspaceSearchSubmitted",
        projectId,
        query: query.trim(),
        mode: "both",
      });
    }, 150);
    return () => window.clearTimeout(handle);
  }, [query, input.quickOpenOpen, onEvent, projectId]);

  const close = useCallback(() => {
    onEvent({ type: "quickOpenToggled", open: false });
    onEvent({ type: "workspaceSearchCancelled" });
  }, [onEvent]);

  const selectHit = useCallback(
    (path: string) => {
      if (!projectId) return;
      onEvent({ type: "workspaceSearchResultSelected", projectId, path });
    },
    [onEvent, projectId],
  );

  if (!input.quickOpenOpen) return null;

  const filenameHits = input.workspaceSearchHits.filter((h) => h.kind === "filename");
  const contentHits = input.workspaceSearchHits.filter((h) => h.kind === "content");
  const flatHits = [...filenameHits, ...contentHits];

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setIndex((i) => Math.min(i + 1, Math.max(0, flatHits.length - 1)));
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setIndex((i) => Math.max(i - 1, 0));
      return;
    }
    if (e.key === "Enter" && flatHits[index]) {
      e.preventDefault();
      selectHit(flatHits[index].path);
    }
  };

  return (
    <div class="quick-open-overlay" role="dialog" aria-label="Quick open" onClick={close}>
      <div class="quick-open-modal" onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          class="quick-open-input"
          type="search"
          placeholder="Search files and content…"
          value={query}
          onInput={(e) => {
            setQuery((e.target as HTMLInputElement).value);
            setIndex(0);
          }}
          onKeyDown={onKeyDown}
        />
        <div class="quick-open-results mac-scrollbar">
          {filenameHits.length > 0 && (
            <section>
              <div class="quick-open-section-label">Files</div>
              {filenameHits.map((hit, i) => (
                <button
                  key={`f-${hit.path}`}
                  type="button"
                  class={i === index ? "quick-open-row selected" : "quick-open-row"}
                  onClick={() => selectHit(hit.path)}
                >
                  <span class="quick-open-path">{hit.path}</span>
                </button>
              ))}
            </section>
          )}
          {contentHits.length > 0 && (
            <section>
              <div class="quick-open-section-label">Content</div>
              {contentHits.map((hit, i) => {
                const rowIndex = filenameHits.length + i;
                return (
                  <button
                    key={`c-${hit.path}-${hit.line}`}
                    type="button"
                    class={rowIndex === index ? "quick-open-row selected" : "quick-open-row"}
                    onClick={() => selectHit(hit.path)}
                  >
                    <span class="quick-open-path">
                      {hit.path}
                      {hit.line ? `:${hit.line}` : ""}
                    </span>
                    <span class="quick-open-snippet">{hit.snippet}</span>
                  </button>
                );
              })}
            </section>
          )}
          {query.trim() && input.workspaceSearchDone && flatHits.length === 0 && (
            <div class="quick-open-empty">No results</div>
          )}
        </div>
      </div>
    </div>
  );
}
