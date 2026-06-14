import { createContext } from "preact";
import { useContext } from "preact/hooks";
import type { ComponentChildren } from "preact";
import { usePaneLayout } from "./use-pane-layout";

type PaneLayoutContextValue = ReturnType<typeof usePaneLayout>;

const PaneLayoutContext = createContext<PaneLayoutContextValue | null>(null);

export function PaneLayoutProvider({ children }: { children: ComponentChildren }) {
  const value = usePaneLayout();
  return <PaneLayoutContext.Provider value={value}>{children}</PaneLayoutContext.Provider>;
}

export function usePaneLayoutContext() {
  const context = useContext(PaneLayoutContext);
  if (!context) {
    throw new Error("usePaneLayoutContext must be used within PaneLayoutProvider");
  }
  return context;
}
