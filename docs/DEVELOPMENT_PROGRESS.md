# Development Progress

_Last updated: 2026-11-25_

## Environment used

- macOS 27.2.1 on Apple Silicon (arm64), on a **managed company laptop**.
- Rust 1.100 (Homebrew rustup), Node 22, Swift 6.4 (Command Line Tools only; no Xcode).
- Accessibility, Input Monitoring and Screen Recording **cannot be granted on this machine without an administrator**. That limits manual verification (see below).

## 0.1.2: startup race, installer, Windows UI (2026-11-26)

- **Startup race:** config-created webviews can call `bootstrap` before `setup` has managed `Rt` (seen on a slow, managed Windows PC as "state not managed").
  - Fixed with `retryWhile(bootstrap)` in the UI, covered by unit tests in `src/lib/retry.test.ts`.
  - Rejected alternative: creating the windows from code after setup. It took about 7–14 s on Windows (WebView2 creation outside the config path), against about 0.2 s from config.
- **Installer:** NSIS hooks (`src-tauri/windows/hooks.nsh`) run `taskkill /F /T` before install and uninstall.
  - Verified on a GitHub Windows runner: installing over a running app exits 0 and leaves the app updated; uninstalling a running app exits 0 and removes it.
  - The installer and uninstaller icons were verified by extracting them from the built executables.
  - Upgrading *from 0.1.0* still runs the old 0.1.0 uninstaller, which has no hook. If that frozen 0.1.0 is still running, close it manually once (`taskkill /F /T /IM minpoint.exe`).

## 0.1.1: startup freeze fix (2026-11-26)

- **Root cause:** `secrets::database_key()` (Keychain / Credential Manager) ran synchronously in Tauri `setup` on the main thread. A Keychain prompt, or a slow or held Credential Manager, blocked it, so IPC never answered and the UI stayed blank.
- **Verified on this Mac via the new log:** before the fix, startup stopped after the first log line; after the fix, setup finishes in about 70 ms and the UI reports it started.
- **Windows verification:** the 0.1.0 installer and the fixed build both render on a clean GitHub Windows 11 runner (WebView2 157). The user's PC has WebView2 158, so WebView2 was ruled out.

## Rename & family style (2026-11-25)

- Renamed InputFlow to **MinPoint**: bundle `app.minpoint.desktop`, crate `minpoint-core`, helper `minpoint-helper`.
- Restyled to the Min-family design system (shared with MinTransfer and MinSoundFX), with an orange→pink accent.
- New family-style app icon (`scripts/make-icons.swift`).
- README, `docs/` layout, CONTRIBUTING, LICENSE (MIT) and CI follow the sibling repos.
- Per-architecture installers plus `SHA256SUMS.txt`.

## Completed

**Phase 1: Foundation**
- Workspace: `minpoint-core` crate, Tauri 2 app, Swift helper, React UI.
- Action registry, versioned settings with migration, SQLite with migrations.
- Keychain secrets, permission manager.
- Menu bar tray (pause/resume, launcher, clipboard, OCR, quit).
- Single-instance guard, launch at login, keep running on close.

**Phase 2: Context menu**
- Shift + Right Click event tap that consumes only Shift + Right Click; normal right-click is untouched.
- Context-aware menu resolver with favorites, order, submenus, profiles, exclusions and capability gating.
- AX selection read with a guarded Cmd+C fallback that restores the clipboard.
- Focus-verified insertion: AX write first, transient-paste fallback.
- Undo of the last replacement.

**Phase 3: ChatGPT**
- Official Sign in with ChatGPT: PKCE, loopback callback, dynamic client registration, JWKS ID-token verification, plan-scope check.
- Keychain storage, refresh, revoke.
- Model list, streaming `/v1/responses`, documented error mapping, usage-limit handling, Manage usage.
- Writing operations, prompt isolation, Write with ChatGPT templates, custom AI actions.

**Phase 5: Productivity**
- Clipboard history: opt-in, AES-GCM at rest, nspasteboard.org concealed/transient markers respected, exclusions, retention, pinned/favorite items.
- Text expander: bounded buffer, dynamic variables, per-app, import/export.
- Apple Vision OCR.
- Screenshots: area, window, full.
- Read aloud.
- Safe calculator and unit conversion.
- Developer tools.
- Quick launcher: fuzzy search and instant answers.

**Phase 6/7 (partial)**
- Window snapping: halves, quarters, maximize, center, next display.
- Global shortcuts with conflict detection.
- Emergency pause shortcut.
- Application profiles.

**UI**
- Dashboard (20 sections), onboarding wizard, overlay surfaces, light/dark/system theme, accent colours, text size, compact menu, reduced motion, increased contrast, searchable settings, Privacy Center.

## In progress / pending

| Feature | Status |
|---|---|
| Voice Anywhere (whisper.cpp, model manager, overlay) | Pending (Phase 4) |
| Keyboard Studio remapping, Mouse Studio, Trackpad Studio | Pending (Phase 6) |
| Automation / macro recording | Pending (Phase 7) |
| Saved window layouts | Pending |
| Screenshot annotation and redaction | Pending |
| `{input:Label}` prompt during auto-expansion | Expands to empty for now |
| Windows adapter (UI Automation, SendInput, low-level hooks, Windows OCR) | Pending. Windows currently reports no native capabilities. |
| Signed auto-update | Not configured |
| Code signing and notarization | Needs an Apple Developer ID (see PLATFORM_COMPATIBILITY.md) |

## Test results (this session)

- `cargo test --workspace`: **39 passed, 0 failed**.
  - Core: 30.
  - App: 9, including a real loopback OAuth callback test, PKCE against the RFC 7636 vector, encrypted clipboard storage and shortcut conflicts.
- `npm test` (vitest): **2 passed**.
- `tsc -b`: clean.
- `npm run build`: JS bundle 374 KB (113 KB gzip). The icon set was trimmed from 1,040 KB.
- `tauri build --debug --bundles app`: built `MinPoint.app` (debug, 64.5 MiB unoptimised). The helper is 400 KB universal.
- Smoke-tested the helper protocol directly: ping, permissions, frontmostApp, voices, 33 OCR languages, mouseLocation, error path.
- Launched the bundled app: the main process and the helper sidecar both started. Killing the app made the helper exit (crash-safety path verified).
- UI was rendered through the dev harness in headless Chrome: Overview (dark) and onboarding inspected.

## Not verified (blocked)

- End-to-end Shift + Right Click, text insertion, OCR capture, window snapping, clipboard watching and snippet expansion in real apps. All of these need TCC permissions that this managed Mac won't grant without admin rights.
- ChatGPT sign-in and inference. No sign-in was performed, and eligibility depends on the user's plan.
- Battery impact and startup time haven't been benchmarked yet.

## Measured (release build, Apple Silicon, 2026-11-25)

Measured with `ps`, 10 s after launch, with the onboarding window open:

| Process | Memory | CPU |
|---|---|---|
| `minpoint` | 97.5 MB RSS | 0.0% |
| `minpoint-helper` | 31.7 MB RSS | 0.0% |

WebKit renderer processes are separate and were not isolated in this measurement.

Sizes:
- DMG (arm64): 5.6 MB
- app binary: 7.8 MB
- helper: 209 KB (arm64)

## Known issues / limitations

- In `tauri dev`, macOS attributes permissions to the terminal that launched the app, not to MinPoint. Test permissions with the bundled `.app`.
- Apps that don't expose `AXSelectedText` (many Electron/Chromium apps) use the Cmd+C fallback. In those apps a changed selection can't be detected before replacing; the focus/pid check still applies.
- Overlays are opaque (no private transparency API), so the menu-opacity setting has no effect.
- Each settings change re-registers shortcuts and the event tap. That's fine for toggles; text fields such as profile names save on every keystroke and should be debounced.

## Next tasks

1. Manual verification on a Mac where permissions can be granted. Fill in `PLATFORM_COMPATIBILITY.md`.
2. Voice Anywhere with whisper.cpp (Metal), a model download manager with SHA-256 verification, and an AVAudioEngine capture overlay.
3. Windows adapter: UI Automation, SendInput, `WH_MOUSE_LL` for Shift + Right Click, Windows.Media.Ocr, Credential Manager (already wired via `keyring`).
4. Debounce settings writes from text inputs.
5. Release benchmarks and signing/notarization pipeline (GitHub Actions).
