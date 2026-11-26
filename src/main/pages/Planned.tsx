import { Badge, Icon, PageHeader, Row, Section } from "../../components/ui";

// Honest status pages for modules that are designed but not implemented yet.
// No controls are shown that would pretend to work.
const INFO: Record<string, { title: string; subtitle: string; icon: string; plan: [string, string][]; notes: string }> = {
  keyboard: {
    title: "Keyboard Studio",
    subtitle: "Key remapping, modifier remapping, Caps Lock behaviour, per-app shortcuts and long-press / double-press actions.",
    icon: "keyboard",
    plan: [
      ["Remapping engine", "CGEventTap on macOS / low-level keyboard hook on Windows, with loop protection (events MinPoint synthesises are tagged and ignored)."],
      ["Visual keyboard", "Pick a key on an on-screen layout and choose what it does."],
      ["Conflict detection", "Shortcut conflict checks already run for global shortcuts in this build."],
      ["Crash safety", "Remaps live in the native helper; if MinPoint quits or crashes, the helper exits and normal input returns immediately."],
    ],
    notes: "Available today: global shortcuts for MinPoint actions (Settings → Global shortcuts, Window Manager, Quick Launcher) and the emergency pause shortcut.",
  },
  mouse: {
    title: "Mouse Studio",
    subtitle: "Button remapping, side-button actions, scroll speed and direction, gestures and per-app mappings.",
    icon: "mouse",
    plan: [
      ["Button remapping", "Middle/side buttons → MinPoint actions, shortcuts, media keys, window actions."],
      ["Scrolling", "Speed, reverse direction per device, horizontal scrolling with a modifier."],
      ["Hardware honesty", "DPI and polling rate are shown only where a supported interface exists."],
    ],
    notes: "Available today: Shift + Right Click opens the MinPoint menu without affecting your normal right-click.",
  },
  trackpad: {
    title: "Trackpad Studio",
    subtitle: "Additional multi-finger gestures where the operating system allows it.",
    icon: "hand",
    plan: [
      ["Research first", "macOS exposes no public API for custom multi-finger gestures system-wide; Windows Precision Touchpad gestures are configurable only through OS settings."],
      ["No private APIs", "MinPoint won't use private frameworks or drivers to claim gestures it can't support reliably. Unsupported gestures will be clearly marked."],
    ],
    notes: "This module may ship with a limited, capability-aware gesture set.",
  },
  voice: {
    title: "Voice Anywhere",
    subtitle: "Speak instead of typing in any text field, transcribed offline on this computer.",
    icon: "mic",
    plan: [
      ["Offline transcription", "whisper.cpp with Metal acceleration on Apple Silicon; models downloaded on demand from the official repository with checksums."],
      ["Privacy", "Explicit activation only, visible recording indicator, no audio retained, password fields excluded, never sent to the cloud."],
      ["Optional AI cleanup", "Only the transcribed text — never audio — can be sent to ChatGPT, and only if you enable it."],
    ],
    notes: "The microphone permission is not requested until this module ships.",
  },
  automation: {
    title: "Automation Studio",
    subtitle: "Build and record macros: open apps, press shortcuts, insert text, wait, click, scroll and run MinPoint actions.",
    icon: "workflow",
    plan: [
      ["Safety limits", "Step and time limits, an always-on emergency stop, recursion guards, and no shell commands."],
      ["Imports are data", "Imported macros never run automatically and can't execute privileged operations."],
      ["AI isolation", "AI-generated text can never trigger automation."],
    ],
    notes: "Available today: Quick Launcher, window shortcuts and custom ChatGPT actions.",
  },
};

export function Planned({ id }: { id: string }) {
  const i = INFO[id];
  return (
    <>
      <PageHeader title={i.title} subtitle={i.subtitle} actions={<Badge tone="accent">Planned — not in this build</Badge>} />
      <div className="mb-6 flex items-start gap-3 rounded-[10px] border border-line bg-raised p-4 text-[13px]">
        <Icon name="info" className="mt-0.5 text-accent" />
        <div>
          <div className="font-medium">This module isn't implemented yet.</div>
          <div className="mt-1 text-muted">{i.notes}</div>
        </div>
      </div>
      <Section title="Design">
        {i.plan.map(([t, d]) => <Row key={t} icon={i.icon} title={t} description={d} />)}
      </Section>
    </>
  );
}
