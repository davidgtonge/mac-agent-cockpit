import type { ComponentType } from "preact";
import { useCallback, useRef, useState } from "preact/hooks";
import {
  AgentModeIcon,
  AskModeIcon,
  CheckIcon,
  ChevronDownIcon,
  DebugModeIcon,
  PlanModeIcon,
} from "../../components/icons";
import type { PickerOption } from "./composer-utils";
import { useOutsideClose } from "./use-outside-close";

type IconProps = { size?: number; class?: string };

const MODE_ICONS: Record<string, ComponentType<IconProps>> = {
  agent: AgentModeIcon,
  plan: PlanModeIcon,
  ask: AskModeIcon,
  debug: DebugModeIcon,
};

function ModeIcon({ id, size = 14 }: { id: string; size?: number }) {
  const Icon = MODE_ICONS[id] ?? AgentModeIcon;
  return <Icon size={size} />;
}

type ModePickerProps = {
  label: string;
  value: string;
  options: PickerOption[];
  disabled: boolean;
  onSelect: (id: string) => void;
};

export function ModePicker({ label, value, options, disabled, onSelect }: ModePickerProps) {
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);
  const close = useCallback(() => setOpen(false), []);
  useOutsideClose(open, wrapRef, close);

  return (
    <div class="picker-wrap" ref={wrapRef}>
      <button
        type="button"
        class="picker-pill"
        disabled={disabled}
        aria-expanded={open}
        aria-haspopup="listbox"
        onMouseDown={(event) => event.stopPropagation()}
        onClick={() => {
          if (!disabled) setOpen((v) => !v);
        }}
      >
        <ModeIcon id={value} size={13} />
        <span class="picker-pill-label">{label}</span>
        <ChevronDownIcon size={11} />
      </button>
      {open && (
        <div class="picker-menu picker-menu-mode" role="listbox">
          {options.map((option) => (
            <button
              type="button"
              key={option.id}
              role="option"
              aria-selected={value === option.id}
              class={value === option.id ? "picker-row selected" : "picker-row"}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => {
                onSelect(option.id);
                close();
              }}
            >
              <span class="picker-row-icon">
                <ModeIcon id={option.id} size={14} />
              </span>
              <span class="picker-row-label">{option.label}</span>
              {value === option.id && <CheckIcon class="picker-row-check" size={14} />}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
