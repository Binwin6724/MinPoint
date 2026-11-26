import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";
import { StoreProvider } from "./main/store";
import { MainApp } from "./main/App";
import { OverlayApp } from "./overlay/Overlay";
import { ErrorBoundary } from "./components/ErrorScreen";

const isOverlay = window.location.hash.startsWith("#/overlay");
document.documentElement.dataset.surface = isOverlay ? "overlay" : "main";

// Make startup problems visible instead of leaving a blank window.
const root = document.getElementById("root")!;
window.addEventListener("unhandledrejection", (e) => console.error("[MinPoint] unhandled rejection:", e.reason));
window.addEventListener("error", (e) => console.error("[MinPoint] error:", e.message));

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
