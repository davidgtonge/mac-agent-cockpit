import { useViewModel } from "../../state/use-view-model";
import { ToastsView } from "../views/ToastsView";

export function ToastsHost() {
  const toasts = useViewModel((vm) => vm.toasts);
  if (!toasts) return null;
  return <ToastsView input={toasts} />;
}
