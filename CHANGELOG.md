# Changelog

## 0.1.2 — 2026-11-26

### Fixed
- **"MinPoint couldn't load … state not managed" on startup** (seen on Windows). On a slower PC (for example while security software scans a newly installed app), the window could ask for its data before MinPoint had finished starting. The window now waits and retries for up to 10 seconds instead of failing. The window still appears in about 0.2 s on Windows.
- **Upgrading while MinPoint is running.** The installer and uninstaller now force-close MinPoint, including its tray process and WebView2 helpers, before installing or removing files. Upgrades no longer stop with "Failed to kill MinPoint".
- **"Continue with ChatGPT" button text disappeared on hover in dark mode.**
- If the local database can't be opened, MinPoint now starts with a temporary database and logs the error, instead of quitting.

### Changed
- The setup file and uninstaller use the MinPoint icon.
- The Windows UI is clearer:
  - the status shows **In-app tools only** instead of a red error;
  - macOS-only features carry a **Mac** tag and an explanation;
  - Permissions explains that Windows needs none;
  - text says "system tray" instead of "menu bar";
  - the onboarding wizard skips the macOS permissions step.

## 0.1.1 — 2026-11-26

### Fixed
- **Blank window at startup** (reported on Windows; also reproduced on macOS). MinPoint read its key from the system credential store (Keychain / Windows Credential Manager) on the main thread during startup. A Keychain prompt, or a slow Credential Manager on a managed PC, froze startup before the window could talk to the app, leaving a blank window. Credential-store access now happens lazily on background threads, and only when clipboard history or a saved ChatGPT sign-in actually needs it.

### Added
- If the interface can't start, MinPoint now shows the error, with a **Try again** button, instead of a blank window.
- A troubleshooting log at `%LOCALAPPDATA%\app.minpoint.desktop\logs\minpoint.log` (Windows) or `~/Library/Logs/app.minpoint.desktop/minpoint.log` (macOS). It records startup steps and errors only, never your text.
- Right-click ▸ **Inspect** is available for troubleshooting.

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
