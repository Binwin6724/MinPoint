# Security

MinPoint can see the screen, read selections and synthesise input. Every one of those capabilities is constrained below.

## Trust boundaries

1. **Webview (untrusted UI)**
   - Has no generic native bridge. Only the narrow commands in `src-tauri/src/commands.rs` exist.
   - Capability file `src-tauri/capabilities/default.json` grants only core window/event permissions.
   - Strict CSP and `freezePrototype` are enabled.
2. **Rust runtime (trusted)**
   - Validates inputs: permission kinds, screenshot modes, external URL allow-list, `run_on_text` action prefixes, snippet fields, and that launcher app paths exist in the scanned app list.
3. **Swift helper (trusted, minimal)**
   - Fixed method list. No shell, no network.
   - Writes files only to its own temp capture files (0700 directory).
   - `launchApp` accepts only `.app` bundles.
   - Exits when its parent goes away.
4. **ChatGPT (external)**
   - Receives only text the user explicitly acted on.

## Secrets

- OAuth tokens and the database key live in the macOS Keychain / Windows Credential Manager (`keyring`).
- Tokens are never sent to the webview. `ai_status` returns account name, email and flags only.
- Tokens are never logged and never placed in URLs. `id_token_hint` goes only to the authorize endpoint, as specified.
- No client secret or API key exists. Registration is per install via dynamic client registration.

## AI isolation

- Selected text is wrapped in `<user_text>` delimiters. Embedded delimiters are escaped. The instructions declare the content untrusted (see `writing.rs` and its tests).
- Requests are tool-less, with `store:false`. Output is only ever shown as text in a preview. It is never executed, never triggers macros, and never sends messages or submits forms.
- AI output is inserted only after the user clicks Replace, unless auto-replace is enabled, and then only into the verified original app.

## Input safety

- The event tap consumes only Shift + Right Click (and its matching mouse-up). Everything else passes through.
- Synthetic events carry a marker (`eventSourceUserData = 0x1F10`) and are ignored by the tap, which prevents loops.
- Secure Input (`IsSecureEventInputEnabled`) and `AXSecureTextField` block reading, expansion and insertion.
- Emergency pause shortcut (default Ctrl+Alt+Shift+Esc). It stays registered while paused.
- Crash safety: the helper exits on stdin EOF, which releases the tap.

## Imports

- Snippet and config imports are data only.
- Imports cannot enable clipboard history, text expansion or auto-replace.
- Values are sanitised. Unknown fields are ignored.

## Calculator

- A hand-written recursive-descent parser with an allow-list of functions, a 512-character input limit and a 64-level depth limit. No `eval`.

## Reporting

Open a private security advisory on the project repository. Please don't file public issues for vulnerabilities.
