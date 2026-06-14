import { useCallback, useRef, useState } from "preact/hooks";

export type PaneLayout = {
  left: number;
  center: number;
  fileTree: number;
};

export type PaneKey = keyof PaneLayout;

const STORAGE_KEY = "cockpit.layout.v1";

export const PANE_LAYOUT_DEFAULTS: PaneLayout = {
  left: 228,
  center: 420,
  fileTree: 220,
};

export const PANE_LAYOUT_LIMITS: Record<PaneKey, { min: number; max: number }> = {
  left: { min: 180, max: 360 },
  center: { min: 320, max: 640 },
  fileTree: { min: 160, max: 400 },
};

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function loadLayout(): PaneLayout {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...PANE_LAYOUT_DEFAULTS };
    const parsed = JSON.parse(raw) as Partial<PaneLayout>;
    return {
      left: clamp(Number(parsed.left) || PANE_LAYOUT_DEFAULTS.left, PANE_LAYOUT_LIMITS.left.min, PANE_LAYOUT_LIMITS.left.max),
      center: clamp(
        Number(parsed.center) || PANE_LAYOUT_DEFAULTS.center,
        PANE_LAYOUT_LIMITS.center.min,
        PANE_LAYOUT_LIMITS.center.max,
      ),
      fileTree: clamp(
        Number(parsed.fileTree) || PANE_LAYOUT_DEFAULTS.fileTree,
        PANE_LAYOUT_LIMITS.fileTree.min,
        PANE_LAYOUT_LIMITS.fileTree.max,
      ),
    };
  } catch {
    return { ...PANE_LAYOUT_DEFAULTS };
  }
}

function saveLayout(layout: PaneLayout) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
  } catch {
    // Ignore quota / private-mode failures.
  }
}

export function usePaneLayout() {
  const [layout, setLayout] = useState<PaneLayout>(loadLayout);
  const layoutRef = useRef(layout);
  layoutRef.current = layout;

  const resetPane = useCallback((pane: PaneKey) => {
    setLayout((prev) => {
      const next = { ...prev, [pane]: PANE_LAYOUT_DEFAULTS[pane] };
      saveLayout(next);
      return next;
    });
  }, []);

  const startResize = useCallback((pane: PaneKey, event: PointerEvent) => {
    event.preventDefault();
    const handle = event.currentTarget as HTMLElement;
    handle.setPointerCapture(event.pointerId);

    const startX = event.clientX;
    const startValue = layoutRef.current[pane];
    const { min, max } = PANE_LAYOUT_LIMITS[pane];

    const onPointerMove = (moveEvent: PointerEvent) => {
      const delta = moveEvent.clientX - startX;
      const next = clamp(startValue + delta, min, max);
      setLayout((prev) => ({ ...prev, [pane]: next }));
    };

    const onPointerUp = () => {
      handle.releasePointerCapture(event.pointerId);
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
      saveLayout(layoutRef.current);
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
  }, []);

  const shellStyle = {
    "--left-w": `${layout.left}px`,
    "--center-w": `${layout.center}px`,
  } as Record<string, string>;

  const editorWorkspaceStyle = {
    "--file-tree-w": `${layout.fileTree}px`,
  } as Record<string, string>;

  return {
    layout,
    shellStyle,
    editorWorkspaceStyle,
    startResize,
    resetPane,
  };
}
