# Contributing to MinPoint

Thanks for helping!

## Setup
```bash
npm install
```

```bash
npm run app:dev
```

On macOS, `app:dev` builds the Swift helper first. Rust stable and the Xcode Command Line Tools are required.

## Before opening a PR
```bash
cargo test --workspace && npm test && npx tsc -b
```

- Put platform-independent logic in `crates/minpoint-core`, with unit tests.
- OS-specific code goes behind `PlatformAdapter`. Report a capability only when it really works; the UI hides everything else.
- Never log selected text, clipboard contents, AI output or credentials. Never send tokens to the webview.
- Unfinished modules use the *Planned* page. Don't add buttons that do nothing.
- Keep `docs/PLATFORM_COMPATIBILITY.md` honest. Mark something *Verified* only after you've tested it on real hardware.

## Reporting bugs
Include the MinPoint version, OS and version, the app you were working in, and steps to reproduce. Report security issues privately (see `docs/SECURITY.md`).
