type Props =
  | { kind: "loading" }
  | { kind: "error"; message: string };

export function SplashScreen(props: Props) {
  if (props.kind === "error") {
    return (
      <main class="splash-screen splash-screen--error">
        <h1>Could not start Agent Cockpit</h1>
        <p>{props.message}</p>
      </main>
    );
  }

  return (
    <main class="splash-screen">
      <div class="splash-screen__card">
        <div class="splash-screen__logo">
          <img src="/icon.png" width="72" height="72" alt="" />
        </div>
        <h1 class="splash-screen__title">Agent Cockpit</h1>
        <p class="splash-screen__tagline">Starting engine…</p>
        <div class="splash-screen__spinner" aria-hidden="true" />
      </div>
    </main>
  );
}
