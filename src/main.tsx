import { invoke } from "@tauri-apps/api/core";
import { render } from "preact";
import { useEffect, useState } from "preact/hooks";
import { bootEngine } from "./engine/ipc-client";
import { App } from "./ui/App";
import { SplashScreen } from "./ui/components/SplashScreen";
import "./styles.css";

async function revealMainWindow() {
  try {
    await invoke("app_ready");
  } catch {
    // Non-Tauri contexts (e.g. vite-only) skip window management.
  }
}

function Root() {
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    bootEngine()
      .then(async () => {
        await revealMainWindow();
        setReady(true);
      })
      .catch((e) => {
        void revealMainWindow();
        setError(String(e));
      });
  }, []);
  if (error) return <SplashScreen kind="error" message={error} />;
  if (!ready) return <SplashScreen kind="loading" />;
  return <App />;
}

render(<Root />, document.getElementById("app")!);
