# Platform Compatibility

Legend:
- **Verified**: exercised manually or by automated tests on real hardware.
- **Implemented**: code complete and compiles, but not yet manually verified.
- **Partial**: works with stated limits.
- **Unsupported**: not possible or not implemented on that platform.
- **Untested**: no evidence either way.

## Capability matrix

| Feature | macOS (Apple Silicon) | macOS (Intel) | Windows 10/11 |
|---|---|---|---|
| App shell, tray, settings, onboarding | Verified (app launched, UI rendered) | Untested (universal helper built) | Untested (not built) |
| Native helper protocol | Verified (stdio smoke test) | Untested | N/A |
| Helper exits when the app dies | Verified | Untested | N/A |
| Shift + Right Click (CGEventTap) | Implemented, blocked by TCC on test machine | Untested | Unsupported (adapter pending) |
| Read selection (AX, then Cmd+C fallback) | Implemented | Untested | Unsupported |
| Replace selection / insert | Implemented | Untested | Unsupported |
| ChatGPT sign-in and inference | Implemented (unit tests for PKCE/URL/callback) | Untested | Implemented (portable Rust), untested |
| Clipboard history | Implemented (DB layer verified by tests) | Untested | Unsupported (no watcher yet) |
| Text expander | Implemented (buffer verified by tests) | Untested | Unsupported |
| Screen OCR (Apple Vision) | Implemented (33 languages reported by Vision) | Untested | Unsupported |
| Screenshots | Implemented | Untested | Unsupported |
| Read aloud (AVSpeechSynthesizer) | Implemented (voices enumerated) | Untested | Unsupported |
| Window management (AX) | Implemented | Untested | Unsupported |
| Global shortcuts | Implemented (tauri-plugin-global-shortcut) | Untested | Implemented, untested |
| Calculator, units, text and dev tools | Verified (unit tests) | Verified (portable) | Verified (portable code; tests run on macOS) |
| Quick launcher | Implemented | Untested | Partial (no app list) |
| Voice typing, key/mouse remap, trackpad gestures, macros | Unsupported (planned) | Unsupported | Unsupported |

## Application compatibility matrix

No application has been verified yet: TCC permissions could not be granted on the development machine. Expected behaviour, to be confirmed:

| App | Selection read | Insert method | Status |
|---|---|---|---|
| TextEdit, Notes | AX | AX write | Untested |
| Safari, Chrome, Edge (web text fields) | Cmd+C fallback | Paste | Untested |
| Slack, Teams, VS Code (Electron) | Cmd+C fallback | Paste | Untested |
| Microsoft Word | AX (partial) | AX or paste | Untested |
| Gmail (browser) | Cmd+C fallback | Paste | Untested |
| Password fields / Secure Input | Never read | Refused | By design |

## Distribution requirements

- **macOS**:
  - A Developer ID Application certificate ($99/yr Apple Developer Program) to sign `MinPoint.app` and the `minpoint-helper` sidecar.
  - Notarization through `notarytool` (Apple ID plus app-specific password, or an API key).
  - Configure `bundle.macOS.signingIdentity` and the `APPLE_*` environment variables for `tauri build`.
  - Without signing, macOS TCC permission grants may reset on every rebuild.
- **Windows**: an Authenticode code-signing certificate (OV/EV). Set `bundle.windows.certificateThumbprint`. The NSIS `setup.exe` installer (per-user) is built by CI.
- **Current releases**: macOS DMGs are **ad-hoc signed** (`signingIdentity: "-"`), like the other Min apps, and not notarized. The Windows installer is unsigned. CI (`.github/workflows/ci.yml`) builds `mac-arm64`, `mac-x64` and `win-x64`, verifies the helper is inside the macOS app, and publishes `SHA256SUMS.txt`.
- **Updates**: the Tauri updater plugin with a signing key pair is **not configured**. Do not enable unsigned updates.
