import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";
import { StoreProvider } from "./main/store";
import { MainApp } from "./main/App";
import { OverlayApp } from "./overlay/Overlay";
import { ErrorBoundary } from "./components/ErrorScreen";
import { api } from "./lib/api";

const isOverlay = window.location.hash.startsWith("#/overlay");
document.documentElement.dataset.surface = isOverlay ? "overlay" : "main";

// Make startup problems visible instead of leaving a blank window.
const root = document.getElementById("root")!;
const describe = (x: unknown) => (x instanceof Error ? `${x.message}\n${x.stack ?? ""}` : String(x));
window.addEventListener("unhandledrejection", (e) => {
  console.error("[MinPoint] unhandled rejection:", e.reason);
  api.logFrontend("error", `unhandled rejection (${isOverlay ? "overlay" : "main"}): ${describe(e.reason)}`);
});
window.addEventListener("error", (e) => {
  console.error("[MinPoint] error:", e.message);
  api.logFrontend("error", `error (${isOverlay ? "overlay" : "main"}): ${e.message} at ${e.filename}:${e.lineno}`);
});
api.logFrontend("info", `ui started (${isOverlay ? "overlay" : "main"}) ${navigator.userAgent}`);

createRoot(root).render(
  <StrictMode>
    <ErrorBoundary>
      {isOverlay ? (
        <OverlayApp />
      ) : (
        <StoreProvider>
          {(ready) =>
            ready ? <MainApp /> : <div style={{ display: "grid", placeItems: "center", height: "100%", color: "#9aa2b5", fontFamily: "system-ui, sans-serif", fontSize: 13 }}>Starting MinPoint…</div>
          }
        </StoreProvider>
      )}
    </ErrorBoundary>
  </StrictMode>,
);
