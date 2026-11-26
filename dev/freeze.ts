// Dev-only: reproduce Tauri's `freezePrototype: true` in the harness.
if (new URLSearchParams(location.search).get("freeze") === "1") Object.freeze(Object.prototype);
