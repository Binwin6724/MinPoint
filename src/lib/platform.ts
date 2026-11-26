// Platform-aware wording and capability flags for the UI.
export type Os = "macos" | "windows" | string;

export const trayName = (os: Os) => (os === "macos" ? "menu bar" : "system tray");

/** Pages whose features need MinPoint's native integration (macOS only for now). */
export const NATIVE_PAGES = new Set(["actions", "writing", "clipboard", "snippets", "ocr", "screenshots", "windows"]);
