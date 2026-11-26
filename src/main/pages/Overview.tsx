import { useStore } from "../store";
import { Badge, Button, Icon, PageHeader, Section, prettyAccelerator } from "../../components/ui";
import type { PermState } from "../../lib/api";

function Stat({ label, value, sub, icon, onClick }: { label: string; value: string; sub?: string; icon: string; onClick?: () => void }) {
  return (
    <button onClick={onClick} className="flex flex-col items-start rounded-[10px] border border-line bg-panel p-3.5 text-left hover:bg-hover disabled:hover:bg-panel" disabled={!onClick}>
      <div className="mb-2 flex items-center gap-1.5 text-[12px] text-muted">
        <Icon name={icon} size={14} /> {label}
      </div>
      <div className="text-[15px] font-semibold">{value}</div>
      {sub && <div className="mt-0.5 text-[11.5px] text-muted">{sub}</div>}
    </button>
  );
}

const permLabel: Record<string, string> = { accessibility: "Accessibility", inputMonitoring: "Input Monitoring", screenRecording: "Screen Recording", microphone: "Microphone" };

export function Overview() {
  const { status, settings, setPage, actions } = useStore();
  const ai = status?.ai;
  const perms = status?.permissions;
  const granted = perms ? Object.values(perms).filter((p: PermState) => p === "granted").length : 0;
  const favorites = settings.menu.favorites.map((id) => actions.find((a) => a.id === id)).filter(Boolean);
  const actionName = (id: string) => actions.find((a) => a.id === id)?.name ?? ({ "menu.open": "Open context menu", "app.emergency_disable": "Emergency pause" } as Record<string, string>)[id] ?? id;

  return (
    <>
      <PageHeader title="Overview" subtitle="Everything InputFlow is doing on this computer, at a glance." />

      {status && !status.helperRunning && (
        <div className="mb-5 flex items-center gap-3 rounded-[10px] border border-bad/30 bg-bad/5 p-3 text-[13px]">
          <Icon name="circle-alert" className="text-bad" />
          <div className="flex-1">
            {status.platform === "unsupported"
              ? "System-wide features aren't available on this operating system in this build. Local tools in the app still work."
              : "InputFlow's native helper isn't running, so system-wide features are unavailable."}
          </div>
        </div>
      )}
      {status?.inputTapError && !status.paused && (
        <div className="mb-5 flex items-center gap-3 rounded-[10px] border border-warn/30 bg-warn/5 p-3 text-[13px]">
          <Icon name="triangle-alert" className="text-warn" />
          <div className="flex-1">Shift + Right Click is inactive: {status.inputTapError}</div>
          <Button size="sm" onClick={() => setPage("permissions")}>Fix permissions</Button>
        </div>
      )}

      <div className="mb-6 grid grid-cols-3 gap-3">
        <Stat
          icon="activity"
          label="Service"
          value={status?.paused ? "Paused" : status?.helperRunning ? "Running" : "Unavailable"}
          sub={status ? (status.inputTapRunning ? "Shift + Right Click active" : "Shift + Right Click inactive") : "…"}
        />
        <Stat
          icon="circle-user"
          label="ChatGPT"
          value={ai?.state === "connected" ? (ai.planUsageAuthorized ? "Connected" : "Plan usage off") : ai?.state === "connecting" ? "Connecting…" : "Not connected"}
          sub={ai?.state === "connected" ? ai.email ?? undefined : "Optional — local features work without it"}
          onClick={() => setPage("account")}
        />
        <Stat icon="shield-check" label="Permissions" value={perms ? `${granted} of 4 granted` : "…"} sub="Requested only when a feature needs them" onClick={() => setPage("permissions")} />
        <Stat icon="app-window" label="Active app" value={status?.activeApp ?? "—"} sub={status?.activeProfile ? `Profile: ${status.activeProfile}` : "Default profile"} onClick={() => setPage("profiles")} />
        <Stat icon="clipboard-list" label="Clipboard history" value={settings.clipboard.historyEnabled ? `${status?.clipboardItems ?? 0} items` : "Off"} sub={settings.clipboard.historyEnabled ? `Kept ${settings.clipboard.retentionDays} days` : "Opt-in"} onClick={() => setPage("clipboard")} />
        <Stat
          icon="cpu"
          label="InputFlow resource use"
          value={status?.resources ? `${status.resources.appMemoryMb} MB` : "—"}
          sub={status?.resources ? `${status.resources.appCpuPercent.toFixed(1)}% CPU · measured just now` : "Measured live"}
        />
      </div>

      <div className="grid grid-cols-2 gap-5">
        <Section title="Modules">
          {[
            { id: "actions", label: "Universal context menu", on: settings.menu.shiftRightClick && !!status?.inputTapRunning, icon: "mouse-pointer-click" },
            { id: "writing", label: "Writing assistant", on: settings.ai.enabled && ai?.state === "connected" && ai.planUsageAuthorized, icon: "sparkles" },
            { id: "clipboard", label: "Smart clipboard", on: settings.clipboard.historyEnabled, icon: "clipboard-list" },
            { id: "snippets", label: "Text expander", on: settings.expander.enabled, icon: "text-cursor-input" },
            { id: "voice", label: "Voice typing", on: false, icon: "mic", planned: true },
          ].map((m) => (
            <button key={m.id} onClick={() => setPage(m.id)} className="flex w-full items-center gap-3 border-b border-line px-4 py-2.5 text-left text-[13px] last:border-b-0 hover:bg-hover">
              <Icon name={m.icon} className="text-muted" />
              <span className="flex-1">{m.label}</span>
              {m.planned ? <Badge>Planned</Badge> : m.on ? <Badge tone="ok">On</Badge> : <Badge>Off</Badge>}
            </button>
          ))}
        </Section>

        <Section title="Permissions">
          {perms &&
            (Object.keys(permLabel) as (keyof typeof perms)[]).map((k) => (
              <button key={k} onClick={() => setPage("permissions")} className="flex w-full items-center gap-3 border-b border-line px-4 py-2.5 text-left text-[13px] last:border-b-0 hover:bg-hover">
                <span className="flex-1">{permLabel[k]}</span>
                {perms[k] === "granted" ? <Badge tone="ok">Granted</Badge> : perms[k] === "unsupported" ? <Badge>N/A</Badge> : <Badge tone="warn">Not granted</Badge>}
              </button>
            ))}
        </Section>

        <Section title="Shortcuts">
          {(status?.shortcuts ?? []).map((s) => (
            <div key={s.actionId} className="flex items-center gap-3 border-b border-line px-4 py-2.5 text-[13px] last:border-b-0">
              <span className="flex-1 truncate">{actionName(s.actionId)}</span>
              <kbd className="rounded border border-line bg-raised px-1.5 py-0.5 text-[11.5px]">{prettyAccelerator(s.accelerator)}</kbd>
              {!s.registered && <span title={s.error ?? ""}><Icon name="triangle-alert" size={14} className="text-warn" /></span>}
            </div>
          ))}
          <div className="flex items-center gap-3 px-4 py-2.5 text-[13px]">
            <span className="flex-1">Universal context menu</span>
            <kbd className="rounded border border-line bg-raised px-1.5 py-0.5 text-[11.5px]">Shift + Right Click</kbd>
          </div>
        </Section>

        <div>
          <Section title="Favorite actions">
            {favorites.length === 0 ? (
              <div className="px-4 py-3 text-[12.5px] text-muted">Star actions in Quick Actions to pin them to the top of the menu.</div>
            ) : (
              favorites.map((a) => (
                <div key={a!.id} className="flex items-center gap-3 border-b border-line px-4 py-2.5 text-[13px] last:border-b-0">
                  <Icon name={a!.icon} className="text-muted" />
                  <span className="flex-1">{a!.name}</span>
                </div>
              ))
            )}
          </Section>
          <Section title="Recently used" description="Action names and counts only — never the text you worked on.">
            {(status?.recentActions ?? []).length === 0 ? (
              <div className="px-4 py-3 text-[12.5px] text-muted">Nothing yet. Try Shift + Right Click on some selected text.</div>
            ) : (
              status!.recentActions.map((r) => (
                <div key={r.actionId} className="flex items-center gap-3 border-b border-line px-4 py-2 text-[13px] last:border-b-0">
                  <span className="flex-1">{r.name}</span>
                  <span className="text-[12px] text-muted">{r.count}×</span>
                </div>
              ))
            )}
          </Section>
        </div>
      </div>
    </>
  );
}
