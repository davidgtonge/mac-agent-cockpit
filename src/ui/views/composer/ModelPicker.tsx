import { useCallback, useMemo, useRef, useState } from "preact/hooks";
import { CheckIcon, ChevronDownIcon } from "../../components/icons";
import { filterOptions, type PickerOption } from "./composer-utils";
import { useOutsideClose } from "./use-outside-close";

type ModelPickerProps = {
  label: string;
  value: string;
  options: PickerOption[];
  disabled: boolean;
  onSelect: (id: string) => void;
};

export function ModelPicker({ label, value, options, disabled, onSelect }: ModelPickerProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const wrapRef = useRef<HTMLDivElement>(null);
  const filtered = useMemo(() => filterOptions(options, query), [options, query]);
  const close = useCallback(() => {
    setOpen(false);
    setQuery("");
  }, []);
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
        <span class="picker-pill-label">{label}</span>
        <ChevronDownIcon size={11} />
      </button>
      {open && (
        <div class="picker-menu picker-menu-model" role="listbox">
          <input
            autoFocus
            placeholder="Search models"
            class="picker-search"
            value={query}
            onInput={(event) => setQuery((event.currentTarget as HTMLInputElement).value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.stopPropagation();
                close();
              }
            }}
          />
          <div class="picker-list mac-scrollbar">
            {filtered.length === 0 ? (
              <div class="picker-empty">No models match.</div>
            ) : (
              filtered.map((option) => (
                <button
                  type="button"
                  key={option.id}
                  role="option"
                  aria-selected={value === option.id}
                  class={value === option.id ? "picker-row picker-row-model selected" : "picker-row picker-row-model"}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => {
                    onSelect(option.id);
                    close();
                  }}
                >
                  <span class="picker-row-label">{option.label}</span>
                  {option.description && <span class="picker-row-badge">{option.description}</span>}
                  {value === option.id && <CheckIcon class="picker-row-check" size={14} />}
                </button>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}
