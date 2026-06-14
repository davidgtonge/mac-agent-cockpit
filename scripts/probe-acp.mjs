#!/usr/bin/env node
/**
 * Probe Cursor `agent acp` JSON-RPC responses (initialize → authenticate → session/new).
 */
import { spawn } from "node:child_process";
import readline from "node:readline";

const cwd = process.argv[2] ?? process.cwd();
const agentBin = process.env.CURSOR_AGENT_BINARY ?? "agent";

const child = spawn(agentBin, ["acp"], {
  cwd,
  stdio: ["pipe", "pipe", "inherit"],
});

let nextId = 1;
const pending = new Map();

function send(method, params) {
  const id = nextId++;
  const msg = JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n";
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    child.stdin.write(msg);
  });
}

const rl = readline.createInterface({ input: child.stdout });
rl.on("line", (line) => {
  if (!line.trim()) return;
  let raw;
  try {
    raw = JSON.parse(line);
  } catch {
    console.error("NON-JSON:", line.slice(0, 200));
    return;
  }
  if (raw.id != null && (raw.result !== undefined || raw.error !== undefined)) {
    const handler = pending.get(raw.id);
    if (handler) {
      pending.delete(raw.id);
      if (raw.error) handler.reject(new Error(JSON.stringify(raw.error)));
      else handler.resolve(raw.result);
    }
    return;
  }
  // notifications
  if (raw.method) {
    console.error("\n[notification]", raw.method, JSON.stringify(raw.params)?.slice(0, 300));
  }
});

async function main() {
  console.log(`Probing agent acp in ${cwd}\n`);

  const init = await send("initialize", {
    protocolVersion: 1,
    clientCapabilities: { fs: { readTextFile: false, writeTextFile: false }, terminal: false },
    clientInfo: { name: "mac-agent-cockpit-probe", version: "0.1.0" },
  });
  console.log("=== initialize ===");
  console.log(JSON.stringify(init, null, 2));

  const auth = await send("authenticate", { methodId: "cursor_login" });
  console.log("\n=== authenticate ===");
  console.log(JSON.stringify(auth, null, 2));

  const session = await send("session/new", { cwd, mcpServers: [] });
  console.log("\n=== session/new (full) ===");
  console.log(JSON.stringify(session, null, 2));

  if (session.configOptions) {
    console.log("\n=== configOptions summary ===");
    for (const opt of session.configOptions) {
      const items = opt.options?.items ?? opt.options?.groups?.flatMap((g) => g.items ?? []) ?? [];
      console.log(`- id=${opt.id} category=${opt.category ?? "(none)"} type=${opt.type} current=${opt.currentValue} items=${items.length}`);
      for (const item of items.slice(0, 8)) {
        console.log(`    · ${item.value ?? item.id}: ${item.label ?? item.name ?? ""}`);
      }
      if (items.length > 8) console.log(`    … +${items.length - 8} more`);
    }
  }

  if (session.modes) {
    console.log("\n=== modes (legacy) ===");
    console.log(JSON.stringify(session.modes, null, 2));
  }

  child.stdin.end();
  child.kill();
  process.exit(0);
}

main().catch((err) => {
  console.error("PROBE FAILED:", err.message ?? err);
  child.kill();
  process.exit(1);
});
