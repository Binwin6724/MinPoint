# Implementation Plan

| Phase | Scope | Status |
|---|---|---|
| 1 Foundation | Workspace, Tauri shell, helper sidecar, adapter trait, action registry, settings, SQLite, secrets, tray, permissions | Done |
| 2 Context menu | Shift + Right Click tap, context detection, menu resolver, overlay, selection read/insert, preview, undo, menu customization | Done (manual verification pending) |
| 3 ChatGPT | Sign in with ChatGPT, Keychain, refresh/revoke, models, streaming transforms, preview, Write with ChatGPT, custom actions | Done (live sign-in untested) |
| 4 Voice Anywhere | AVAudioEngine capture, whisper.cpp + Metal, model manager with checksums, overlay, AI cleanup modes | Next |
| 5 Productivity | Clipboard, expander, OCR, read aloud, screenshots, calculator, dev tools, launcher | Done (annotation pending) |
| 6 Input customization | Key/modifier remap, mouse buttons/scroll, capability-aware gestures, conflict detection, emergency stop | Partial (shortcuts, conflicts, emergency pause) |
| 7 Automation & windows | Snapping (done), saved layouts, macro recorder/builder with limits, profiles (done), custom actions (done) | Partial |
| 8 Hardening | Windows adapter, signing/notarization, updater, benchmarks, accessibility audit, compatibility matrix | Pending |

## Windows adapter design (Phase 8)

- **Shift + Right Click**: a `WH_MOUSE_LL` hook on a dedicated thread. Swallow `WM_RBUTTONDOWN`/`UP` only when Shift is down.
- **Selection**: UI Automation `TextPattern.GetSelection`, with a guarded Ctrl+C fallback.
- **Insertion**: `ValuePattern`/`TextPattern2`, or `SendInput` paste with clipboard restore.
- **Elevated apps**: skipped. UIPI blocks them; we don't bypass it.
- **Remaining pieces**:
  - OCR via `Windows.Media.Ocr`
  - capture via Windows Graphics Capture
  - TTS via `Windows.Media.SpeechSynthesis`
  - windows via `SetWindowPos` with per-monitor DPI awareness
  - credentials via Credential Manager (already wired through `keyring`)
