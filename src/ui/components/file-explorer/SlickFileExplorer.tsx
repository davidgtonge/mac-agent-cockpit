import type { JSX } from "preact";
import { useCallback, useEffect, useMemo, useRef, useState } from "preact/hooks";
import type { AppEvent, FileNode, FileTreeVm, GitFileStatus, ProjectId } from "../../../generated/engine-types";

export type SlickFileExplorerInput = {
  fileTree: FileTreeVm;
  projectId: ProjectId | null;
};

export type SlickFileExplorerEvent = Extract<
  AppEvent,
  | { type: "fileTreeNodeExpanded" }
  | { type: "fileTreeNodeCollapsed" }
  | { type: "fileSelected" }
>;

type ExplorerRow = {
  id: string;
  path: string;
  name: string;
  isDir: boolean;
  ignored: boolean;
  depth: number;
  selected: boolean;
  expanded: boolean;
  loaded: boolean;
  sizeBytes?: number | null;
  gitStatus?: GitFileStatus | null;
  changeCount?: number | null;
  synthetic?: boolean;
};

type SlickFileExplorerProps = {
  input: SlickFileExplorerInput;
  onEvent: (event: SlickFileExplorerEvent) => void;
  class?: string;
  variant?: "default" | "sidebar";
};

const ROW_HEIGHT = 24;
const OVERSCAN = 10;
const ROOT_PATH = ".";

export function SlickFileExplorerView({ input, onEvent, class: className, variant = "default" }: SlickFileExplorerProps) {
  const { fileTree, projectId } = input;
  const viewportRef = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState("");
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(400);
  const [activeIndex, setActiveIndex] = useState(0);

  const rows = useMemo(() => buildRows(fileTree), [fileTree]);
  const visibleRows = useMemo(() => filterRows(rows, query), [rows, query]);

  const totalHeight = visibleRows.length * ROW_HEIGHT;
  const startIndex = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const endIndex = Math.min(visibleRows.length, Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN);
  const windowRows = visibleRows.slice(startIndex, endIndex);

  useEffect(() => {
    const element = viewportRef.current;
    if (!element) return;

    const update = () => setViewportHeight(element.clientHeight || 400);
    update();

    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    setActiveIndex((index) => clamp(index, 0, Math.max(visibleRows.length - 1, 0)));
  }, [visibleRows.length]);

  const ensureRowVisible = useCallback((index: number) => {
    const element = viewportRef.current;
    if (!element) return;

    const top = index * ROW_HEIGHT;
    const bottom = top + ROW_HEIGHT;
    if (top < element.scrollTop) {
      element.scrollTop = top;
    } else if (bottom > element.scrollTop + element.clientHeight) {
      element.scrollTop = bottom - element.clientHeight;
    }
  }, []);

  const openRow = useCallback((row: ExplorerRow) => {
    if (!projectId) return;
    if (row.isDir) {
      if (row.expanded) {
        onEvent({ type: "fileTreeNodeCollapsed", projectId, path: row.path });
      } else {
        onEvent({ type: "fileTreeNodeExpanded", projectId, path: row.path });
      }
    } else {
      onEvent({ type: "fileSelected", projectId, path: row.path });
    }
  }, [onEvent, projectId]);

  const onKeyDown = useCallback((event: KeyboardEvent) => {
    if (!visibleRows.length) return;

    if (event.key === "ArrowDown") {
      event.preventDefault();
      const next = clamp(activeIndex + 1, 0, visibleRows.length - 1);
      setActiveIndex(next);
      ensureRowVisible(next);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      const next = clamp(activeIndex - 1, 0, visibleRows.length - 1);
      setActiveIndex(next);
      ensureRowVisible(next);
    } else if (event.key === "Home") {
      event.preventDefault();
      setActiveIndex(0);
      ensureRowVisible(0);
    } else if (event.key === "End") {
      event.preventDefault();
      const next = visibleRows.length - 1;
      setActiveIndex(next);
      ensureRowVisible(next);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      openRow(visibleRows[activeIndex]);
    } else if (event.key === "ArrowRight") {
      const row = visibleRows[activeIndex];
      if (row?.isDir && !row.expanded && projectId) {
        event.preventDefault();
        onEvent({ type: "fileTreeNodeExpanded", projectId, path: row.path });
      }
    } else if (event.key === "ArrowLeft") {
      const row = visibleRows[activeIndex];
      if (row?.isDir && row.expanded && projectId) {
        event.preventDefault();
        onEvent({ type: "fileTreeNodeCollapsed", projectId, path: row.path });
      }
    }
  }, [activeIndex, onEvent, ensureRowVisible, openRow, projectId, visibleRows]);

  if (!projectId) {
    return (
      <section class={cn("flex min-h-0 flex-1 items-center justify-center px-6 text-center text-sm text-zinc-500", className)}>
        Select a project to browse files.
      </section>
    );
  }

  if (fileTree.expanded.length === 0) {
    return (
      <section class={cn("flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-6 text-center", className)}>
        <div class="grid size-11 place-items-center rounded-2xl border border-white/10 bg-white/[0.035] shadow-2xl shadow-black/30">
          <FolderIcon class="size-5 text-zinc-300" />
        </div>
        <div>
          <div class="text-sm font-medium text-zinc-200">File tree not loaded</div>
          <div class="mt-1 max-w-56 text-xs leading-5 text-zinc-500">Load the workspace root from Rust and stream only visible slices to the UI.</div>
        </div>
        <button
          class="rounded-xl border border-white/10 bg-white/[0.055] px-3 py-1.5 text-xs font-medium text-zinc-200 shadow-sm hover:bg-white/[0.085] active:scale-[0.99]"
          onClick={() => onEvent({ type: "fileTreeNodeExpanded", projectId, path: ROOT_PATH })}
        >
          Load files
        </button>
      </section>
    );
  }

  return (
    <section class={cn("flex min-h-0 flex-1 flex-col overflow-hidden", variant === "sidebar" ? "bg-transparent" : "bg-[#0b0c10]", className)}>
      {variant !== "sidebar" && (
        <div class="border-b border-white/[0.075] bg-gradient-to-b from-white/[0.045] to-transparent px-2.5 py-2">
          <div class="group flex h-8 items-center gap-2 rounded-xl border border-white/[0.075] bg-black/20 px-2.5 shadow-inner shadow-black/30 focus-within:border-sky-400/35 focus-within:bg-black/25">
            <SearchIcon class="size-3.5 shrink-0 text-zinc-500 group-focus-within:text-zinc-300" />
            <input
              class="h-full min-w-0 flex-1 border-0 bg-transparent p-0 text-xs text-zinc-200 placeholder:text-zinc-600 focus:outline-none"
              value={query}
              onInput={(event) => setQuery((event.currentTarget as HTMLInputElement).value)}
              placeholder="Filter loaded files"
            />
            {query && (
              <button
                class="rounded-md border-0 bg-transparent px-1.5 py-0.5 text-[11px] text-zinc-500 hover:bg-white/10 hover:text-zinc-200"
                onClick={() => setQuery("")}
                aria-label="Clear file filter"
              >
                esc
              </button>
            )}
          </div>
          <div class="mt-2 flex items-center justify-between px-1 text-[10px] uppercase tracking-[0.16em] text-zinc-600">
            <span>{query ? `${visibleRows.length} matches` : `${rows.length} loaded`}</span>
            <span>virtualized</span>
          </div>
        </div>
      )}

      <div
        ref={viewportRef}
        class="mac-scrollbar min-h-0 flex-1 overflow-auto outline-none"
        tabIndex={0}
        onScroll={(event) => setScrollTop((event.currentTarget as HTMLDivElement).scrollTop)}
        onKeyDown={onKeyDown as unknown as (event: JSX.TargetedKeyboardEvent<HTMLDivElement>) => void}
        role="tree"
        aria-label="Workspace files"
      >
        {visibleRows.length === 0 ? (
          <div class="grid h-full place-items-center px-6 text-center text-sm text-zinc-500">No loaded files match “{query}”.</div>
        ) : (
          <div class="relative" style={{ height: `${totalHeight}px` }}>
            <div class="absolute inset-x-0 top-0" style={{ transform: `translateY(${startIndex * ROW_HEIGHT}px)` }}>
              {windowRows.map((row, offset) => {
                const index = startIndex + offset;
                return (
                  <ExplorerRowView
                    key={row.id}
                    row={row}
                    active={index === activeIndex}
                    onHover={() => setActiveIndex(index)}
                    onOpen={() => openRow(row)}
                  />
                );
              })}
            </div>
          </div>
        )}
      </div>
    </section>
  );
}

function ExplorerRowView({ row, active, onHover, onOpen }: { row: ExplorerRow; active: boolean; onHover: () => void; onOpen: () => void }) {
  return (
    <button
      class={cn(
        "group grid w-full grid-cols-[1fr_auto] items-center gap-2 border-0 bg-transparent px-2 text-left text-[11px] leading-none text-zinc-300 outline-none transition-colors duration-75",
        "hover:bg-white/[0.055]",
        active && "bg-sky-400/[0.105] text-zinc-100 ring-1 ring-inset ring-sky-300/[0.11]",
        row.selected && "bg-sky-400/[0.155] text-white ring-1 ring-inset ring-sky-300/20",
        row.ignored && "text-zinc-600"
      )}
      style={{ height: `${ROW_HEIGHT}px`, paddingLeft: `${6 + row.depth * 13}px` }}
      onMouseEnter={onHover}
      onClick={onOpen}
      role="treeitem"
      aria-selected={row.selected}
      aria-expanded={row.isDir ? row.expanded : undefined}
    >
      <span class="flex min-w-0 items-center gap-1.5">
        <span class="grid size-3.5 shrink-0 place-items-center text-zinc-500">
          {row.isDir ? (
            <ChevronIcon class={cn("size-3 transition-transform duration-150", row.expanded && "rotate-90")} />
          ) : null}
        </span>
        <FileGlyph row={row} />
        <span class="min-w-0 truncate">{row.name}</span>
        {row.isDir && !row.loaded && <span class="rounded bg-white/[0.055] px-1 py-0.5 text-[9px] uppercase tracking-wide text-zinc-500">lazy</span>}
      </span>

      <span class="flex shrink-0 items-center gap-1.5 opacity-80 group-hover:opacity-100">
        {row.isDir && row.changeCount != null && row.changeCount > 0 ? (
          <span class="rounded bg-amber-400/10 px-1 py-0.5 text-[9px] tabular-nums text-amber-200/90">{row.changeCount}</span>
        ) : null}
        {row.gitStatus && row.gitStatus !== "clean" ? <GitBadge status={row.gitStatus} /> : null}
        {row.synthetic ? <span class="text-[9px] uppercase tracking-wide text-zinc-500">git</span> : null}
        {!row.isDir && row.sizeBytes != null && <span class="text-[10px] tabular-nums text-zinc-600">{formatCompactBytes(row.sizeBytes)}</span>}
      </span>
    </button>
  );
}

function FileGlyph({ row }: { row: ExplorerRow }) {
  if (row.isDir) {
    return row.expanded ? <FolderOpenIcon class="size-3.5 shrink-0 text-sky-300/90" /> : <FolderIcon class="size-3.5 shrink-0 text-sky-300/80" />;
  }

  const extension = row.name.includes(".") ? row.name.split(".").pop()?.toLowerCase() : "";
  if (extension === "tsx" || extension === "ts" || extension === "jsx" || extension === "js") return <CodeIcon class="size-3.5 shrink-0 text-amber-300/90" />;
  if (extension === "rs") return <CodeIcon class="size-3.5 shrink-0 text-orange-300/90" />;
  if (extension === "css" || extension === "scss") return <PaletteIcon class="size-3.5 shrink-0 text-fuchsia-300/90" />;
  if (extension === "json" || extension === "toml" || extension === "yaml" || extension === "yml") return <BracesIcon class="size-3.5 shrink-0 text-emerald-300/90" />;
  if (extension === "md" || extension === "mdx") return <TextIcon class="size-3.5 shrink-0 text-violet-300/90" />;
  return <FileIcon class="size-3.5 shrink-0 text-zinc-400" />;
}

function GitBadge({ status }: { status: GitFileStatus }) {
  const label = statusLabel(status);
  return <span class={cn("size-1.5 rounded-full", statusClass(status))} title={label} aria-label={label} />;
}

function buildRows(fileTree: FileTreeVm): ExplorerRow[] {
  const directories = new Map<string, FileNode[]>();
  for (const dir of fileTree.expanded) directories.set(normalizePath(dir.path), sortNodes(dir.children));

  const expandedPaths = new Set(directories.keys());
  const rows: ExplorerRow[] = [];
  const visited = new Set<string>();

  const rootChildren = directories.get(ROOT_PATH) ?? directories.get("");
  if (rootChildren) {
    for (const child of rootChildren) appendNode(child, 0);
  } else {
    for (const [dir, children] of directories) {
      if (visited.has(dir)) continue;
      for (const child of children) appendNode(child, depthForPath(dir));
    }
  }

  return rows;

  function appendNode(node: FileNode, depth: number) {
    const path = normalizePath(node.path);
    if (visited.has(path)) return;
    visited.add(path);

    const expanded = node.isDir && expandedPaths.has(path);
    rows.push({
      id: path || node.name,
      path,
      name: node.name || basename(path),
      isDir: node.isDir,
      ignored: node.ignored,
      depth,
      selected: normalizePath(fileTree.selectedPath ?? "") === path,
      expanded,
      loaded: node.isDir ? expandedPaths.has(path) : true,
      sizeBytes: node.sizeBytes,
      gitStatus: node.gitStatus ?? null,
      changeCount: node.changeCount ?? null,
      synthetic: node.synthetic ?? false,
    });

    if (expanded) {
      const children = directories.get(path) ?? [];
      for (const child of children) appendNode(child, depth + 1);
    }
  }
}

function filterRows(rows: ExplorerRow[], query: string): ExplorerRow[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return rows;
  const parts = needle.split(/\s+/).filter(Boolean);
  return rows.filter((row) => {
    const target = `${row.name} ${row.path}`.toLowerCase();
    return parts.every((part) => fuzzyIncludes(target, part));
  });
}

function fuzzyIncludes(target: string, needle: string): boolean {
  let cursor = 0;
  for (const char of needle) {
    cursor = target.indexOf(char, cursor);
    if (cursor === -1) return false;
    cursor += 1;
  }
  return true;
}

function sortNodes(nodes: FileNode[]): FileNode[] {
  return [...nodes].sort((a, b) => {
    if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
    return a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" });
  });
}

function normalizePath(path: string): string {
  const cleaned = path.replace(/\\/g, "/").replace(/^\.\//, "").replace(/\/+$/g, "");
  return cleaned === "" ? ROOT_PATH : cleaned;
}

function basename(path: string): string {
  const normalized = normalizePath(path);
  if (normalized === ROOT_PATH) return ROOT_PATH;
  return normalized.split("/").filter(Boolean).pop() ?? normalized;
}

function depthForPath(path: string): number {
  const normalized = normalizePath(path);
  if (normalized === ROOT_PATH) return 0;
  return normalized.split("/").filter(Boolean).length;
}

function formatCompactBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)}g`;
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)}m`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)}k`;
  return `${bytes}b`;
}

function statusLabel(status: GitFileStatus): string {
  switch (status) {
    case "modified": return "Modified";
    case "added": return "Added";
    case "deleted": return "Deleted";
    case "renamed": return "Renamed";
    case "untracked": return "Untracked";
    case "conflicted": return "Conflicted";
    case "binary": return "Binary";
    default: return status;
  }
}

function statusClass(status: GitFileStatus): string {
  switch (status) {
    case "added": return "bg-emerald-400 shadow-[0_0_10px_rgba(52,211,153,0.7)]";
    case "deleted": return "bg-rose-400 shadow-[0_0_10px_rgba(251,113,133,0.65)]";
    case "renamed": return "bg-violet-400 shadow-[0_0_10px_rgba(167,139,250,0.65)]";
    case "untracked": return "bg-sky-400 shadow-[0_0_10px_rgba(56,189,248,0.65)]";
    case "conflicted": return "bg-orange-400 shadow-[0_0_10px_rgba(251,146,60,0.65)]";
    default: return "bg-amber-300 shadow-[0_0_10px_rgba(252,211,77,0.65)]";
  }
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function cn(...classes: Array<string | false | null | undefined>): string {
  return classes.filter(Boolean).join(" ");
}

type IconProps = { class?: string };

function ChevronIcon({ class: className }: IconProps) {
  return <svg class={className} viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>;
}

function SearchIcon({ class: className }: IconProps) {
  return <svg class={className} viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M7.2 12.2a5 5 0 1 0 0-10 5 5 0 0 0 0 10ZM11 11l3 3" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>;
}

function FolderIcon({ class: className }: IconProps) {
  return <svg class={className} viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M1.8 4.6c0-.9.7-1.6 1.6-1.6h3l1.4 1.5h4.8c.9 0 1.6.7 1.6 1.6v5.8c0 .9-.7 1.6-1.6 1.6H3.4c-.9 0-1.6-.7-1.6-1.6V4.6Z" fill="currentColor" /></svg>;
}

function FolderOpenIcon({ class: className }: IconProps) {
  return <svg class={className} viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M1.7 6.2V4.7c0-1 .8-1.7 1.7-1.7h3l1.4 1.5h4.5c.9 0 1.6.7 1.6 1.6v.5H3.5c-.8 0-1.5.5-1.8 1.2Z" fill="currentColor" opacity=".75" /><path d="M2.3 7.2c.2-.7.8-1.1 1.5-1.1h10.4c.5 0 .8.5.7.9l-1.2 4.9c-.2.7-.8 1.1-1.5 1.1H2.4c-.5 0-.8-.5-.7-.9l.6-4.9Z" fill="currentColor" /></svg>;
}

function FileIcon({ class: className }: IconProps) {
  return <svg class={className} viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M4 1.8h5.2L12.8 5.4v8.8H4V1.8Z" fill="currentColor" opacity=".25" /><path d="M9.2 1.8v3.6h3.6M4 1.8h5.2L12.8 5.4v8.8H4V1.8Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" /></svg>;
}

function CodeIcon({ class: className }: IconProps) {
  return <svg class={className} viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M5.9 4.2 2.8 8l3.1 3.8M10.1 4.2 13.2 8l-3.1 3.8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>;
}

function PaletteIcon({ class: className }: IconProps) {
  return <svg class={className} viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M8 2a6 6 0 0 0-1.5 11.8c.7.2 1.1-.5.8-1.1-.4-.8.2-1.7 1.1-1.7h1.1A4.5 4.5 0 0 0 14 6.5C14 4 11.3 2 8 2Z" stroke="currentColor" stroke-width="1.2" /><path d="M5.2 6.1h.1M7.4 4.7h.1M10 4.9h.1M11.7 7h.1" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></svg>;
}

function BracesIcon({ class: className }: IconProps) {
  return <svg class={className} viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M6.2 3.2c-1.5 0-2 .8-2 2v1.1c0 .7-.3 1.2-1 1.2.7 0 1 .5 1 1.2v1.1c0 1.2.5 2 2 2M9.8 3.2c1.5 0 2 .8 2 2v1.1c0 .7.3 1.2 1 1.2-.7 0-1 .5-1 1.2v1.1c0 1.2-.5 2-2 2" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" /></svg>;
}

function TextIcon({ class: className }: IconProps) {
  return <svg class={className} viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M4 3.5h8M4 6.5h8M4 9.5h5.5M4 12.5h7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" /></svg>;
}
