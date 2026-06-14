import type { AppEvent, PermissionOption, PermissionRequestVm } from "../../../generated/engine-types";

export type ApprovalInput = {
  approval: PermissionRequestVm;
};

export type ApprovalEvent = Extract<AppEvent, { type: "agentPermissionSelected" }>;

type ApprovalViewProps = {
  input: ApprovalInput;
  onEvent: (event: ApprovalEvent) => void;
};

function optionButtonClass(option: PermissionOption): string {
  const kind = option.kind ?? option.optionId;
  if (kind.includes("reject")) {
    return "danger";
  }
  if (kind.includes("always")) {
    return "secondary";
  }
  return "primary";
}

function formatToolKind(kind?: string | null): string {
  if (!kind) {
    return "Permission";
  }
  return kind.replace(/_/g, " ");
}

export function ApprovalView({ input, onEvent }: ApprovalViewProps) {
  const { approval } = input;

  return (
    <article class="approval">
      <header class="approval-header">
        <span class="approval-badge">{formatToolKind(approval.toolKind)}</span>
        <h3>{approval.title}</h3>
      </header>

      {approval.toolCallTitle && approval.toolCallTitle !== approval.title && (
        <p class="approval-command">{approval.toolCallTitle}</p>
      )}

      <p class="approval-summary">{approval.summary}</p>

      <div class="approval-actions">
        {approval.options.map((option) => (
          <button
            key={option.optionId}
            type="button"
            class={optionButtonClass(option)}
            title={option.description ?? undefined}
            onClick={() =>
              onEvent({
                type: "agentPermissionSelected",
                requestId: approval.requestId,
                optionId: option.optionId,
              })
            }
          >
            {option.label}
          </button>
        ))}
      </div>
    </article>
  );
}
