import { useState } from "react";
import { useStore } from "../store";
import type { Profile } from "../../lib/api";
import { Badge, Button, Empty, Icon, PageHeader, Row, Section, Toggle } from "../../components/ui";
import { AppListEditor } from "./Clipboard";

const TEMPLATES: { name: string; apps: string[]; disabledActions: string[]; onlyActions: string[] | null; disableOverlays: boolean }[] = [
  { name: "Slack", apps: ["com.tinyspeck.slackmacgap"], disabledActions: ["ai.email", "ai.explain_code", "ai.readable_code", "ai.document_code"], onlyActions: null, disableOverlays: false },
  { name: "VS Code", apps: ["com.microsoft.VSCode"], disabledActions: ["ai.casual", "ai.friendly", "ai.email", "ai.slack"], onlyActions: null, disableOverlays: false },
  { name: "Browser", apps: ["com.apple.Safari", "com.google.Chrome", "com.microsoft.edgemac", "org.mozilla.firefox"], disabledActions: [], onlyActions: null, disableOverlays: false },
  { name: "Gaming", apps: [], disabledActions: [], onlyActions: [], disableOverlays: true },
];

export function Profiles() {
  const { settings, update, status, actions } = useStore();
  const [open, setOpen] = useState<string | null>(null);

  const add = (t: (typeof TEMPLATES)[number] | null) =>
    update((s) => {
      const p: Profile = { id: crypto.randomUUID().slice(0, 8), name: t?.name ?? "New profile", apps: t?.apps ?? [], priority: 0, disabledActions: t?.disabledActions ?? [], onlyActions: t?.onlyActions ?? null, disableOverlays: t?.disableOverlays ?? false };
      s.profiles.push(p);
      setOpen(p.id);
    });
  const patch = (id: string, fn: (p: Profile) => void) => update((s) => { const p = s.profiles.find((x) => x.id === id); if (p) fn(p); });

  return (
    <>
      <PageHeader
        title="Application Profiles"
        subtitle="Change which actions appear depending on the app you're using. MinPoint switches automatically using only the name of the frontmost app — it never reads window contents."
        actions={<Button variant="primary" onClick={() => add(null)}><Icon name="plus" size={13} /> New profile</Button>}
      />
      {status?.activeApp && <p className="-mt-2 mb-4 text-[12.5px] text-muted">Active app: {status.activeApp} · Profile: {status.activeProfile ?? "Default"}</p>}

      <Section title="Profiles">
        {settings.profiles.length === 0 ? (
          <Empty icon="app-window" title="Using the default profile everywhere">
            Start from a template:
            <div className="mt-3 flex flex-wrap justify-center gap-2">
              {TEMPLATES.map((t) => <Button key={t.name} size="sm" onClick={() => add(t)}>{t.name}</Button>)}
            </div>
          </Empty>
        ) : (
          settings.profiles.map((p) => (
            <div key={p.id} className="border-b border-line last:border-b-0">
              <button onClick={() => setOpen(open === p.id ? null : p.id)} className="flex w-full items-center gap-3 px-4 py-3 text-left hover:bg-hover">
                <Icon name={open === p.id ? "chevron-down" : "chevron-right"} size={14} className="text-faint" />
                <span className="flex-1 text-[13px] font-medium">{p.name}</span>
                <span className="text-[12px] text-muted">{p.apps.length} app{p.apps.length === 1 ? "" : "s"}</span>
                {p.disableOverlays && <Badge tone="warn">MinPoint off</Badge>}
              </button>
              {open === p.id && (
                <div className="space-y-1 bg-raised px-4 pb-4">
                  <div className="grid grid-cols-[1fr_120px] gap-3 pt-2">
                    <label className="text-[12px] text-muted">Name<input className="field mt-1" value={p.name} onChange={(e) => patch(p.id, (x) => void (x.name = e.target.value))} /></label>
                    <label className="text-[12px] text-muted">Priority<input type="number" className="field mt-1" value={p.priority} onChange={(e) => patch(p.id, (x) => void (x.priority = Number(e.target.value) || 0))} /></label>
                  </div>
                  <div className="pt-2 text-[12px] font-medium text-muted">Apps (bundle identifiers)</div>
                  <div className="rounded-[8px] border border-line bg-panel">
                    <AppListEditor value={p.apps} onChange={(v) => patch(p.id, (x) => void (x.apps = v))} placeholder="com.tinyspeck.slackmacgap" />
                  </div>
                  <div className="rounded-[8px] border border-line bg-panel">
                    <Row title="Turn MinPoint off in these apps" description="No context menu or overlays (e.g. games, remote desktops).">
                      <Toggle label="Disable MinPoint" checked={p.disableOverlays} onChange={(v) => patch(p.id, (x) => void (x.disableOverlays = v))} />
                    </Row>
                  </div>
                  {!p.disableOverlays && (
                    <>
                      <div className="pt-2 text-[12px] font-medium text-muted">Hide these actions in this profile</div>
                      <div className="flex max-h-[200px] flex-wrap gap-1.5 overflow-y-auto rounded-[8px] border border-line bg-panel p-3">
                        {actions.filter((a) => a.contexts.length).map((a) => {
                          const hidden = p.disabledActions.includes(a.id);
                          return (
                            <button key={a.id} onClick={() => patch(p.id, (x) => void (x.disabledActions = hidden ? x.disabledActions.filter((i) => i !== a.id) : [...x.disabledActions, a.id]))}
                              className={`rounded-full border px-2 py-0.5 text-[11.5px] ${hidden ? "border-bad/30 bg-bad/10 text-bad line-through" : "border-line text-muted hover:bg-hover"}`}>{a.name}</button>
                          );
                        })}
                      </div>
                    </>
                  )}
                  <div className="flex justify-end pt-2">
                    <Button size="sm" variant="danger" onClick={() => update((s) => void (s.profiles = s.profiles.filter((x) => x.id !== p.id)))}>Delete profile</Button>
                  </div>
                </div>
              )}
            </div>
          ))
        )}
      </Section>

      <Section title="Exclude apps completely" description="MinPoint ignores these apps entirely, regardless of profile.">
        <AppListEditor value={settings.general.excludedApps} onChange={(v) => update((s) => void (s.general.excludedApps = v))} placeholder="com.example.game" />
      </Section>
    </>
  );
}
