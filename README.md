# Mac Agent Cockpit

A Mac-only Tauri app for running multiple [Cursor](https://cursor.com) agents through ACP — with a **Rust application kernel** and a **thin Preact UI**.

**Status:** alpha (`0.1.0-alpha.1`)

## Why this exists

### 1. A real app for the Rust/TypeScript split

The browser demos ([rust-tetris](https://github.com/davidgtonge/rust-tetris), [rust-weather-spiral](https://github.com/davidgtonge/rust-weather-spiral)) prove the loop. Agent Cockpit is where I wanted it for actual work.

The whole product is orchestration: conversations, permissions, streaming ACP messages, git overlays, preview lifecycle, process budgets. That is state-machine territory. I do not want it spread across Preact hooks while agents are also trying to help.

So the shape is deliberate:

```txt
Preact UI event  →  AppEvent  →  Engine::handle_input  →  ViewModelPatch[]  →  applyPatches  →  render
ACP / SQLite / process / file event  →  system AppEvent  →  same loop
```

`app-core` owns `AppState`, events, view-model projection, and effect planning. TypeScript is a patch cache and presentational components. `dispatchAppEvent` is a Tauri `invoke`. `bootEngine` listens for `engine://patches` and applies diffs. That is most of the frontend orchestration.

Effects are where native I/O lives: spawn `agent acp`, write to SQLite, load directory previews, sample process CPU, watch workspace dirty state, run git diffs, manage local preview servers. Rust plans. The Tauri shell performs.

No Wasm worker here — the kernel is native Rust in-process. The boundary is Tauri IPC instead of `postMessage`. The discipline is the same: **boring TypeScript, authoritative Rust, explicit effects.**

Write-up: *Make TypeScript Boring* — Mac Agent Cockpit section (same architecture essay as the [rust-tetris](https://github.com/davidgtonge/rust-tetris) / [engine-shell](https://github.com/davidgtonge/engine-shell) demos). Scaffold lineage: [engine-shell](https://github.com/davidgtonge/engine-shell).

### 2. CPU control when running several agents

Running two or three Cursor agents at once is useful. Running `cargo build`, `tsc --noEmit`, and a dev server across all of them at the same time is not — the machine grinds to a halt and everything becomes unusable.

Cursor does not give you per-agent CPU limits. Agent Cockpit does.

Each conversation gets a **process group** tracked by `app-process`:

- Samples the full tree (`agent acp` plus descendants) via `ps`
- **Pause / resume / kill** per conversation
- **Per-agent CPU budget** — when usage exceeds the cap, the supervisor applies **SIGSTOP/SIGCONT duty-cycle throttling** so compiles and type-checks back off instead of starving the rest of the machine
- Live CPU and memory stats in the UI

You can keep agents working without letting one `cargo check` own every core.

## What it does

| Area | Crate | Notes |
| --- | --- | --- |
| Application kernel | `app-core` | `AppState`, `AppEvent`, `ViewModel`, patch diffing, effect planning |
| Persistence | `app-storage` | SQLite WAL, conversations, messages, FTS5 search, ACP event log |
| Cursor ACP | `app-agent` | Spawns `agent acp`, JSON-RPC, streaming `session/update`, permissions |
| Process supervision | `app-process` | Process groups, sampling, budgets, pause/resume/kill, throttling |
| Workspace | `app-workspace` | Lazy directory tree, file previews, git status/diffs, file watcher |
| Local preview | `app-preview` | Dev-server preview lifecycle, suspend/destroy |
| Shell | `src-tauri` | Tauri bridge, effect execution, background event pump |

## Quick start

Requires macOS 13+, [Rust](https://rustup.rs/), Node.js, and the Cursor CLI (`agent`).

```bash
git clone https://github.com/davidgtonge/mac-agent-cockpit.git
cd mac-agent-cockpit
npm install
npm run dev
```

Before using ACP:

```bash
agent login
```

The app starts `agent acp` when you create a new conversation.

### Build a release

```bash
npm run build    # .app + .dmg in src-tauri/target/release/bundle/
```

### Regenerate icons

Edit `scripts/icon-source.svg`, then:

```bash
npm run icons
```

## Architecture loop

```txt
UI event
  → Rust AppEvent
  → AppState transition
  → ViewModelPatch[]
  → TS patch cache
  → render

ACP / process / storage / file event
  → Rust system AppEvent
  → same loop
```

TypeScript does not own business rules. If you need new behaviour, you change the Rust engine and the view model — not a hook.

## Alpha caveats

This is an alpha with real boundaries and working core logic. Known gaps:

- Deeper ACP schema coverage
- Edge cases in macOS process-tree handling
- Generated boundary types (`specta` / `ts-rs`) instead of hand-maintained `engine-types.ts`
- Integration tests against the installed Cursor CLI

See [VALIDATION.md](./VALIDATION.md) for what has been run locally.

## Related projects

- [engine-shell](https://github.com/davidgtonge/engine-shell) — reusable Wasm worker + patch scaffold
- [rust-tetris](https://github.com/davidgtonge/rust-tetris) — same loop in the browser ([live demo](https://davidgtonge.github.io/rust-tetris/))
- [rust-weather-spiral](https://github.com/davidgtonge/rust-weather-spiral) — data-heavy canvas case ([live demo](https://davidgtonge.github.io/rust-weather-spiral/))
