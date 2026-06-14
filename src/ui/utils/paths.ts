export function normalizeWorkspacePath(path: string): string {
  const cleaned = path.replace(/\\/g, "/").replace(/^\.\//, "").replace(/\/+$/g, "");
  return cleaned === "" ? "." : cleaned;
}

export function pathsMatch(a: string | null | undefined, b: string | null | undefined): boolean {
  if (!a || !b) return false;
  return normalizeWorkspacePath(a) === normalizeWorkspacePath(b);
}
