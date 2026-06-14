import type { DispatchTimingVm } from "../generated/engine-types";

export type DispatchTimingRecord = DispatchTimingVm & {
  clientInvokeMs: number;
  clientApplyPatchesMs: number;
};

const MAX_HISTORY = 10;

let history: DispatchTimingRecord[] = [];
const listeners = new Set<() => void>();

function notify(): void {
  for (const listener of listeners) listener();
}

export function pushDispatchTiming(
  server: DispatchTimingVm,
  clientInvokeMs: number,
  clientApplyPatchesMs: number,
): void {
  history = [
    { ...server, clientInvokeMs, clientApplyPatchesMs },
    ...history,
  ].slice(0, MAX_HISTORY);
  notify();
}

export function getDispatchTimingHistory(): DispatchTimingRecord[] {
  return history;
}

export function subscribeDispatchTimingHistory(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
