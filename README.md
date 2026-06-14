# Mac Agent Cockpit

A Mac-only Tauri + Preact app for running Cursor agents through ACP with Rust as the application kernel and TypeScript as a thin view-model renderer.

## Phase coverage

| Phase | Code |
| --- | --- |
| 1. Rust-owned UI state skeleton | `crates/app-core` owns `AppState`, `AppEvent`, `ViewModel`, patch diffing, and effect planning. `src/` holds only a patch cache and presentational Preact components. |
| 2. SQLite and conversations | `crates/app-storage` sets SQLite WAL pragmas, creates conversations/messages/rendered_blocks/acp_events/FTS5 schema, and exposes hot-path load/write/search helpers. |
| 3. Cursor ACP | `crates/app-agent` spawns `agent acp`, sends newline JSON-RPC requests, streams `session/update`, handles `session/request_permission`, and supports prompt/cancel/permission responses. |
| 4. Mac process supervisor | `crates/app-process` owns process groups, `ps`-based sampling, pause/resume/kill, budget changes, and SIGSTOP/SIGCONT duty-cycle throttling. |
| 5. Workspace and diffs | `crates/app-workspace` implements lazy directory loading, bounded file previews, git changed-file listing, and selected-file diffs. |
| 6. Local preview | `crates/app-preview` tracks preview lifecycle, port detection, and preview suspend/destroy state. The Tauri bridge emits preview state changes to the UI. |

## Run on macOS

```bash
npm install
npm run dev
```

Before using Cursor ACP:

```bash
agent login
```

The app starts `agent acp` itself when a new conversation is created.

## Architecture loop

```text
UI event -> Rust AppEvent -> AppState transition -> ViewModelPatch[] -> TS patch cache -> render
ACP/process/storage/file event -> Rust system AppEvent -> AppState transition -> ViewModelPatch[] -> render
```

## Notes

This is an MVP implementation scaffold with real boundaries and working core logic. Production hardening work should include deeper ACP schema coverage, robust process-tree edge-case handling on macOS, type generation via `specta`/`ts-rs`, and full integration tests against the installed Cursor CLI.
