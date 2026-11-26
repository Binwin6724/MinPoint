// The overlay window hosts MinPoint's floating surfaces: context menu, result
// preview, Write with ChatGPT, command palette, clipboard picker, snippet picker
// and OCR result. It's shown by the Rust runtime with a mode + payload.
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorText, type ClipItem, type LauncherResponse, type MenuEntry, type Settings, type Snippet, type WritingOp } from "../lib/api";
import { Icon, useTheme } from "../components/ui";
import { wordDiff } from "./diff";

type Show = { mode: string; payload: any; seq?: number };

function Shell({ children, title, onClose, icon }: { children: ReactNode; title?: ReactNode; onClose: () => void; icon?: string }) {
  return (
    <div className="flex h-screen flex-col overflow-hidden border border-line bg-[var(--if-panel-solid)] text-fg">
      {title && (
        <div className="drag-region flex h-9 shrink-0 items-center gap-2 border-b border-line px-3">
          {icon && <Icon name={icon} size={14} className="text-accent" />}
          <div className="flex-1 truncate text-[12.5px] font-semibold">{title}</div>
          <button onClick={onClose} aria-label="Close" className="rounded p-1 text-faint hover:bg-hover hover:text-fg" style={{ WebkitAppRegion: "no-drag" } as never}><Icon name="x" size={14} /></button>
        </div>
      )}
      {children}
    </div>
  );
}

// ---------------- Context menu ----------------

function MenuView({ payload, settings, close }: { payload: any; settings: Settings | null; close: () => void }) {
  const [stack, setStack] = useState<{ name: string; items: MenuEntry[] }[]>([{ name: "", items: payload.entries }]);
  const [idx, setIdx] = useState(0);
  const [msg, setMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const level = stack[stack.length - 1];
  const selectable = level.items.map((e, i) => (e.kind === "separator" || (e.kind === "item" && !e.available) ? -1 : i)).filter((i) => i >= 0);
  const compact = settings?.appearance.compact;

  useEffect(() => setIdx(selectable[0] ?? 0), [stack.length]);

  const languages = settings?.ai.favoriteLanguages.length ? settings.ai.favoriteLanguages : ["English", "Spanish", "French", "Hindi"];
  const choose = async (e: MenuEntry) => {
    if (e.kind === "submenu") return setStack([...stack, { name: e.name, items: e.items }]);
    if (e.kind !== "item") return;
    if (!e.available) return setMsg(e.reason);
    if (e.id === "ai.translate" && !e.id.includes(":")) {
      return setStack([...stack, { name: "Translate to", items: languages.map((l) => ({ kind: "item", id: `ai.translate:${l}`, name: l, icon: "languages", requiresAi: true, available: true, reason: null })) }]);
    }
    setBusy(true);
    try {
      if (e.id.startsWith("ai.translate:")) await api.overlayRun("ai.translate", { language: e.id.split(":")[1] });
      else await api.overlayRun(e.id);
    } catch (err) {
      setMsg(errorText(err));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    const onKey = (ev: KeyboardEvent) => {
      const pos = selectable.indexOf(idx);
      if (ev.key === "ArrowDown") { ev.preventDefault(); setIdx(selectable[(pos + 1) % selectable.length]); }
      else if (ev.key === "ArrowUp") { ev.preventDefault(); setIdx(selectable[(pos - 1 + selectable.length) % selectable.length]); }
      else if (ev.key === "Enter" || ev.key === "ArrowRight") { const e = level.items[idx]; if (e && (ev.key === "Enter" || e.kind === "submenu")) choose(e); }
      else if (ev.key === "ArrowLeft" || ev.key === "Backspace") { if (stack.length > 1) setStack(stack.slice(0, -1)); }
      else if (ev.key === "Escape") { if (stack.length > 1) setStack(stack.slice(0, -1)); else close(); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const inv = payload.invocation;
  return (
    <div className="anim-pop flex h-screen flex-col overflow-hidden border border-line bg-[var(--if-panel-solid)] text-fg" role="menu" aria-label="MinPoint actions">
      <div className="flex h-8 shrink-0 items-center gap-1.5 border-b border-line px-2.5 text-[11.5px] text-muted">
        {stack.length > 1 ? (
          <button className="flex items-center gap-1 hover:text-fg" onClick={() => setStack(stack.slice(0, -1))}><Icon name="chevron-left" size={13} /> {level.name}</button>
        ) : (
          <>
            <Icon name="sparkles" size={12} className="text-accent" />
            <span className="truncate">{inv.selectionLen > 0 ? `${inv.selectionLen} characters selected` : inv.editable ? "Text field" : "MinPoint"}</span>
            <span className="ml-auto truncate text-faint">{inv.app?.name}</span>
          </>
        )}
      </div>
      <div className="flex-1 overflow-y-auto py-1">
        {level.items.map((e, i) =>
          e.kind === "separator" ? (
            <div key={i} className="mx-2.5 my-1 h-px bg-line" />
          ) : (
            <button
              key={e.id}
              role="menuitem"
              aria-disabled={e.kind === "item" && !e.available}
              onMouseEnter={() => selectable.includes(i) && setIdx(i)}
              onClick={() => choose(e)}
              className={`mx-1 flex w-[calc(100%-8px)] items-center gap-2.5 rounded-[6px] px-2 text-left text-[13px] ${compact ? "h-[26px]" : "h-[30px]"} ${idx === i ? "row-active" : ""} ${e.kind === "item" && !e.available ? "opacity-45" : ""}`}
            >
              <Icon name={e.kind === "submenu" ? "folder" : e.icon} size={14} className={idx === i ? "" : "text-muted"} />
              <span className="flex-1 truncate">{e.name}</span>
              {e.kind === "item" && e.requiresAi && <Icon name="cloud" size={11} className={idx === i ? "opacity-80" : "text-faint"} />}
              {e.kind === "submenu" && <Icon name="chevron-right" size={13} className="opacity-60" />}
            </button>
          ),
        )}
      </div>
      {(msg || busy) && <div className="border-t border-line px-3 py-2 text-[11.5px] text-muted">{busy ? "Working…" : msg}</div>}
    </div>
  );
}

// ---------------- Preview ----------------

const TONES: [WritingOp, string][] = [["professional", "Professional"], ["casual", "Casual"], ["friendly", "Friendly"], ["formal", "Formal"], ["shorten", "Shorter"], ["expand", "Longer"], ["simplify", "Simpler"]];

function PreviewView({ payload, close }: { payload: any; close: () => void }) {
  const [result, setResult] = useState<string>(payload.result ?? "");
  const [streaming, setStreaming] = useState<boolean>(payload.streaming);
  const [error, setError] = useState<{ kind: string; message: string } | null>(null);
  const [showDiff, setShowDiff] = useState(false);
  const [showOriginal, setShowOriginal] = useState(false);
  const [custom, setCustom] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [undo, setUndo] = useState(false);
  const autoDone = useRef(false);

  useEffect(() => {
    setResult(payload.result ?? "");
    setStreaming(payload.streaming);
    setError(null);
    setStatus(null);
    autoDone.current = false;
    api.hasUndo().then(setUndo);
  }, [payload]);

  useEffect(() => {
    const a = listen<string>("preview:delta", (e) => setResult((r) => r + e.payload));
    const b = listen<{ result: string; autoReplace: boolean }>("preview:done", (e) => {
      setResult(e.payload.result);
      setStreaming(false);
      if (e.payload.autoReplace && !autoDone.current) {
        autoDone.current = true;
        api.overlayAccept(e.payload.result).catch((err) => setStatus(errorText(err)));
      }
    });
    const c = listen<{ kind: string; message: string }>("preview:error", (e) => { setError(e.payload); setStreaming(false); });
    return () => void [a, b, c].forEach((p) => p.then((f) => f()));
  }, []);

  const replace = async () => {
    setStatus("Inserting…");
    try {
      const how = await api.overlayAccept(result);
      setStatus(how === "paste" ? "Replaced (via paste)" : "Replaced");
    } catch (e) {
      setStatus(errorText(e));
    }
  };
  const regenerate = (op?: WritingOp) => {
    setResult("");
    setStreaming(true);
    setError(null);
    api.overlayRegenerate(custom || undefined, op).catch((e) => { setError({ kind: "api", message: errorText(e) }); setStreaming(false); });
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter" && !streaming && payload.replaceable) replace();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const diff = useMemo(() => (showDiff && !streaming ? wordDiff(payload.original ?? "", result) : null), [showDiff, streaming, result, payload.original]);

  return (
    <Shell title={payload.title} icon={payload.ai ? "sparkles" : "wand"} onClose={close}>
      <div className="flex min-h-0 flex-1 flex-col gap-2 p-3">
        {payload.original && (
          <button onClick={() => setShowOriginal(!showOriginal)} className="flex items-center gap-1 self-start text-[11.5px] text-muted hover:text-fg">
            <Icon name={showOriginal ? "chevron-down" : "chevron-right"} size={12} /> Original
          </button>
        )}
        {showOriginal && <div className="selectable max-h-[90px] overflow-y-auto whitespace-pre-wrap rounded-[7px] bg-raised p-2 text-[12px] text-muted">{payload.original}</div>}
        {error ? (
          <div className="flex flex-1 flex-col items-start gap-2 rounded-[8px] bg-bad/5 p-3 text-[12.5px]">
            <div className="flex items-center gap-1.5 font-medium text-bad"><Icon name="triangle-exclamation-point" size={14} /> {error.message}</div>
            {error.kind === "usage_limit" && <button className="rounded-[8px] btn-grad px-2.5 py-1 text-[12px]" onClick={() => api.openExternal("manage_usage")}>Manage usage</button>}
            {["reauth", "not_connected", "plan_not_authorized"].includes(error.kind) && <span className="text-muted">Open MinPoint → Connected Account to reconnect.</span>}
          </div>
        ) : diff ? (
          <div className="selectable min-h-0 flex-1 overflow-y-auto whitespace-pre-wrap rounded-[8px] border border-line p-2.5 text-[13px] leading-relaxed">
            {diff.map((p, i) => <span key={i} className={p.type === "add" ? "rounded-sm bg-ok/15 text-ok" : p.type === "del" ? "rounded-sm bg-bad/10 text-bad line-through" : ""}>{p.text}</span>)}
          </div>
        ) : (
          <textarea
            className="field min-h-0 flex-1 resize-none text-[13px]"
            value={result}
            onChange={(e) => setResult(e.target.value)}
            readOnly={streaming}
            placeholder={streaming ? "Writing…" : ""}
            aria-label="Result (editable)"
            autoFocus
          />
        )}
        {payload.ai && !error && (
          <div className="flex items-center gap-1.5">
            <input className="field h-7 flex-1 py-0 text-[12px]" placeholder="Custom instructions, e.g. “mention the deadline is Friday”" value={custom} onChange={(e) => setCustom(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !streaming && regenerate()} />
            <select className="field h-7 w-[118px] py-0 text-[12px]" value="" onChange={(e) => e.target.value && regenerate(e.target.value as WritingOp)} disabled={streaming} aria-label="Change tone">
              <option value="">Change tone…</option>
              {TONES.map(([op, l]) => <option key={op} value={op}>{l}</option>)}
            </select>
          </div>
        )}
      </div>
      <div className="flex shrink-0 items-center gap-1.5 border-t border-line px-3 py-2">
        {streaming ? (
          <span className="flex items-center gap-1.5 text-[12px] text-muted"><Icon name="loader-circle" size={13} className="animate-spin" /> {payload.ai ? "Using ChatGPT plan…" : "Working…"}</span>
        ) : (
          <span className="truncate text-[11.5px] text-muted">{status ?? (payload.ai ? "Using ChatGPT plan · review before replacing" : "Computed locally")}</span>
        )}
        <div className="flex-1" />
        {undo && !streaming && <button className="rounded-[6px] px-2 py-1 text-[12px] hover:bg-hover" onClick={() => api.overlayUndo().then(() => setStatus("Undone")).catch((e) => setStatus(errorText(e)))}>Undo last</button>}
        {payload.original && !streaming && !error && <button className={`rounded-[6px] px-2 py-1 text-[12px] hover:bg-hover ${showDiff ? "text-accent" : ""}`} onClick={() => setShowDiff(!showDiff)}>Changes</button>}
        {payload.ai && !streaming && <button className="rounded-[6px] px-2 py-1 text-[12px] hover:bg-hover" onClick={() => regenerate()}>Regenerate</button>}
        <button className="rounded-[6px] px-2 py-1 text-[12px] hover:bg-hover" onClick={close}>{streaming ? "Stop" : "Cancel"}</button>
        <button className="rounded-[6px] border border-line px-2.5 py-1 text-[12px] font-medium hover:bg-hover disabled:opacity-40" disabled={streaming || !result || !!error} onClick={() => api.overlayCopy(result)}>Copy</button>
        {payload.replaceable && (
          <button className="rounded-[8px] btn-grad px-3 py-1 text-[12px] font-medium disabled:opacity-40" disabled={streaming || !result || !!error} onClick={replace} title="⌘↩">
            Replace selection
          </button>
        )}
      </div>
    </Shell>
  );
}

// ---------------- Write with ChatGPT ----------------

function WriteView({ payload, close }: { payload: any; close: () => void }) {
  const [prompt, setPrompt] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const ref = useRef<HTMLTextAreaElement>(null);
  useEffect(() => ref.current?.focus(), []);
  const go = () => api.overlayGenerate(prompt).catch((e) => setErr(errorText(e)));
  return (
    <Shell title={`Write with ChatGPT${payload.app ? ` · ${payload.app}` : ""}`} icon="pen-line" onClose={close}>
      <div className="flex flex-1 flex-col gap-2 p-3">
        <div className="flex flex-wrap gap-1">
          {payload.templates.map((t: any) => (
            <button key={t.id} onClick={() => { setPrompt(t.prefix); ref.current?.focus(); }} className="rounded-full border border-line px-2 py-0.5 text-[11.5px] text-muted hover:bg-hover hover:text-fg">{t.name}</button>
          ))}
        </div>
        <textarea
          ref={ref}
          className="field flex-1 resize-none text-[13px]"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Escape") close(); if ((e.metaKey || e.ctrlKey) && e.key === "Enter") go(); }}
          placeholder="Write a professional email requesting two days of vacation."
        />
        {err && <div className="text-[12px] text-bad">{err}</div>}
      </div>
      <div className="flex items-center gap-2 border-t border-line px-3 py-2">
        <span className="text-[11.5px] text-muted">Using ChatGPT plan · you'll review before anything is inserted. Nothing is sent for you.</span>
        <div className="flex-1" />
        <button className="rounded-[8px] btn-grad px-3 py-1 text-[12px] font-medium disabled:opacity-40" disabled={!prompt.trim()} onClick={go}>Generate ⌘↩</button>
      </div>
    </Shell>
  );
}

// ---------------- List pickers (palette / clipboard / snippets) ----------------

function useListNav(count: number, onEnter: (i: number) => void, close: () => void) {
  const [idx, setIdx] = useState(0);
  useEffect(() => setIdx(0), [count]);
  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") { e.preventDefault(); setIdx((i) => Math.min(i + 1, count - 1)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setIdx((i) => Math.max(i - 1, 0)); }
    else if (e.key === "Enter") { e.preventDefault(); if (count > 0) onEnter(idx); }
    else if (e.key === "Escape") close();
  };
  useEffect(() => document.querySelector(`[data-row="${idx}"]`)?.scrollIntoView({ block: "nearest" }), [idx]);
  return { idx, setIdx, onKeyDown };
}

function SearchBox({ value, onChange, onKeyDown, placeholder, icon }: { value: string; onChange: (v: string) => void; onKeyDown: (e: React.KeyboardEvent) => void; placeholder: string; icon: string }) {
  return (
    <div className="drag-region flex h-12 shrink-0 items-center gap-2.5 border-b border-line px-4">
      <Icon name={icon} size={17} className="text-muted" />
      <input autoFocus className="flex-1 bg-transparent text-[15px] outline-none placeholder:text-faint" style={{ WebkitAppRegion: "no-drag" } as never} value={value} onChange={(e) => onChange(e.target.value)} onKeyDown={onKeyDown} placeholder={placeholder} aria-label={placeholder} />
    </div>
  );
}

function PaletteView({ close }: { close: () => void }) {
  const [q, setQ] = useState("");
  const [res, setRes] = useState<LauncherResponse>({ answer: null, results: [] });
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    api.launcherSearch(q).then((r) => live && setRes(r));
    return () => void (live = false);
  }, [q]);
  const rows = [...(res.answer ? [{ kind: "answer", id: res.answer.value, title: res.answer.display, subtitle: res.answer.kind === "unit" ? "Unit conversion · Enter to copy" : "Calculator · Enter to copy", icon: res.answer.kind === "unit" ? "ruler" : "calculator", score: 0 }] : []), ...res.results];
  const run = (i: number) => {
    const r = rows[i];
    api.launcherExecute(r.kind, r.id).catch((e) => setErr(errorText(e)));
  };
  const nav = useListNav(rows.length, run, close);
  return (
    <div className="anim-pop flex h-screen flex-col overflow-hidden border border-line bg-[var(--if-panel-solid)]">
      <SearchBox value={q} onChange={setQ} onKeyDown={nav.onKeyDown} placeholder="Search apps, actions, snippets — or calculate" icon="command" />
      <div className="flex-1 overflow-y-auto p-1.5">
        {rows.length === 0 && <div className="p-6 text-center text-[12.5px] text-muted">{q ? "No results" : "Type to search"}</div>}
        {rows.map((r, i) => (
          <button key={`${r.kind}:${r.id}`} data-row={i} onMouseEnter={() => nav.setIdx(i)} onClick={() => run(i)} className={`flex w-full items-center gap-3 rounded-[8px] px-3 py-2 text-left ${nav.idx === i ? "row-active" : ""}`}>
            <Icon name={r.icon} size={16} className={nav.idx === i ? "" : "text-muted"} />
            <span className="flex-1 truncate text-[13.5px]">{r.title}</span>
            <span className={`text-[11.5px] ${nav.idx === i ? "opacity-80" : "text-faint"}`}>{r.subtitle}</span>
          </button>
        ))}
      </div>
      {err && <div className="border-t border-line px-3 py-1.5 text-[11.5px] text-bad">{err}</div>}
    </div>
  );
}

function ClipboardView({ payload, close }: { payload: any; close: () => void }) {
  const [q, setQ] = useState("");
  const [items, setItems] = useState<ClipItem[]>(payload.items);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => void api.clipboardList(q).then(setItems), [q]);
  const paste = (i: number) => api.clipboardPaste(items[i].id).catch((e) => setErr(errorText(e)));
  const nav = useListNav(items.length, paste, close);
  return (
    <div className="anim-pop flex h-screen flex-col overflow-hidden border border-line bg-[var(--if-panel-solid)]">
      <SearchBox value={q} onChange={setQ} onKeyDown={nav.onKeyDown} placeholder="Search clipboard history" icon="clipboard-list" />
      <div className="flex-1 overflow-y-auto p-1.5">
        {items.length === 0 && <div className="p-6 text-center text-[12.5px] text-muted">No clipboard items</div>}
        {items.map((it, i) => (
          <button key={it.id} data-row={i} onMouseEnter={() => nav.setIdx(i)} onClick={() => paste(i)} className={`flex w-full items-start gap-2.5 rounded-[8px] px-3 py-2 text-left ${nav.idx === i ? "row-active" : ""}`}>
            <Icon name={it.pinned ? "pin" : it.kind === "files" ? "file" : "type"} size={14} className={`mt-0.5 ${nav.idx === i ? "" : "text-faint"}`} />
            <span className="line-clamp-2 flex-1 whitespace-pre-wrap break-words text-[12.5px]">{it.text}</span>
          </button>
        ))}
      </div>
      <div className="border-t border-line px-3 py-1.5 text-[11px] text-muted">{err ?? "↩ paste as plain text into the previous app · esc close"}</div>
    </div>
  );
}

function SnippetsView({ payload, close }: { payload: any; close: () => void }) {
  const [q, setQ] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const list: Snippet[] = payload.snippets.filter((s: Snippet) => `${s.name} ${s.trigger} ${s.content}`.toLowerCase().includes(q.toLowerCase()));
  const insert = (i: number) => api.snippetInsert(list[i].id).catch((e) => setErr(errorText(e)));
  const nav = useListNav(list.length, insert, close);
  return (
    <div className="anim-pop flex h-screen flex-col overflow-hidden border border-line bg-[var(--if-panel-solid)]">
      <SearchBox value={q} onChange={setQ} onKeyDown={nav.onKeyDown} placeholder="Insert a snippet" icon="text-cursor-input" />
      <div className="flex-1 overflow-y-auto p-1.5">
        {list.length === 0 && <div className="p-6 text-center text-[12.5px] text-muted">{payload.snippets.length ? "No matches" : "No snippets yet — create them in MinPoint → Text Expander."}</div>}
        {list.map((s, i) => (
          <button key={s.id} data-row={i} onMouseEnter={() => nav.setIdx(i)} onClick={() => insert(i)} className={`flex w-full flex-col rounded-[8px] px-3 py-2 text-left ${nav.idx === i ? "row-active" : ""}`}>
            <span className="flex items-center gap-2 text-[13px] font-medium">{s.name}{s.trigger && <code className="mono text-[11px] opacity-70">{s.trigger}</code>}</span>
            <span className={`line-clamp-1 text-[12px] ${nav.idx === i ? "opacity-80" : "text-muted"}`}>{s.content}</span>
          </button>
        ))}
      </div>
      {err && <div className="border-t border-line px-3 py-1.5 text-[11.5px] text-bad">{err}</div>}
    </div>
  );
}

function OcrView({ payload, close }: { payload: any; close: () => void }) {
  const [text, setText] = useState<string>(payload.text);
  const [msg, setMsg] = useState<string | null>(null);
  useEffect(() => setText(payload.text), [payload]);
  const empty = !payload.text.trim();
  return (
    <Shell title={`Captured text${payload.lines ? ` · ${payload.lines} lines` : ""}`} icon="scan-text" onClose={close}>
      <div className="flex flex-1 flex-col p-3">
        {empty ? (
          <div className="flex flex-1 items-center justify-center text-[13px] text-muted">No readable text found in that area.</div>
        ) : (
          <textarea className="field flex-1 resize-none text-[13px]" value={text} onChange={(e) => setText(e.target.value)} autoFocus aria-label="Recognized text" onKeyDown={(e) => e.key === "Escape" && close()} />
        )}
      </div>
      <div className="flex items-center gap-1.5 border-t border-line px-3 py-2">
        <span className="truncate text-[11.5px] text-muted">{msg ?? "Recognized on this Mac"}</span>
        <div className="flex-1" />
        <button className="rounded-[6px] px-2 py-1 text-[12px] hover:bg-hover disabled:opacity-40" disabled={empty} onClick={() => api.speak(text).catch((e) => setMsg(errorText(e)))}>Read aloud</button>
        {payload.aiReady && <button className="rounded-[6px] px-2 py-1 text-[12px] hover:bg-hover disabled:opacity-40" disabled={empty} onClick={() => api.overlayRunOnText("ai.translate", text).catch((e) => setMsg(errorText(e)))}>Translate</button>}
        {payload.aiReady && <button className="rounded-[6px] px-2 py-1 text-[12px] hover:bg-hover disabled:opacity-40" disabled={empty} onClick={() => api.overlayRunOnText("ai.polish", text).catch((e) => setMsg(errorText(e)))}>Polish</button>}
        <button className="rounded-[8px] btn-grad px-3 py-1 text-[12px] font-medium disabled:opacity-40" disabled={empty} onClick={() => api.overlayCopy(text)}>Copy</button>
      </div>
    </Shell>
  );
}

// ---------------- Root ----------------

const DISMISS_ON_BLUR = new Set(["menu", "palette", "clipboard", "snippets"]);

export function OverlayApp() {
  const [show, setShow] = useState<Show | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  useTheme(settings?.appearance);
  const close = useCallback(() => {
    api.overlayCancel();
    setShow(null);
  }, []);

  useEffect(() => {
    // The overlay window is opaque (no private transparency APIs), so match the panel colour.
    document.body.style.background = "var(--if-panel-solid)";
    const a = listen<Show>("overlay:show", (e) => {
      setShow({ ...e.payload, seq: Date.now() });
      api.bootstrap().then((b) => setSettings(b.settings));
    });
    const b = listen("overlay:blur", () => {
      setShow((s) => {
        if (s && DISMISS_ON_BLUR.has(s.mode)) {
          api.overlayCancel();
          return null;
        }
        return s;
      });
    });
    return () => void [a, b].forEach((p) => p.then((f) => f()));
  }, []);

  if (!show) return null;
  const key = String(show.seq);
  switch (show.mode) {
    case "menu": return <MenuView key={key} payload={show.payload} settings={settings} close={close} />;
    case "preview": return <PreviewView payload={show.payload} close={close} />;
    case "write": return <WriteView payload={show.payload} close={close} />;
    case "palette": return <PaletteView key={key} close={close} />;
    case "clipboard": return <ClipboardView key={key} payload={show.payload} close={close} />;
    case "snippets": return <SnippetsView key={key} payload={show.payload} close={close} />;
    case "ocr": return <OcrView payload={show.payload} close={close} />;
    default: return null;
  }
}
