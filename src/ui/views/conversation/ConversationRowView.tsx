import type { AppEvent, ConversationRowVm } from "../../../generated/engine-types";
import { ArchiveIcon } from "../../components/icons";
import { formatRelativeTime } from "../../utils/format";

export type ConversationRowInput = {
  row: ConversationRowVm;
};

export type ConversationRowEvent = Extract<
  AppEvent,
  { type: "conversationSelected" } | { type: "conversationArchived" }
>;

type ConversationRowViewProps = {
  input: ConversationRowInput;
  onEvent: (event: ConversationRowEvent) => void;
};

function fileLabel(path: string) {
  return path.split("/").pop() ?? path;
}

export function ConversationRowView({ input, onEvent }: ConversationRowViewProps) {
  const { row } = input;
  const active = row.processState != null;
  const editedSummary =
    row.editedFilePaths.length > 0
      ? row.editedFilePaths.map(fileLabel).join(", ")
      : row.editedFileCount > 0
        ? `${row.editedFileCount} files edited`
        : null;

  return (
    <div class={row.selected ? "conv-row selected" : "conv-row"}>
      <button
        type="button"
        class="conv-row-main"
        onClick={() => onEvent({ type: "conversationSelected", conversationId: row.id })}
      >
        {active ? (
          <span class="conv-acp-dot connected" data-state={row.processState} aria-label="Agent active" />
        ) : row.acpConnected ? (
          <span class="conv-acp-dot connected" aria-label="ACP connected" />
        ) : row.acpConnecting ? (
          <span class="conv-acp-dot connecting" aria-label="Connecting to ACP" />
        ) : (
          <span class="conv-acp-dot idle" aria-hidden="true" />
        )}
        <span class="conv-row-body">
          <span class="conv-row-topline">
            <span class="conv-row-title">{row.title}</span>
            {row.updatedAt > 0 && (
              <span class="conv-row-time">{formatRelativeTime(row.updatedAt)}</span>
            )}
          </span>
          {(row.lastMessagePreview || editedSummary) && (
            <span class="conv-row-meta">
              {row.lastMessagePreview && <span class="conv-row-preview">{row.lastMessagePreview}</span>}
              {row.lastMessagePreview && editedSummary && (
                <span class="conv-row-meta-sep" aria-hidden="true">
                  ·
                </span>
              )}
              {editedSummary && <span class="conv-row-edited">{editedSummary}</span>}
            </span>
          )}
        </span>
        {active && row.cpuPercent > 0 && (
          <span class="conv-row-cpu">{Math.round(row.cpuPercent)}%</span>
        )}
      </button>
      <button
        type="button"
        class="conv-archive-btn"
        aria-label={`Archive ${row.title}`}
        onClick={(e) => {
          e.stopPropagation();
          onEvent({ type: "conversationArchived", conversationId: row.id });
        }}
      >
        <ArchiveIcon size={13} />
      </button>
    </div>
  );
}
