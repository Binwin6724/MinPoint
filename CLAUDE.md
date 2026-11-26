# CLAUDE.md — working on MinPoint

- Rust via Homebrew rustup: `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`.
- Build the Swift helper first: `npm run helper:mac`. It writes `src-tauri/binaries/minpoint-helper-<triple>`, which Tauri needs.
- Tests: `cargo test --workspace && npm test && npx tsc -b`. Keep them green.
- Platform-independent logic goes in `crates/minpoint-core` (no I/O, unit-tested). OS code goes behind `PlatformAdapter`.
- New native capability:
  1. Add a method to the Swift helper (`handle` switch).
  2. Add it to `PlatformAdapter` and implement it in `MacAdapter` and `UnsupportedAdapter`.
  3. Expose it through a narrow command in `commands.rs` and register it in `lib.rs`.
  4. Add it to `src/lib/api.ts`.
- New action: add it to `builtin_actions()` with honest capabilities. The menu hides actions whose capabilities are missing.
- Never fake integrations. Unimplemented modules use `pages/Planned.tsx`.
- Never expose tokens to the webview or logs. Never log selected text, clipboard contents or AI output.
- Icons: use lucide kebab-case names in quotes. `npm run build` regenerates `src/components/icons.gen.ts`.
- Update docs/DEVELOPMENT_PROGRESS.md and docs/PLATFORM_COMPATIBILITY.md with real evidence only.
- Match the Min-family style (MinTransfer, MinSoundFX, MinDesk):
  - dark glassy cards and a two-tone gradient accent (MinPoint: #ff7a45 → #ff4f9a);
  - the README layout;
  - installer names `MinPoint-<v>-mac-arm64.dmg`, `-mac-x64.dmg`, `-win-x64-setup.exe`, plus `SHA256SUMS.txt`.
- Release: bump the version in package.json, Cargo.toml and src-tauri/tauri.conf.json, update CHANGELOG.md, then push a `vX.Y.Z` tag. `.github/workflows/ci.yml` builds all three installers and publishes the release.
- README screenshots: run `npx vite --port 1420`, then open `dev/harness.html?demo=1&theme=dark&page=<id>` (or `&overlay=menu#/overlay`).
