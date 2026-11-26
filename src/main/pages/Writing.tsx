import { useState } from "react";
import { useStore } from "../store";
import { Badge, Button, Empty, Icon, PageHeader, Row, Section, Toggle } from "../../components/ui";
import type { CustomAiAction } from "../../lib/api";

const LANGS = ["English", "Spanish", "French", "German", "Italian", "Portuguese", "Hindi", "Malayalam", "Tamil", "Telugu", "Kannada", "Bengali", "Marathi", "Urdu", "Arabic", "Chinese (Simplified)", "Japanese", "Korean", "Russian", "Turkish", "Dutch", "Polish", "Indonesian", "Vietnamese", "Thai"];

const blank = (): CustomAiAction => ({ id: "", name: "", description: "", icon: "wand-sparkles", instructions: "", inMenu: true, apps: [] });

export function Writing() {
  const { settings, update, status, setPage, toast } = useStore();
  const [editing, setEditing] = useState<CustomAiAction | null>(null);
  const ai = status?.ai;
  const ready = ai?.state === "connected" && ai.planUsageAuthorized;

  const saveCustom = () => {
    if (!editing) return;
    if (!editing.name.trim() || !editing.instructions.trim()) {
      toast("Give the action a name and instructions.", "error");
      return;
    }
    const a = { ...editing, id: editing.id || crypto.randomUUID().slice(0, 8), name: editing.name.trim() };
    update((s) => {
      const i = s.ai.customActions.findIndex((x) => x.id === a.id);
      if (i >= 0) s.ai.customActions[i] = a;
      else s.ai.customActions.push(a);
    }).then(() => setEditing(null));
  };

  return (
    <>
      <PageHeader title="Writing Assistant" subtitle="Select text in any app, press Shift + Right Click, and choose Polish, Fix Grammar, Translate and more. Results always appear in a preview you control." />

      {!ready && (
        <div className="mb-5 flex items-center gap-3 rounded-[10px] border border-line bg-raised p-3 text-[13px]">
          <Icon name="cloud-off" className="text-muted" />
          <div className="flex-1">AI writing needs a connected ChatGPT account. Local text tools (case changes, cleanup, counting) work without it.</div>
          <Button size="sm" variant="primary" onClick={() => setPage("account")}>Connect</Button>
        </div>
      )}

      <Section title="Behavior">
        <Row icon="sparkles" title="Enable AI actions" description="Turn off to hide every ChatGPT-powered action from menus and the launcher.">
          <Toggle label="Enable AI actions" checked={settings.ai.enabled} onChange={(v) => update((s) => void (s.ai.enabled = v))} />
        </Row>
        <Row
          icon="eye"
          title="Preview before replacing"
          description="Recommended. Review, edit, copy or reject the result before it touches your text. When off, results replace the selection automatically (you can still undo)."
        >
          <Toggle label="Preview before replacing" checked={!settings.ai.autoReplace} onChange={(v) => update((s) => void (s.ai.autoReplace = !v))} />
        </Row>
        <Row icon="languages" title="Default translation language">
          <select className="field w-[200px]" value={settings.ai.defaultTargetLanguage} onChange={(e) => update((s) => void (s.ai.defaultTargetLanguage = e.target.value))} aria-label="Default translation language">
            {LANGS.map((l) => <option key={l}>{l}</option>)}
          </select>
        </Row>
        <Row icon="star" title="Favorite languages" description="Shown first when translating.">
          <div className="flex max-w-[380px] flex-wrap justify-end gap-1">
            {LANGS.slice(0, 14).map((l) => {
              const on = settings.ai.favoriteLanguages.includes(l);
              return (
                <button key={l} onClick={() => update((s) => void (s.ai.favoriteLanguages = on ? s.ai.favoriteLanguages.filter((x) => x !== l) : [...s.ai.favoriteLanguages, l]))}
                  className={`rounded-full border px-2 py-0.5 text-[11.5px] ${on ? "border-accent bg-accent/10 text-accent" : "border-line text-muted hover:bg-hover"}`}>{l}</button>
              );
            })}
          </div>
        </Row>
      </Section>

      <Section
        title="Custom actions"
        description="Your own ChatGPT instructions, available in the Shift + Right Click menu and the launcher."
        right={<Button size="sm" onClick={() => setEditing(blank())}><Icon name="plus" size={13} /> New action</Button>}
      >
        {editing && (
          <div className="space-y-3 border-b border-line bg-raised p-4">
            <div className="grid grid-cols-2 gap-3">
              <label className="text-[12px] text-muted">Name<input className="field mt-1" value={editing.name} onChange={(e) => setEditing({ ...editing, name: e.target.value })} placeholder="Refine Jira Ticket" /></label>
              <label className="text-[12px] text-muted">Description<input className="field mt-1" value={editing.description} onChange={(e) => setEditing({ ...editing, description: e.target.value })} placeholder="Turn notes into a ticket" /></label>
            </div>
            <label className="block text-[12px] text-muted">
              Instructions
              <textarea className="field mt-1" rows={4} value={editing.instructions} onChange={(e) => setEditing({ ...editing, instructions: e.target.value })}
                placeholder="Rewrite the selected content as a concise Jira ticket containing a title, description, acceptance criteria, and implementation considerations. Preserve the original requirements." />
            </label>
            <div className="flex items-center gap-3">
              <label className="flex items-center gap-2 text-[12.5px]"><Toggle label="Show in menu" checked={editing.inMenu} onChange={(v) => setEditing({ ...editing, inMenu: v })} /> Show in Shift + Right Click menu</label>
              <div className="flex-1" />
              <Button variant="ghost" onClick={() => setEditing(null)}>Cancel</Button>
              <Button variant="primary" onClick={saveCustom}>Save action</Button>
            </div>
          </div>
        )}
        {settings.ai.customActions.length === 0 && !editing ? (
          <Empty icon="wand-sparkles" title="No custom actions yet">For example “Refine Jira Ticket” or “Reply as support agent”.</Empty>
        ) : (
          settings.ai.customActions.map((c) => (
            <Row key={c.id} icon={c.icon} title={<span className="flex items-center gap-2">{c.name} {!c.inMenu && <Badge>Launcher only</Badge>}</span>} description={c.description || c.instructions.slice(0, 110)}>
              <Button size="sm" variant="ghost" onClick={() => setEditing(c)}>Edit</Button>
              <Button size="sm" variant="ghost" onClick={() => update((s) => void (s.ai.customActions = s.ai.customActions.filter((x) => x.id !== c.id)))} aria-label={`Delete ${c.name}`}><Icon name="trash-2" size={13} /></Button>
            </Row>
          ))
        )}
      </Section>

      <Section title="Safety">
        <Row icon="shield-check" title="Selected text is treated as data" description="Instructions hidden inside your selection are not followed. AI actions can't run commands, open files, send messages or trigger automations." />
        <Row icon="send-horizontal" title="Nothing is ever sent for you" description="InputFlow only inserts text into the field you were editing. It never presses Send or submits forms." />
        <Row icon="crosshair" title="Focus-safe replacement" description="If you switch apps or change the selection while a result is being generated, InputFlow won't insert into the wrong place — it keeps the result for you to copy." />
      </Section>
    </>
  );
}
