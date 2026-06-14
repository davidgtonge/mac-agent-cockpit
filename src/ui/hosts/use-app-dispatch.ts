import { useCallback } from "preact/hooks";
import type { AppEvent } from "../../generated/engine-types";
import { dispatchAppEvent } from "../../engine/ipc-client";

export function useAppDispatch() {
  return useCallback((event: AppEvent) => void dispatchAppEvent(event), []);
}
