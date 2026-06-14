import { useEffect, useMemo, useState } from "preact/hooks";
import { getViewModel, subscribe } from "./view-model-store";
import type { ViewModel } from "../generated/engine-types";

export function useViewModel<T>(selector: (vm: ViewModel) => T): T | null {
  const [version, setVersion] = useState(0);
  useEffect(() => subscribe(() => setVersion((v) => v + 1)), []);
  return useMemo(() => {
    const vm = getViewModel();
    return vm ? selector(vm) : null;
  }, [selector, version]);
}
