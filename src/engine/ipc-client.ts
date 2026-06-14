import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppEvent, DispatchTimingVm, EngineResponse, ViewModelPatch } from "../generated/engine-types";
import { pushDispatchTiming } from "../state/dispatch-timing-overlay";
import { applyPatches } from "../state/view-model-store";

export async function bootEngine(): Promise<void> {
  const output = await invoke<EngineResponse>("initial_view_model");
  applyPatches(output.patches);
  await listen<EngineResponse>("engine://patches", (event) => applyPatches(event.payload.patches));
}

function latestDispatchTiming(patches: ViewModelPatch[]): DispatchTimingVm | null {
  const patch = patches.find((entry) => entry.op === "replace" && entry.path === "rightPane.dispatchTimings");
  if (!patch || patch.op !== "replace") return null;
  const timings = patch.value as DispatchTimingVm[];
  return timings[0] ?? null;
}

export async function dispatchAppEvent(event: AppEvent): Promise<void> {
  const invokeStart = performance.now();
  const output = await invoke<EngineResponse>("dispatch_app_event", { event });
  const invokeMs = performance.now() - invokeStart;
  const patchStart = performance.now();
  applyPatches(output.patches);
  const applyPatchesMs = performance.now() - patchStart;
  const serverTiming = latestDispatchTiming(output.patches);
  if (serverTiming) {
    pushDispatchTiming(serverTiming, invokeMs, applyPatchesMs);
  }
}
