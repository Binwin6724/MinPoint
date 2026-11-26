import { useMemo, useState } from "react";
import { useStore } from "../store";
import { Icon, OnlineDot, PageHeader, Row, Section, ShortcutRecorder, Toggle } from "../../components/ui";
import type { ActionDef } from "../../lib/api";

const CONTEXTS: { id: "selection" | "editable" | "anywhere"; title: string; description: string }[] = [
  { id: "selection", title: "When text is selected", description: "Writing, case changes, developer and text utilities." },
  { id: "editable", title: "When typing in a text field", description: "Insert, paste and generate." },
  { id: "anywhere", title: "Anywhere", description: "Launcher, capture and window tools." },
];

export function QuickActions() {
  const { settings, actions, update, status } = useStore();
  const [dragId, setDragId] = useState<string | null>(null);
  const [filter, setFilter] = useState("");

  const ordered = useMemo(() => {
    const byId = new Map(actions.map((a) => [a.id, a]));
    const first = settings.menu.order.map((id) => byId.get(id)).filter(Boolean) as ActionDef[];
    return [...first, ...actions.filter((a) => !settings.menu.order.includes(a.id))];
  }, [actions, settings.menu.order]);

  const toggleDisabled = (id: string, enabled: boolean) =>
    update((s) => {
      s.menu.disabled = enabled ? s.menu.disabled.filter((x) => x !== id) : [...s.menu.disabled, id];
    });
  const toggleFav = (id: string) =>
    update((s) => {
      s.menu.favorites = s.menu.favorites.includes(id) ? s.menu.favorites.filter((x) => x !== id) : [...s.menu.favorites, id];
    });
  const drop = (targetId: string) => {
    if (!dragId || dragId === targetId) return;
    const ids = ordered.map((a) => a.id).filter((x) => x !== dragId);
    ids.splice(ids.indexOf(targetId), 0, dragId);
    update((s) => void (s.menu.order = ids));
    setDragId(null);
  };

  return (
    <>
      <PageHeader title="Quick Actions" subtitle="Shift + Right Click anywhere to open MinPoint's menu. Your app's normal right-click menu is untouched — only Shift + Right Click is intercepted." />

      <Section title="Trigger">
        <Row icon="mouse-pointer-click" title="Shift + Right Click" description={status?.inputTapRunning ? "Active." : status?.inputTapError ?? "Needs Accessibility and Input Monitoring permission."}>
          <Toggle label="Shift + Right Click" checked={settings.menu.shiftRightClick} onChange={(v) => update((s) => void (s.menu.shiftRightClick = v))} />
        </Row>
        <Row icon="keyboard" title="Keyboard shortcut" description="Opens the menu at the mouse pointer.">
          <ShortcutRecorder value={settings.shortcuts["menu.open"] ?? ""} onChange={(v) => update((s) => void (s.shortcuts["menu.open"] = v))} />
        </Row>
        <Row title="Hide unavailable actions" description="Hide actions this computer can't perform right now (AI actions stay visible with a hint to connect).">
          <Toggle label="Hide unavailable actions" checked={settings.menu.hideUnavailable} onChange={(v) => update((s) => void (s.menu.hideUnavailable = v))} />
        </Row>
        <Row title="Group related actions into submenus" description="Tone, text case, developer tools and window actions.">
          <Toggle label="Group into submenus" checked={settings.menu.showGroupsAsSubmenus} onChange={(v) => update((s) => void (s.menu.showGroupsAsSubmenus = v))} />
        </Row>
      </Section>

      <div className="mb-3 flex items-center gap-2">
        <div className="relative flex-1">
          <Icon name="search" size={14} className="absolute left-2.5 top-2 text-faint" />
          <input className="field pl-8" placeholder="Filter actions" value={filter} onChange={(e) => setFilter(e.target.value)} aria-label="Filter actions" />
        </div>
        <span className="text-[12px] text-muted">Drag to reorder · ★ to pin to the top</span>
      </div>

      {CONTEXTS.map((c) => {
        const list = ordered.filter((a) => a.contexts.includes(c.id) && a.name.toLowerCase().includes(filter.toLowerCase()));
        if (list.length === 0) return null;
        return (
          <Section key={c.id} title={c.title} description={c.description}>
            {list.map((a) => {
              const enabled = !settings.menu.disabled.includes(a.id);
              const fav = settings.menu.favorites.includes(a.id);
              return (
                <div
                  key={a.id}
                  draggable
                  onDragStart={() => setDragId(a.id)}
                  onDragOver={(e) => e.preventDefault()}
                  onDrop={() => drop(a.id)}
                  className={`flex items-center gap-3 border-b border-line px-3 py-2 last:border-b-0 ${dragId === a.id ? "opacity-50" : ""}`}
                >
                  <Icon name="grip-vertical" size={14} className="cursor-grab text-faint" />
                  <Icon name={a.icon} size={15} className={enabled ? "text-fg" : "text-faint"} />
                  <div className="min-w-0 flex-1">
                    <div className={`text-[13px] ${enabled ? "" : "text-muted"}`}>{a.name}</div>
                    {a.description && <div className="truncate text-[11.5px] text-muted">{a.description}</div>}
                  </div>
                  <OnlineDot ai={a.requiresAi} />
                  <button onClick={() => toggleFav(a.id)} aria-label={fav ? "Unpin" : "Pin to top"} className={`rounded p-1 ${fav ? "text-warn" : "text-faint hover:text-fg"}`}>
                    <Icon name="star" size={14} className={fav ? "fill-current" : ""} />
                  </button>
                  <Toggle label={`Show ${a.name}`} checked={enabled} onChange={(v) => toggleDisabled(a.id, v)} />
                </div>
              );
            })}
          </Section>
        );
      })}
    </>
  );
}
