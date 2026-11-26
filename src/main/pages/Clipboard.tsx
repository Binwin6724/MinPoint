import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "../store";
import { api, errorText, type ClipItem } from "../../lib/api";
import { Button, Empty, Icon, PageHeader, Row, Section, ShortcutRecorder, Toggle } from "../../components/ui";

export function timeAgo(ts: number) {
  const s = Math.max(0, Date.now() / 1000 - ts);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}

export function AppListEditor({ value, onChange, placeholder }: { value: string[]; onChange: (v: string[]) => void; placeholder: string }) {
  const [draft, setDraft] = useState("");
  return (
    <div className="p-3">
      <div className="mb-2 flex flex-wrap gap-1.5">
        {value.length === 0 && <span className="text-[12px] text-faint">None</span>}
        {value.map((a) => (
          <span key={a} className="inline-flex items-center gap-1 rounded-full border border-line bg-raised px-2 py-0.5 text-[11.5px]">
            {a}
            <button onClick={() => onChange(value.filter((x) => x !== a))} aria-label={`Remove ${a}`} className="text-faint hover:text-fg"><Icon name="x" size={11} /></button>
          </span>
        ))}
      </div>
      <form className="flex gap-2" onSubmit={(e) => { e.preventDefault(); const v = draft.trim(); if (v && !value.includes(v)) onChange([...value, v]); setDraft(""); }}>
        <input className="field" value={draft} onChange={(e) => setDraft(e.target.value)} placeholder={placeholder} />
        <Button type="submit" size="sm">Add</Button>
      </form>
    </div>
  );
}

export function Clipboard() {
  const { settings, update, toast } = useStore();
  const [items, setItems] = useState<ClipItem[]>([]);
  const [q, setQ] = useState("");
  const on = settings.clipboard.historyEnabled;

  const load = useCallback(() => {
    if (on) api.clipboardList(q).then(setItems).catch((e) => toast(errorText(e), "error"));
    else setItems([]);
  }, [on, q, toast]);
  useEffect(() => {
    load();
    const un = listen("clipboard:changed", load);
    return () => void un.then((f) => f());
  }, [load]);

  const act = async (p: Promise<unknown>, msg?: string) => {
    try {
      await p;
      if (msg) toast(msg, "success");
      load();
    } catch (e) {
      toast(errorText(e), "error");
    }
  };

  return (
    <>
      <PageHeader title="Smart Clipboard" subtitle="A private clipboard history stored only on this computer, encrypted at rest. Off until you turn it on." />
      <Section title="History">
        <Row icon="clipboard-list" title="Keep clipboard history" description="Text and file paths you copy are saved locally. Content marked as concealed by password managers is skipped automatically.">
          <Toggle label="Keep clipboard history" checked={on} onChange={(v) => update((s) => void (s.clipboard.historyEnabled = v))} />
        </Row>
        <Row title="Open clipboard picker" description="Pick an earlier item and paste it into the app you're using.">
          <ShortcutRecorder value={settings.shortcuts["clip.history"] ?? ""} onChange={(v) => update((s) => void (s.shortcuts["clip.history"] = v))} />
        </Row>
        <Row title="Keep items for">
          <select className="field w-[140px]" value={settings.clipboard.retentionDays} onChange={(e) => update((s) => void (s.clipboard.retentionDays = Number(e.target.value)))} aria-label="Retention">
            {[1, 3, 7, 14, 30, 90, 365].map((d) => <option key={d} value={d}>{d === 1 ? "1 day" : `${d} days`}</option>)}
          </select>
        </Row>
        <Row title="Maximum items" description="Pinned and favorite items are never removed automatically.">
          <select className="field w-[140px]" value={settings.clipboard.maxItems} onChange={(e) => update((s) => void (s.clipboard.maxItems = Number(e.target.value)))} aria-label="Maximum items">
            {[50, 100, 200, 500, 1000].map((d) => <option key={d} value={d}>{d}</option>)}
          </select>
        </Row>
      </Section>

      <Section title="Never record from these apps" description="Bundle identifiers (e.g. com.1password.1password). Common password managers are included by default. Password detection can't be guaranteed in every app.">
        <AppListEditor value={settings.clipboard.excludedApps} onChange={(v) => update((s) => void (s.clipboard.excludedApps = v))} placeholder="com.example.app" />
      </Section>

      {on && (
        <Section
          title={`Items (${items.length})`}
          right={
            <div className="flex gap-2">
              <Button size="sm" variant="ghost" onClick={() => act(api.clipboardClear(true), "Cleared unpinned items")}>Clear unpinned</Button>
              <Button size="sm" variant="danger" onClick={() => confirm("Delete all clipboard history, including pinned items? This can't be undone.") && act(api.clipboardClear(false), "Clipboard history deleted")}>Delete all</Button>
            </div>
          }
        >
          <div className="border-b border-line p-2">
            <input className="field" placeholder="Search history" value={q} onChange={(e) => setQ(e.target.value)} aria-label="Search clipboard history" />
          </div>
          {items.length === 0 ? (
            <Empty icon="clipboard" title="Nothing here yet">Copy something in any app and it'll show up here.</Empty>
          ) : (
            <div className="max-h-[420px] overflow-y-auto">
              {items.map((it) => (
                <div key={it.id} className="group flex items-start gap-3 border-b border-line px-3 py-2 last:border-b-0">
                  <Icon name={it.kind === "files" ? "file" : "type"} size={14} className="mt-0.5 text-faint" />
                  <div className="min-w-0 flex-1">
                    <div className="selectable line-clamp-2 whitespace-pre-wrap break-words text-[12.5px]">{it.text}</div>
                    <div className="mt-0.5 text-[11px] text-faint">{timeAgo(it.createdAt)}{it.app ? ` · ${it.app}` : ""}</div>
                  </div>
                  <div className="flex items-center gap-0.5">
                    <button className="rounded p-1 text-faint hover:text-fg" aria-label="Copy" onClick={() => act(api.clipboardCopy(it.id), "Copied")}><Icon name="copy" size={13} /></button>
                    <button className={`rounded p-1 ${it.pinned ? "text-accent" : "text-faint hover:text-fg"}`} aria-label={it.pinned ? "Unpin" : "Pin"} onClick={() => act(api.clipboardFlag(it.id, "pinned", !it.pinned))}><Icon name="pin" size={13} /></button>
                    <button className={`rounded p-1 ${it.favorite ? "text-warn" : "text-faint hover:text-fg"}`} aria-label="Favorite" onClick={() => act(api.clipboardFlag(it.id, "favorite", !it.favorite))}><Icon name="star" size={13} className={it.favorite ? "fill-current" : ""} /></button>
                    <button className="rounded p-1 text-faint hover:text-bad" aria-label="Delete" onClick={() => act(api.clipboardDelete(it.id))}><Icon name="trash" size={13} /></button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Section>
      )}
    </>
  );
}
