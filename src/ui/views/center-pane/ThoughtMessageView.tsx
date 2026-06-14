import { useEffect, useRef, useState } from "preact/hooks";
import type { MessageVm } from "../../../generated/engine-types";
import { ChevronDownIcon } from "../../components/icons";

type ThoughtMessageViewProps = {
  input: MessageVm;
};

export function ThoughtMessageView({ input: message }: ThoughtMessageViewProps) {
  const streaming = message.streaming;
  const [expanded, setExpanded] = useState(false);
  const bodyRef = useRef<HTMLDivElement>(null);
  const showBody = streaming || expanded;

  useEffect(() => {
    if (!streaming) setExpanded(false);
  }, [streaming]);

  useEffect(() => {
    if (streaming && bodyRef.current) {
      bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
    }
  }, [message.text, streaming]);

  return (
    <article class={`message thought${streaming ? " is-streaming" : ""}${showBody ? "" : " is-collapsed"}`}>
      <button
        type="button"
        class="thought-toggle"
        aria-expanded={showBody}
        disabled={streaming}
        onClick={() => {
          if (!streaming) setExpanded((open) => !open);
        }}
      >
        <span class="thought-toggle-label">Thinking</span>
        {streaming && <span class="thought-live-dot" aria-hidden="true" />}
        <ChevronDownIcon class={showBody ? "thought-chevron open" : "thought-chevron"} size={12} />
      </button>
      {showBody && (
        <div ref={bodyRef} class={streaming ? "thought-body streaming" : "thought-body"}>
          <p>{message.text}</p>
        </div>
      )}
    </article>
  );
}
