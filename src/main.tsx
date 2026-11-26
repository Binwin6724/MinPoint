import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";
import { StoreProvider } from "./main/store";
import { MainApp } from "./main/App";
import { OverlayApp } from "./overlay/Overlay";

const isOverlay = window.location.hash.startsWith("#/overlay");
document.documentElement.dataset.surface = isOverlay ? "overlay" : "main";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {isOverlay ? <OverlayApp /> : <StoreProvider>{(ready) => (ready ? <MainApp /> : null)}</StoreProvider>}
  </StrictMode>,
);
