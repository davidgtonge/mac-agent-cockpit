import type { AgentRowVm, AppEvent } from "../../../generated/engine-types";

export type AgentChipInput = {
  row: AgentRowVm;
};

export type AgentChipEvent = Extract<AppEvent, { type: "agentPaused" }>;

type AgentChipViewProps = {
  input: AgentChipInput;
  onEvent: (event: AgentChipEvent) => void;
};

export function AgentChipView({ input, onEvent }: AgentChipViewProps) {
  const { row } = input;
  return (
    <article class="agent-chip">
      <span class="agent-dot" data-state={row.state} />
      <span class="agent-chip-title">{row.title}</span>
      <span class="agent-chip-meta">{row.cpuLabel}</span>
      <button
        type="button"
        class="icon-btn ghost tiny"
        onClick={() => onEvent({ type: "agentPaused", conversationId: row.id })}
      >
        ⏸
      </button>
    </article>
  );
}
