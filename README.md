# InputFlow

**Your Computer. Your Controls.**

InputFlow is a free, open-source, offline-first desktop utility for macOS (and, in progress, Windows). It adds a universal **Shift + Right Click** menu with writing tools, a private clipboard history, a text expander, on-device screen OCR, window snapping, a quick launcher and developer utilities. Your ChatGPT account is optional. If you connect it, InputFlow can rewrite, translate and explain text.

- No subscription, ads, account, telemetry or cloud backend.
- Everything except ChatGPT actions runs locally.
- ChatGPT access uses OpenAI's official **Sign in with ChatGPT**. You never paste an API key.

> **Status: early development (v0.1.0).** See [DEVELOPMENT_PROGRESS.md](DEVELOPMENT_PROGRESS.md) for what works, what is planned and what has been verified. See [PLATFORM_COMPATIBILITY.md](PLATFORM_COMPATIBILITY.md) for per-feature, per-OS status.

## What works in this build (macOS)

| Area | Status |
|---|---|
| Shift + Right Click context menu (selection / text field / anywhere) | Implemented; needs Accessibility + Input Monitoring |
| AI writing actions (polish, grammar, tone, translate, summarize, explain, custom actions) with streaming preview, diff, regenerate, replace, undo | Implemented; needs a connected ChatGPT account |
| Write with ChatGPT (generate into a text field) | Implemented |
| Smart clipboard (opt-in, encrypted at rest, search, pin, favorite, retention, exclusions) | Implemented |
| Text expander (abbreviations, dynamic variables, import/export, per-app) | Implemented |
| Screen OCR (Apple Vision, on-device) | Implemented; needs Screen Recording |
| Screenshots (area / window / full, copy and/or save) | Implemented; annotation planned |
| Window manager (halves, quarters, maximize, center, next display) | Implemented; needs Accessibility |
| Quick launcher (apps, actions, snippets, calculator, unit conversion) | Implemented |
| Developer tools (JSON/XML, Base64, URL, hashes, timestamps, colours, regex, naming cases…) | Implemented |
| Read aloud (offline system voices) | Implemented |
| Application profiles, excluded apps, menu customization, appearance, Privacy Center | Implemented |
| Keyboard / Mouse / Trackpad Studio, Voice Anywhere, Automation Studio | **Planned**, with honest status pages in the app |

## Build from source (macOS)

Requirements: Xcode Command Line Tools (Swift 6+), Rust stable (`brew install rustup && rustup-init`), Node 20+.

```bash
npm install
```

```bash
npm run app:dev
```

`app:dev` compiles the Swift helper, then starts `tauri dev`. To produce an app bundle and DMG:

```bash
npm run app:build
```

To run all tests:

```bash
cargo test --workspace && npm test && npx tsc -b
```

### Permissions (macOS)

InputFlow asks for a permission only when you turn on the feature that needs it:

- **Accessibility**: read the selection, insert text, move windows.
- **Input Monitoring**: detect Shift + Right Click and snippet abbreviations.
- **Screen Recording**: OCR and screenshots, and only when you start a capture.

On managed (MDM) Macs these may require an administrator.

## Project layout

```
crates/inputflow-core/   Platform-independent engine (actions, menu, settings, calculator, text/dev tools, snippets, prompts)
src-tauri/               Tauri 2 app: runtime, IPC commands, ChatGPT auth/inference, SQLite, tray, shortcuts, platform adapters
native/macos/            Swift helper (Accessibility, CGEventTap, Vision OCR, AVSpeech, window management, pasteboard)
src/                     React + TypeScript UI (dashboard + overlay surfaces)
dev/                     Development-only UI harness (fake IPC; never shipped)
scripts/                 Build helpers (helper sidecar, icons)
```

Read [ARCHITECTURE.md](ARCHITECTURE.md) for the design, [SECURITY.md](SECURITY.md) and [PRIVACY.md](PRIVACY.md) for the threat model and data handling, and [TESTING.md](TESTING.md) for test coverage.

## License

MIT OR Apache-2.0.
