import { useStore } from "../store";
import { Badge, PageHeader, Row, Section, ShortcutRecorder } from "../../components/ui";

const ACTIONS: [string, string, string][] = [
  ["window.left", "Left half", "panel-left"],
  ["window.right", "Right half", "panel-right"],
  ["window.top_left", "Top-left quarter", "arrow-up-left"],
  ["window.top_right", "Top-right quarter", "arrow-up-right"],
  ["window.bottom_left", "Bottom-left quarter", "arrow-down-left"],
  ["window.bottom_right", "Bottom-right quarter", "arrow-down-right"],
  ["window.maximize", "Maximize (fill screen)", "maximize"],
  ["window.center", "Center", "square"],
  ["window.next_display", "Move to next display", "monitor"],
];

export function WindowManager() {
  const { settings, update, status } = useStore();
  const ax = status?.permissions.accessibility === "granted";
  return (
    <>
      <PageHeader title="Window Manager" subtitle="Snap the frontmost window with a shortcut or from the Shift + Right Click menu (Window Management submenu). Respects the menu bar, Dock and each display's scaling; full-screen windows are left alone." />
      {!ax && (
        <div className="mb-5 rounded-[10px] border border-warn/30 bg-warn/5 p-3 text-[13px]">Window management needs the Accessibility permission. Grant it in Permissions.</div>
      )}
      <Section title="Shortcuts" description="Assign a key combination to each action. Shortcuts act on the window that's in front.">
        {ACTIONS.map(([id, label, icon]) => (
          <Row key={id} icon={icon} title={label}>
            <ShortcutRecorder value={settings.shortcuts[id] ?? ""} onChange={(v) => update((s) => { if (v) s.shortcuts[id] = v; else delete s.shortcuts[id]; })} />
          </Row>
        ))}
      </Section>
      <Section title="Layouts">
        <Row icon="layout-template" title="Saved layouts and app-specific layouts" description="Saving and restoring multi-window layouts isn't in this build yet."><Badge>Planned</Badge></Row>
      </Section>
    </>
  );
}
