import { useCallback, useState } from "preact/hooks";
import type { AppEvent, CenterPaneVm } from "../../../generated/engine-types";
import { DotsIcon, FolderIcon } from "../../components/icons";
import { ToolStatusBar } from "../../components/ToolStatusBar";
import { ComposerView } from "../composer/ComposerView";
import { ApprovalView } from "./ApprovalView";
import { MessageView } from "./MessageView";
import { PlanPaneView } from "./PlanPaneView";

export type CenterPaneInput = CenterPaneVm;

export type CenterPaneEvent = Extract<
  AppEvent,
  | { type: "userPromptSubmitted" }
  | { type: "composerModeSelected" }
  | { type: "composerModelSelected" }
  | { type: "agentPermissionSelected" }
  | { type: "agentCancelled" }
  | { type: "agentKilled" }
  | { type: "agentPaused" }
  | { type: "agentResumed" }
  | { type: "queuedPromptRemoved" }
  | { type: "queuedPromptEdited" }
>;

type CenterPaneViewProps = {
  input: CenterPaneInput;
  onEvent: (event: CenterPaneEvent) => void;
  onOpenProjectFolder: () => void;
  canOpenProjectFolder: boolean;
};

function acpStatusClass(vm: CenterPaneInput): string {
  if (vm.acpConnected) return "acp-status connected";
  if (vm.status === "starting" || vm.status === "running") return "acp-status connecting";
  return "acp-status disconnected";
}

export function CenterPaneView({
  input: vm,
  onEvent,
  onOpenProjectFolder,
  canOpenProjectFolder,
}: CenterPaneViewProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const conversationId = vm.selectedConversationId;

  const agentAction = useCallback(
    (type: "agentCancelled" | "agentKilled" | "agentPaused" | "agentResumed") => {
      if (!conversationId) return;
      onEvent({ type, conversationId } as CenterPaneEvent);
      setMenuOpen(false);
    },
    [conversationId, onEvent],
  );

  return (
    <section class="pane center-pane">
      <header class="center-toolbar">
        <div class="center-toolbar-left">
          <span class="workspace-name">{vm.projectName}</span>
          <button
            type="button"
            class="icon-btn ghost"
            aria-label="Open folder"
            disabled={!canOpenProjectFolder}
            onClick={onOpenProjectFolder}
          >
            <FolderIcon size={14} />
          </button>
          {conversationId && (
            <div class={acpStatusClass(vm)} aria-label={vm.acpStatusLabel} title={vm.acpStatusLabel}>
              <span class="acp-status-dot" aria-hidden="true" />
              <span class="acp-status-label">{vm.acpStatusLabel}</span>
            </div>
          )}
        </div>
        <div class="center-toolbar-right">
          {conversationId && vm.agentRunning && (
            <div
              class="center-cpu-meter compact"
              title={`CPU ${Math.round(vm.cpuPercent)}% of ${Math.round(vm.cpuBudgetPercent)}% budget`}
            >
              <span class="center-cpu-value">{Math.round(vm.cpuPercent)}%</span>
            </div>
          )}
          <div class="center-menu-wrap">
            <button
              type="button"
              class="icon-btn ghost"
              aria-label="More options"
              aria-expanded={menuOpen}
              onClick={() => setMenuOpen((v) => !v)}
            >
              <DotsIcon size={14} />
            </button>
            {menuOpen && conversationId && (
              <div class="center-menu" role="menu">
                {vm.agentRunning ? (
                  <>
                    <button type="button" role="menuitem" onClick={() => agentAction("agentCancelled")}>
                      Stop agent
                    </button>
                    <button type="button" role="menuitem" onClick={() => agentAction("agentPaused")}>
                      Pause
                    </button>
                  </>
                ) : (
                  <button type="button" role="menuitem" onClick={() => agentAction("agentResumed")}>
                    Resume
                  </button>
                )}
                <button type="button" role="menuitem" class="destructive" onClick={() => agentAction("agentKilled")}>
                  Force kill
                </button>
              </div>
            )}
          </div>
        </div>
      </header>

      <div class="center-body">
        {vm.planVisible && vm.planText && <PlanPaneView text={vm.planText} />}
        <div class="center-workspace mac-scrollbar">
          {vm.approvals.map((approval) => (
            <ApprovalView key={approval.requestId} input={{ approval }} onEvent={onEvent} />
          ))}
          <ToolStatusBar status={vm.toolStatus} />
          {vm.messages.length === 0 && !vm.approvals.length && vm.toolStatus.running === 0 && vm.toolStatus.completed === 0 ? (
            <div class="empty-workspace" />
          ) : (
            vm.messages
              .filter((m) => !(vm.planVisible && m.kind === "plan"))
              .map((message) => <MessageView key={message.id} input={message} />)
          )}
        </div>

        <ComposerView
          key={vm.selectedConversationId ?? "none"}
          input={vm}
          onEvent={onEvent}
        />
      </div>
    </section>
  );
}
