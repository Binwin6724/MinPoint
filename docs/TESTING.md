# Testing

## Automated

Run each of these:

```bash
cargo test --workspace
```

```bash
npm test
```

```bash
npx tsc -b
```

```bash
npm run build
```

| Suite | Covers |
|---|---|
| `minpoint-core::calc` | Precedence, right-associative powers, implicit multiplication, percent / "X% of Y", factorial, functions, thousands separators, division by zero, unknown identifiers (no code execution), depth/length limits |
| `::units` | Length, mass, data (SI and IEC), time, temperature; mismatched dimensions rejected |
| `::text` | Title/sentence case, whitespace, dedupe, sort, join, naming conventions (acronyms), find/replace, stats |
| `::devtools` | JSON format/minify/validate, Base64, URL, SHA-256/MD5 test vectors, timestamps, colours, CSV→Markdown, XML, regex |
| `::snippets` | Bounded buffer, longest-first matching, app scoping, case sensitivity, all variables, invalid strftime safety |
| `::writing` | Untrusted-text delimiter escaping, empty/oversized input, translation target, output cleanup |
| `::menu` | Selection / field / anywhere menus, favorites, defaults, AI-disconnected hints, capability hiding, secure fields, paused, excluded apps |
| `::settings` | Privacy-preserving defaults, round-trip, partial files, migration, clamping, profile priority |
| `::actions` | Unique namespaced ids, AI flag consistency |
| `minpoint::ai::auth` | RFC 7636 PKCE vector, authorize URL for first vs returning sign-in, terminal refresh errors, **real loopback callback server** (404 for other paths, state validation) |
| `minpoint::ai` | Mapping of documented plan-usage error codes |
| `minpoint::db` | Clipboard dedupe/ordering/limits, **ciphertext at rest**, pin survives clear, snippets, usage |
| `minpoint::shortcuts` | Reserved and duplicate accelerator detection |
| `src/overlay/diff.test.ts` | Word diff correctness |

Test code lives under `#[cfg(test)]` or `*.test.ts`. The `dev/` UI harness (fake IPC) is not part of the production build.

## UI harness (visual checks without native permissions)

Start the dev server:

```bash
npx vite --port 1420
```

Then open `http://localhost:1420/dev/harness.html`. Query options:
- `?theme=dark` for dark mode
- `?onboarding=1` for the onboarding flow

## Manual checklist (macOS)

Use the bundled app (`npm run app:build` or `npx tauri build --debug --bundles app`). Permissions attach to the `.app`.

1. Onboarding. Grant Accessibility and Input Monitoring. Check Overview: "Shift + Right Click active".
2. TextEdit:
   - Select text, Shift + Right Click, UPPERCASE, Replace. The text is replaced.
   - Normal right-click still shows TextEdit's menu.
3. Safari text area: select text, Shift + Right Click, Polish (connected), Replace. Clipboard contents are unchanged afterwards.
4. Switch apps while a result streams, then press Replace. Expect a refusal and the result kept for copying.
5. Password field: Shift + Right Click shows no text actions.
6. Clipboard: enable history, copy from Safari, see the item. Copy from 1Password and check it's skipped.
7. Expander: add `;sig`, type it in Notes, see it expand. In a password field it must not expand.
8. OCR: run a capture on an image of text.
9. Window shortcuts across two displays, including a scaled display.
10. Emergency pause: the tap is released and normal right-click behaviour is restored.
11. Kill MinPoint in Activity Monitor: `minpoint-helper` exits and input is normal.
12. Sleep/wake: the tap re-enables (helper handles `tapDisabledByTimeout`).

Record results in PLATFORM_COMPATIBILITY.md.
