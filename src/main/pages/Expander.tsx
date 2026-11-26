import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "../store";
import { api, errorText, type Snippet } from "../../lib/api";
import { Badge, Button, Empty, Icon, PageHeader, Row, Section, Toggle } from "../../components/ui";
import { AppListEditor } from "./Clipboard";

const blank = (content = ""): Snippet => ({ id: "", name: "", trigger: "", content, category: "", caseSensitive: false, favorite: false, enabled: true, apps: [] });

const VARS = [
  ["{date}", "Today, e.g. 2026-11-25"],
  ["{date:%d %B %Y}", "Custom date format"],
  ["{time}", "Current time"],
  ["{clipboard}", "Current clipboard text"],
  ["{username}", "Your account name"],
  ["{counter:invoice}", "Increasing number"],
  ["{cursor}", "Where the cursor should end up"],
];

function download(name: string, text: string) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  a.download = name;
  a.click();
  URL.revokeObjectURL(a.href);
}

export function Expander() {
  const { settings, update, toast } = useStore();
  const [list, setList] = useState<Snippet[]>([]);
  const [edit, setEdit] = useState<Snippet | null>(null);
  const [q, setQ] = useState("");
  const [cat, setCat] = useState("");

  const load = () => api.snippets().then(setList).catch((e) => toast(errorText(e), "error"));
  useEffect(() => {
    load();
    const un = listen<{ content: string }>("snippet:new", (e) => setEdit(blank(e.payload.content)));
    return () => void un.then((f) => f());
  }, []);

  const categories = useMemo(() => [...new Set(list.map((s) => s.category).filter(Boolean))].sort(), [list]);
  const shown = list.filter((s) => (!cat || s.category === cat) && `${s.name} ${s.trigger} ${s.content}`.toLowerCase().includes(q.toLowerCase()));

  const save = async () => {
    if (!edit) return;
    try {
      await api.snippetSave(edit);
      setEdit(null);
      load();
      toast("Snippet saved", "success");
    } catch (e) {
      toast(errorText(e), "error");
    }
  };

  const importFile = async (f: File) => {
    try {
      const n = await api.snippetsImport(await f.text());
      toast(`Imported ${n} snippets`, "success");
      load();
    } catch (e) {
      toast(errorText(e), "error");
    }
  };

  return (
    <>
      <PageHeader
        title="Text Expander"
        subtitle="Type a short abbreviation like ;prdone and InputFlow replaces it with your saved text. Snippets can also be inserted from the menu or launcher."
        actions={
          <>
            <label className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-[7px] border border-line bg-panel px-3 text-[13px] font-medium hover:bg-hover">
              <Icon name="upload" size={13} /> Import
              <input type="file" accept="application/json,.json" className="hidden" onChange={(e) => e.target.files?.[0] && importFile(e.target.files[0])} />
            </label>
            <Button onClick={async () => download("inputflow-snippets.json", await api.snippetsExport())}><Icon name="download" size={13} /> Export</Button>
            <Button variant="primary" onClick={() => setEdit(blank())}><Icon name="plus" size={13} /> New snippet</Button>
          </>
        }
      />

      <Section title="Expansion">
        <Row
          icon="keyboard"
          title="Expand abbreviations as I type"
          description="Uses a tiny in-memory buffer only as long as your longest trigger. It's cleared on every click, arrow key or app switch, never saved, and disabled in password fields."
        >
          <Toggle label="Expand abbreviations" checked={settings.expander.enabled} onChange={(v) => update((s) => void (s.expander.enabled = v))} />
        </Row>
      </Section>

      {edit && (
        <Section title={edit.id ? "Edit snippet" : "New snippet"}>
          <div className="space-y-3 p-4">
            <div className="grid grid-cols-3 gap-3">
              <label className="text-[12px] text-muted">Name<input className="field mt-1" value={edit.name} onChange={(e) => setEdit({ ...edit, name: e.target.value })} placeholder="PR done" autoFocus /></label>
              <label className="text-[12px] text-muted">Abbreviation<input className="field mono mt-1" value={edit.trigger} onChange={(e) => setEdit({ ...edit, trigger: e.target.value })} placeholder=";prdone" /></label>
              <label className="text-[12px] text-muted">Category<input className="field mt-1" value={edit.category} onChange={(e) => setEdit({ ...edit, category: e.target.value })} placeholder="Work" list="snippet-cats" /></label>
              <datalist id="snippet-cats">{categories.map((c) => <option key={c} value={c} />)}</datalist>
            </div>
            <label className="block text-[12px] text-muted">
              Text
              <textarea className="field mt-1" rows={5} value={edit.content} onChange={(e) => setEdit({ ...edit, content: e.target.value })} placeholder="I've completed the changes and raised the pull request. Please review it when you have a chance." />
            </label>
            <div className="flex flex-wrap gap-1.5">
              {VARS.map(([v, d]) => (
                <button key={v} title={d} onClick={() => setEdit({ ...edit, content: edit.content + v })} className="mono rounded border border-line bg-raised px-1.5 py-0.5 text-[11px] hover:bg-hover">{v}</button>
              ))}
            </div>
            <div className="flex items-center gap-4 text-[12.5px]">
              <label className="flex items-center gap-2"><Toggle label="Case sensitive" checked={edit.caseSensitive} onChange={(v) => setEdit({ ...edit, caseSensitive: v })} /> Case-sensitive trigger</label>
              <label className="flex items-center gap-2"><Toggle label="Enabled" checked={edit.enabled} onChange={(v) => setEdit({ ...edit, enabled: v })} /> Enabled</label>
              <div className="flex-1" />
              <Button variant="ghost" onClick={() => setEdit(null)}>Cancel</Button>
              <Button variant="primary" onClick={save}>Save</Button>
            </div>
            <details className="text-[12px] text-muted">
              <summary className="cursor-pointer">Only in specific apps</summary>
              <AppListEditor value={edit.apps} onChange={(apps) => setEdit({ ...edit, apps })} placeholder="com.tinyspeck.slackmacgap" />
            </details>
          </div>
        </Section>
      )}

      <Section
        title={`Snippets (${list.length})`}
        right={
          <div className="flex gap-2">
            {categories.length > 0 && (
              <select className="field h-7 w-[140px] py-0 text-[12px]" value={cat} onChange={(e) => setCat(e.target.value)} aria-label="Category">
                <option value="">All categories</option>
                {categories.map((c) => <option key={c}>{c}</option>)}
              </select>
            )}
            <input className="field h-7 w-[180px] py-0 text-[12px]" placeholder="Search" value={q} onChange={(e) => setQ(e.target.value)} aria-label="Search snippets" />
          </div>
        }
      >
        {shown.length === 0 ? (
          <Empty icon="text-cursor-input" title={list.length ? "No matches" : "No snippets yet"}>Create one for text you type often — greetings, addresses, sign-offs, code.</Empty>
        ) : (
          shown.map((s) => (
            <div key={s.id} className="flex items-start gap-3 border-b border-line px-4 py-2.5 last:border-b-0">
              <button className={`mt-0.5 ${s.favorite ? "text-warn" : "text-faint hover:text-fg"}`} aria-label="Favorite" onClick={() => api.snippetSave({ ...s, favorite: !s.favorite }).then(load)}>
                <Icon name="star" size={13} className={s.favorite ? "fill-current" : ""} />
              </button>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2 text-[13px] font-medium">
                  {s.name}
                  {s.trigger && <code className="mono rounded bg-raised px-1.5 text-[11px] text-muted">{s.trigger}</code>}
                  {s.category && <Badge>{s.category}</Badge>}
                  {!s.enabled && <Badge tone="warn">Disabled</Badge>}
                  {s.apps.length > 0 && <Badge>{s.apps.length} app{s.apps.length > 1 ? "s" : ""}</Badge>}
                </div>
                <div className="mt-0.5 line-clamp-2 whitespace-pre-wrap text-[12px] text-muted">{s.content}</div>
              </div>
              <Button size="sm" variant="ghost" onClick={() => setEdit(s)}>Edit</Button>
              <Button size="sm" variant="ghost" aria-label={`Delete ${s.name}`} onClick={() => confirm(`Delete “${s.name}”?`) && api.snippetDelete(s.id).then(load)}><Icon name="trash-2" size={13} /></Button>
            </div>
          ))
        )}
      </Section>

      <Section title="Don't expand in these apps">
        <AppListEditor value={settings.expander.excludedApps} onChange={(v) => update((s) => void (s.expander.excludedApps = v))} placeholder="com.microsoft.VSCode" />
      </Section>
    </>
  );
}
