import type { ViewModel, ViewModelPatch } from "../generated/engine-types";

type Listener = () => void;
let current: ViewModel | null = null;
const listeners = new Set<Listener>();

export function getViewModel(): ViewModel | null {
  return current;
}

export function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function applyPatches(patches: ViewModelPatch[]): void {
  for (const patch of patches) {
    if (patch.op === "replace") {
      if (patch.path === "") {
        current = patch.value as ViewModel;
      } else if (current) {
        replaceAtPath(current as Record<string, unknown>, patch.path, patch.value);
      }
    } else if (patch.op === "remove" && current) {
      removeAtPath(current as Record<string, unknown>, patch.path);
    }
  }
  for (const listener of listeners) listener();
}

function pathParts(path: string): string[] {
  return path.split(".").filter(Boolean);
}

function replaceAtPath(root: Record<string, unknown>, path: string, value: unknown): void {
  const parts = pathParts(path);
  let cursor: Record<string, unknown> = root;
  while (parts.length > 1) {
    const key = parts.shift()!;
    cursor = cursor[key] as Record<string, unknown>;
  }
  cursor[parts[0]] = value;
}

function removeAtPath(root: Record<string, unknown>, path: string): void {
  const parts = pathParts(path);
  let cursor: Record<string, unknown> = root;
  while (parts.length > 1) {
    const key = parts.shift()!;
    cursor = cursor[key] as Record<string, unknown>;
  }
  delete cursor[parts[0]];
}
