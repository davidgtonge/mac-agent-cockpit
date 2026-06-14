import { useCallback, useMemo, useRef, useState } from "preact/hooks";
import type { AppEvent, CenterPaneVm, SlashCommandVm } from "../../../generated/engine-types";
import { ArrowUpIcon, MicIcon, PlusIcon } from "../../components/icons";
import { buildSuggestions, modeIdForSlashCommand, parseSlashContext, type SlashSuggestion } from "./composer-utils";
import { ModePicker } from "./ModePicker";
import { ModelPicker } from "./ModelPicker";

export type ComposerInput = CenterPaneVm;

export type ComposerEvent = Extract<
  AppEvent,
  | { type: "userPromptSubmitted" }
  | { type: "composerModeSelected" }
  | { type: "composerModelSelected" }
  | { type: "queuedPromptRemoved" }
>;

type ComposerViewProps = {
  input: CenterPaneVm;
  onEvent: (event: ComposerEvent) => void;
};

export function ComposerView({ input: vm, onEvent }: ComposerViewProps) {
  const [text, setText] = useState("");
  const [menuIndex, setMenuIndex] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const slashContext = parseSlashContext(text);
  const menuOpen = slashContext !== null && vm.composerEnabled;
  const suggestions = useMemo(
    () => (slashContext ? buildSuggestions(slashContext, vm) : []),
    [slashContext, vm],
  );
  const modeOptions = vm.modeOptions;
  const modelOptions = vm.modelOptions;
  const currentMode = vm.currentMode ?? modeOptions[0]?.id ?? "agent";
  const currentModelId = vm.currentModelId ?? modelOptions[0]?.id ?? "composer-2.5";
  const modeLabel = vm.currentModeLabel ?? modeOptions.find((m) => m.id === currentMode)?.label ?? "Agent";
  const modelLabel = vm.currentModelLabel ?? modelOptions.find((m) => m.id === currentModelId)?.label ?? "Composer 2.5";
  const picksDisabled = !vm.composerEnabled;
  const placeholder =
    vm.currentMode === "plan"
      ? "Describe what you want to plan…"
      : vm.agentRunning && !vm.steerSupported
        ? "Queue a follow-up message…"
        : vm.agentRunning
          ? "Steer the running agent…"
          : "Plan, search, build anything…";

  const applyCommandSuggestion = useCallback(
    (command: SlashCommandVm) => {
      const modeId = modeIdForSlashCommand(command.name);
      if (modeId && vm.selectedConversationId) {
        onEvent({ type: "composerModeSelected", conversationId: vm.selectedConversationId, modeId });
        setText("");
        setMenuIndex(0);
        textareaRef.current?.focus();
        return;
      }
      const next = text.replace(/(?:^|\s)\/[\w-]*$/, (match) => {
        const prefix = match.startsWith(" ") ? " " : "";
        return `${prefix}/${command.name} `;
      });
      setText(next);
      setMenuIndex(0);
      textareaRef.current?.focus();
    },
    [onEvent, text, vm.selectedConversationId],
  );

  const applySuggestion = useCallback(
    (suggestion: SlashSuggestion) => {
      applyCommandSuggestion(suggestion.command);
    },
    [applyCommandSuggestion],
  );

  const submit = useCallback(() => {
    if (!vm.selectedConversationId || !text.trim()) return;
    onEvent({
      type: "userPromptSubmitted",
      conversationId: vm.selectedConversationId,
      text: text.trim(),
    });
    setText("");
    setMenuIndex(0);
  }, [onEvent, text, vm.selectedConversationId]);

  const onModeSelect = useCallback(
    (modeId: string) => {
      if (!vm.selectedConversationId) return;
      onEvent({ type: "composerModeSelected", conversationId: vm.selectedConversationId, modeId });
    },
    [onEvent, vm.selectedConversationId],
  );

  const onModelSelect = useCallback(
    (modelId: string) => {
      if (!vm.selectedConversationId) return;
      onEvent({ type: "composerModelSelected", conversationId: vm.selectedConversationId, modelId });
    },
    [onEvent, vm.selectedConversationId],
  );

  const onKeyDown = (event: KeyboardEvent) => {
    if (!menuOpen || suggestions.length === 0) {
      if (event.key === "Enter" && !event.shiftKey) {
        event.preventDefault();
        submit();
      }
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setMenuIndex((index) => (index + 1) % suggestions.length);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setMenuIndex((index) => (index - 1 + suggestions.length) % suggestions.length);
      return;
    }
    if (event.key === "Tab" || (event.key === "Enter" && !event.shiftKey)) {
      event.preventDefault();
      applySuggestion(suggestions[menuIndex] ?? suggestions[0]);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      setMenuIndex(0);
    }
  };

  return (
    <div class="composer-dock">
      <form
        class="composer-box"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        {menuOpen && (
          <menu class="slash-menu">
            <div class="slash-menu-title">Commands</div>
            {suggestions.length === 0 ? (
              <div class="slash-empty">No matching commands.</div>
            ) : (
              suggestions.map((suggestion, index) => (
                <button
                  type="button"
                  class={index === menuIndex ? "slash-item selected" : "slash-item"}
                  key={`cmd-${suggestion.command.name}`}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    applyCommandSuggestion(suggestion.command);
                  }}
                >
                  <span class="slash-name">/{suggestion.command.name}</span>
                  {suggestion.command.description && (
                    <span class="slash-desc">{suggestion.command.description}</span>
                  )}
                </button>
              ))
            )}
          </menu>
        )}

        {vm.queuedPrompts.length > 0 && (
          <div class="queued-prompts" aria-label="Queued messages">
            {vm.queuedPrompts.map((prompt) => (
              <div class="queued-prompt-chip" key={prompt.id}>
                <span>{prompt.text}</span>
                {vm.selectedConversationId && (
                  <button
                    type="button"
                    class="queued-prompt-remove"
                    aria-label="Remove queued message"
                    onClick={() =>
                      onEvent({
                        type: "queuedPromptRemoved",
                        conversationId: vm.selectedConversationId!,
                        promptId: prompt.id,
                      })
                    }
                  >
                    ×
                  </button>
                )}
              </div>
            ))}
          </div>
        )}

        <textarea
          ref={textareaRef}
          class="composer-input"
          value={text}
          disabled={!vm.composerEnabled}
          placeholder={placeholder}
          rows={2}
          onInput={(e) => {
            setText((e.currentTarget as HTMLTextAreaElement).value);
            setMenuIndex(0);
          }}
          onKeyDown={onKeyDown}
        />

        <div class="composer-toolbar">
          <div class="composer-toolbar-left">
            <button type="button" class="composer-icon-btn" aria-label="Attach" disabled={!vm.composerEnabled}>
              <PlusIcon size={15} />
            </button>
            <ModePicker
              label={modeLabel}
              value={currentMode}
              options={modeOptions}
              disabled={picksDisabled}
              onSelect={onModeSelect}
            />
            <ModelPicker
              label={modelLabel}
              value={currentModelId}
              options={modelOptions}
              disabled={picksDisabled}
              onSelect={onModelSelect}
            />
          </div>
          <div class="composer-toolbar-right">
            <button type="button" class="composer-icon-btn" aria-label="Voice input" disabled={!vm.composerEnabled}>
              <MicIcon size={15} />
            </button>
            <button class="send-btn" disabled={!vm.composerEnabled || !text.trim()} type="submit" aria-label="Send">
              <ArrowUpIcon size={14} />
            </button>
          </div>
        </div>
      </form>
    </div>
  );
}
