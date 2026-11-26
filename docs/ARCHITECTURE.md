# Architecture

## Overview

```
┌────────────────────────── MinPoint.app ──────────────────────────┐
│  Webview (React/TS, untrusted)                                      │
│   ├─ main window: dashboard, settings, onboarding                   │
│   └─ overlay window: menu · preview · write · palette · clipboard   │
│        │  typed, narrow Tauri commands only (src-tauri/commands.rs) │
│  Rust runtime (src-tauri)                                           │
│   ├─ Runtime: invocation context, action execution, overlay control│
│   ├─ minpoint-core: action registry, menu resolver, settings,      │
│   │   calculator, units, text/dev tools, snippets, prompt builder   │
│   ├─ AiService: Sign in with ChatGPT, token lifecycle, /v1/responses│
│   ├─ Db: SQLite (clipboard encrypted with AES-GCM, snippets, usage) │
│   ├─ secrets: Keychain / Credential Manager                         │
│   ├─ shortcuts (global-shortcut plugin), tray                       │
│   └─ PlatformAdapter ──► MacAdapter ──stdio NDJSON──► Swift helper  │
│                       └► UnsupportedAdapter (Windows for now)       │
└─────────────────────────────────────────────────────────────────────┘
```

## Key decisions

| Decision | Why |
|---|---|
| **Tauri 2 + Rust + React** | Small footprint (the debug bundle is mostly the WebKit view; release is much smaller). No Electron. Rust keeps native work out of the webview. |
| **Swift helper sidecar for macOS** | Accessibility (AXUIElement), CGEventTap, Vision, AVSpeechSynthesizer and NSPasteboard are simplest and safest from Swift. The helper exposes a **fixed** method list over stdin/stdout. It has no shell, no networking, and no file access beyond its own temp captures. It exits when stdin closes, so if the app quits or crashes, the event tap goes away and normal input is restored. |
| **`PlatformAdapter` trait** | Each OS gets its own adapter. Capabilities are reported at runtime, and the menu engine **hides** actions whose capabilities are missing, so unsupported features are never faked. Windows currently uses `UnsupportedAdapter`. |
| **Central action registry** (`crates/minpoint-core/src/actions.rs`) | Every action declares id, name, icon, group, contexts, capabilities, permissions, offline/AI flags, output behavior, security class and kind. The menu, shortcuts, launcher and tray all invoke actions through `Runtime::run_action`. |
| **Direct inference, not Codex app-server** | Writing actions are single, tool-less text transformations. The Codex app-server is an agent runtime with local shell/MCP tools, the opposite of the isolation required for untrusted selected text. MinPoint uses the documented `POST /v1/responses` route (`store:false`, `stream:true`) with plan-usage tokens. |
| **Opaque overlay window** | Tauri needs `macOSPrivateApi` for transparent windows. We avoid private APIs, so overlays are opaque, undecorated, always-on-top windows. |
| **One overlay window, many modes** | Lower memory than one webview per surface. Rust sends `overlay:show {mode, payload}`. |

## Context-menu flow

1. The helper's CGEventTap sees `rightMouseDown` with only Shift held. It **consumes** that click and its matching mouse-up, so the app's own menu never opens, and emits `contextMenu {x,y}`. All other right-clicks pass through untouched.
2. `Runtime::open_context_menu` calls `focusInfo` (AX role, editable, secure field, frontmost app). If the AX selection is unavailable and the user acted explicitly, it falls back to a guarded Cmd+C that saves and restores every pasteboard item. Secure input is never read.
3. `menu::resolve` builds entries from the registry, user config (disabled, order, favorites, submenus), the app's profile, exclusions and runtime capabilities.
4. The overlay is positioned at the cursor and clamped to the display that contains it.
5. Running an action:
   - Local text/dev ops compute and show a preview.
   - AI ops build a bounded prompt (`writing.rs`) and stream into the preview.
   - Builtins (copy, paste plain, insert date, OCR, windows…) run directly.
6. **Accept**: the runtime reactivates the original app (by pid) and verifies it is frontmost. Where possible it re-reads the AX selection; if anything changed it refuses and keeps the result for copying. It then inserts through an AX `kAXSelectedText` write. If that fails, it pastes a transient (`org.nspasteboard.TransientType`) pasteboard item and restores the user's clipboard.

## ChatGPT integration

Source: `src-tauri/src/ai/`.

- OAuth 2.0 Authorization Code + PKCE (S256) with a loopback redirect `http://127.0.0.1:<port>/auth/callback`, preferring 1455.
- First registration uses `client_id=dynamic_agent_client` plus `agent_name_hint=MinPoint`. The issued `client_id` returned in the callback is persisted and reused.
- `ext_agent_host_id` is a random `urn:uuid:` stored per install.
- Scopes: `openid profile email offline_access resource.invoke chatgpt.tokens.use.direct`. Resource: `https://api.openai.com/v1`.
- The ID token is verified against the JWKS from OpenID discovery (issuer, audience, expiry, nonce).
- Plan usage is enabled only if `chatgpt.tokens.use.direct` was granted.
- Tokens are stored in the Keychain. Refresh happens about 2 minutes before expiry, serialised through a mutex. Terminal refresh errors clear tokens. Disconnect revokes via `revocation_endpoint`.
- Documented error codes such as `subscription_sharing_usage_limit_exceeded` map to user-facing states, including a **Manage usage** link to `chatgpt.com/settings/usage`.
- Branding follows OpenAI's UI guidelines: **Continue with ChatGPT**, "Use your ChatGPT plan", "Using ChatGPT plan", and a first-use "You're using your ChatGPT plan" notice. The requested "Connect Codex" label was **not** used because it doesn't match the official sign-in wording.

## Data & configuration

- `settings.json` lives in the app config dir. It is versioned (`version`), every field has a serde default, `migrate()` upgrades old schemas, and `sanitize()` clamps values. Invalid files are backed up, never silently overwritten. Writes are atomic with mode 0600.
- `minpoint.db` (SQLite, WAL, `secure_delete`) has these tables:
  - `clipboard_items`: AES-256-GCM-encrypted content, keyed SHA-256 for duplicate detection.
  - `snippets`: JSON rows.
  - `counters`
  - `action_usage`: id, count, last_used only. No content.
- `chatgpt-registration.json` holds non-secret data: host id and issued client id.
- Keychain items:
  - `chatgpt-tokens`
  - `database-key`

## Text expansion privacy

The helper emits printable characters only when the expander is enabled and Secure Input is off. Command and Control chords, navigation keys and clicks emit `inputReset`. `TriggerBuffer` keeps at most N characters, where N is the length of the longest enabled trigger. The buffer clears on a match or reset and is never persisted.

## Repository structure

See README. Tests live next to code (`#[cfg(test)]`) and in `src/**/*.test.ts`.
