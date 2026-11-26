# CLAUDE.md — working on InputFlow

- Rust via Homebrew rustup: `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`.
- Build the Swift helper first: `npm run helper:mac`. It writes `src-tauri/binaries/inputflow-helper-<triple>`, which Tauri needs.
- Tests: `cargo test --workspace && npm test && npx tsc -b`. Keep them green.
- Platform-independent logic goes in `crates/inputflow-core` (no I/O, unit-tested). OS code goes behind `PlatformAdapter`.
- New native capability:
  1. Add a method to the Swift helper (`handle` switch).
  2. Add it to `PlatformAdapter` and implement it in `MacAdapter` and `UnsupportedAdapter`.
  3. Expose it through a narrow command in `commands.rs` and register it in `lib.rs`.
  4. Add it to `src/lib/api.ts`.
- New action: add it to `builtin_actions()` with honest capabilities. The menu hides actions whose capabilities are missing.
- Never fake integrations. Unimplemented modules use `pages/Planned.tsx`.
- Never expose tokens to the webview or logs. Never log selected text, clipboard contents or AI output.
- Icons: use lucide kebab-case names in quotes. `npm run build` regenerates `src/components/icons.gen.ts`.
- Update DEVELOPMENT_PROGRESS.md and PLATFORM_COMPATIBILITY.md with real evidence only.
