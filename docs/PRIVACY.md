# Privacy

**MinPoint has no servers.** There is no telemetry, analytics, crash reporting, advertising or cloud sync.

## Stored locally

| Data | Where | Notes |
|---|---|---|
| Settings, profiles, custom actions | `settings.json` (app config dir, 0600) | Exportable from Settings → Backup |
| Clipboard history | `minpoint.db` | **Off by default.** AES-256-GCM encrypted; the key is in the Keychain. Expires after the chosen retention period. |
| Snippets, counters | `minpoint.db` | |
| Action usage | `minpoint.db` | Action id, count and timestamp only. Never content. |
| ChatGPT tokens | Keychain | Removed on Disconnect or Delete everything |
| Temporary captures | `$TMPDIR/MinPoint` | OCR images are deleted right after recognition. Screenshots are deleted after copying/saving. |

## Never collected

- Keystrokes. The expander's buffer holds at most the length of your longest trigger, lives only in memory, and clears on every click, arrow key or app switch.
- Password fields or Secure Input content.
- Continuous screen captures or microphone audio. Voice is not implemented; when it is, audio will stay on-device and not be retained.
- Window contents for profile switching. Only the frontmost app's bundle identifier is used.

## Sent to OpenAI

This happens only when you choose a ChatGPT action:

- the selected text or your prompt, and
- the action's instructions.

It is sent to `api.openai.com/v1/responses` with `store:false`, under your ChatGPT plan. Clipboard history is never sent automatically.

## Your controls (Privacy Center)

- Clear clipboard history.
- Clear activity.
- Disconnect ChatGPT, which revokes the token.
- Pause MinPoint.
- Exclude apps.
- Export configuration.
- **Delete everything.**
