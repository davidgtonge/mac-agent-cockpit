import type { ComponentChildren } from "preact";
import { useState } from "preact/hooks";
import { ChevronDownIcon, ChevronRightIcon } from "./icons";

type CollapsiblePanelProps = {
  id: string;
  title: string;
  summary?: string;
  defaultOpen?: boolean;
  class?: string;
  bodyClass?: string;
  children: ComponentChildren;
};

const STORAGE_PREFIX = "cockpit.panel.";

function loadPanelOpen(id: string, defaultOpen: boolean) {
  try {
    const stored = localStorage.getItem(`${STORAGE_PREFIX}${id}`);
    if (stored === null) return defaultOpen;
    return stored === "true";
  } catch {
    return defaultOpen;
  }
}

function savePanelOpen(id: string, open: boolean) {
  try {
    localStorage.setItem(`${STORAGE_PREFIX}${id}`, String(open));
  } catch {
    // Ignore quota / private-mode failures.
  }
}

export function CollapsiblePanel({
  id,
  title,
  summary,
  defaultOpen = false,
  class: className,
  bodyClass,
  children,
}: CollapsiblePanelProps) {
  const [open, setOpen] = useState(() => loadPanelOpen(id, defaultOpen));

  const toggle = () => {
    setOpen((prev) => {
      const next = !prev;
      savePanelOpen(id, next);
      return next;
    });
  };

  return (
    <section class={className ? `collapsible-panel ${className}` : "collapsible-panel"} data-open={open}>
      <button type="button" class="collapsible-panel-header" onClick={toggle} aria-expanded={open}>
        <span class="collapsible-panel-chevron" aria-hidden="true">
          {open ? <ChevronDownIcon size={12} /> : <ChevronRightIcon size={12} />}
        </span>
        <span class="collapsible-panel-title">{title}</span>
        {summary && <span class="collapsible-panel-summary">{summary}</span>}
      </button>
      {open && <div class={bodyClass ? `collapsible-panel-body ${bodyClass}` : "collapsible-panel-body"}>{children}</div>}
    </section>
  );
}
