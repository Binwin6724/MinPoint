# Changelog

## 0.1.0 — 2026-11-25 (pre-release)

First development build, released as **MinPoint**. It was called InputFlow during early development.

### Installers

| File | For |
|---|---|
| `MinPoint-0.1.0-mac-arm64.dmg` | Macs with Apple silicon (ad-hoc signed) |
| `MinPoint-0.1.0-mac-x64.dmg` | Intel Macs (ad-hoc signed) |
| `MinPoint-0.1.0-win-x64-setup.exe` | Windows 10/11 x64 |

`SHA256SUMS.txt` is published alongside them.

### Design

Uses the Min-family design: dark glassy cards and an orange→pink gradient accent.

### macOS

Implemented:
- Shift + Right Click universal menu
- AI writing with Sign in with ChatGPT
- Write with ChatGPT
- Custom AI actions
- Smart clipboard
- Text expander
- Apple Vision OCR
- Screenshots
- Window snapping
- Quick launcher
- Developer tools
- Read aloud
- Application profiles
- Privacy Center
- Onboarding

Manual end-to-end verification is still pending.

### Windows

The app builds and runs. Local tools work: developer tools, calculator, settings and snippets management. The ChatGPT connection code is portable but untested. System-wide features (context menu, insertion, OCR, clipboard watching, window management) are **not yet implemented** on Windows and are hidden in the UI.

### Planned

- Voice Anywhere
- Keyboard, Mouse and Trackpad Studio
- Automation Studio
- Screenshot annotation
- Saved window layouts
