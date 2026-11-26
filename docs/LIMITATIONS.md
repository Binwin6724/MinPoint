# Limitations

MinPoint is honest about what each platform allows and what's built so far.

## Windows
- The app, settings, snippets management, developer tools and calculator work.
- The following are **not implemented on Windows yet** and are hidden there:
  - Shift + Right Click menu
  - reading or replacing selected text
  - clipboard history
  - text expansion
  - OCR
  - screenshots
  - window snapping
  - read aloud

  They need a Windows adapter (low-level mouse hook, UI Automation, SendInput, Windows.Media.Ocr).
- The ChatGPT connection uses portable code but hasn't been tested on Windows.
- Elevated (administrator) apps and secure desktops can't be read or typed into from a normal app, by Windows design. MinPoint won't try to bypass that.

## macOS
- Shift + Right Click and text features need **Accessibility** and **Input Monitoring**; OCR and screenshots need **Screen Recording**. Managed Macs may require an administrator.
- Builds are ad-hoc signed, not notarized, so the first launch needs right-click ▸ Open. Because of that, macOS may ask for permissions again after an update.
- Some apps don't expose the selected text through Accessibility (many Electron and Chromium apps). MinPoint then uses a guarded copy/paste, which restores your clipboard. In those apps it can verify the app and window, but not that the selection is unchanged.
- **Secure Input** (password fields, some terminals) blocks reading, expansion and insertion. This is intentional.
- Overlays are opaque windows (no private transparency APIs).

## ChatGPT
- Requires an eligible ChatGPT plan (Plus or Pro) and internet access. Usage counts toward your plan's limits; Plus shares a five-hour limit across apps.
- Some OpenAI API options aren't available for plan usage. MinPoint only uses supported fields.

## Not built yet
- Voice typing
- Keyboard, Mouse and Trackpad Studio
- Automation Studio
- Screenshot annotation
- Saved window layouts
- `{input:…}` prompts during auto-expansion (they expand empty)
- Signed auto-updates

## Verification status
End-to-end behaviour in real apps hasn't been manually verified yet. The development machine couldn't grant the required macOS permissions. See [PLATFORM_COMPATIBILITY.md](PLATFORM_COMPATIBILITY.md).
