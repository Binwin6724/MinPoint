import { useEffect, useState } from "react";
import { useStore } from "../store";
import { api, errorText, type Voice } from "../../lib/api";
import { Badge, Button, PageHeader, Row, Section, ShortcutRecorder, Toggle } from "../../components/ui";

function download(name: string, text: string) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  a.download = name;
  a.click();
  URL.revokeObjectURL(a.href);
}

export function SettingsPage() {
  const { settings, update, toast, boot, status } = useStore();
  const [voices, setVoices] = useState<Voice[]>([]);
  const [q, setQ] = useState("");
  useEffect(() => {
    api.voices().then(setVoices).catch(() => setVoices([]));
  }, []);
  const show = (...words: string[]) => !q || words.join(" ").toLowerCase().includes(q.toLowerCase());
  const conflicts = (status?.shortcuts ?? []).filter((s) => !s.registered && s.error && !s.error.includes("paused"));

  return (
    <>
      <PageHeader title="Settings" subtitle={`InputFlow ${boot.version} · ${boot.os}`} actions={<input className="field w-[220px]" placeholder="Search settings" value={q} onChange={(e) => setQ(e.target.value)} aria-label="Search settings" />} />

      {show("general startup login close menu bar background") && (
        <Section title="General">
          <Row title="Open at login" description="Start InputFlow in the menu bar when you log in.">
            <Toggle label="Open at login" checked={settings.general.launchAtLogin} onChange={(v) => api.setLaunchAtLogin(v).then((on) => update((s) => void (s.general.launchAtLogin = on))).catch((e) => toast(errorText(e), "error"))} />
          </Row>
          <Row title="Keep running when the window is closed" description="Shortcuts, the context menu and clipboard history keep working from the menu bar.">
            <Toggle label="Keep running" checked={settings.general.keepRunningOnClose} onChange={(v) => update((s) => void (s.general.keepRunningOnClose = v))} />
          </Row>
        </Section>
      )}

      {show("shortcut emergency pause keyboard hotkey") && (
        <Section title="Global shortcuts" description="Shortcuts that conflict with the system or each other are not registered.">
          <Row title="Emergency pause" description="Instantly releases every InputFlow hook and shortcut. Press again to resume. Always active.">
            <ShortcutRecorder value={settings.shortcuts["app.emergency_disable"] ?? ""} onChange={(v) => update((s) => void (s.shortcuts["app.emergency_disable"] = v))} />
          </Row>
          <Row title="Quick Launcher"><ShortcutRecorder value={settings.shortcuts["launcher.open"] ?? ""} onChange={(v) => update((s) => void (s.shortcuts["launcher.open"] = v))} /></Row>
          <Row title="Context menu at pointer"><ShortcutRecorder value={settings.shortcuts["menu.open"] ?? ""} onChange={(v) => update((s) => void (s.shortcuts["menu.open"] = v))} /></Row>
          <Row title="Clipboard history"><ShortcutRecorder value={settings.shortcuts["clip.history"] ?? ""} onChange={(v) => update((s) => void (s.shortcuts["clip.history"] = v))} /></Row>
          <Row title="Screen Text Capture"><ShortcutRecorder value={settings.shortcuts["ocr.capture"] ?? ""} onChange={(v) => update((s) => void (s.shortcuts["ocr.capture"] = v))} /></Row>
          {conflicts.map((c) => (
            <div key={c.actionId} className="border-t border-line bg-warn/5 px-4 py-2 text-[12px] text-warn">{c.actionId}: {c.error}</div>
          ))}
        </Section>
      )}

      {show("read aloud speech voice tts speak") && (
        <Section title="Read Aloud" description="Uses voices installed on this computer. They work offline.">
          <Row title="Voice">
            <select className="field w-[260px]" value={settings.speech.voice ?? ""} onChange={(e) => update((s) => void (s.speech.voice = e.target.value || null))} aria-label="Voice">
              <option value="">System default</option>
              {voices.map((v) => <option key={v.id} value={v.id}>{v.name} ({v.language}){v.quality !== "default" ? ` · ${v.quality}` : ""}</option>)}
            </select>
          </Row>
          <Row title="Speed">
            <input type="range" min={0.5} max={2} step={0.1} value={settings.speech.rate} onChange={(e) => update((s) => void (s.speech.rate = Number(e.target.value)))} className="w-[200px] accent-[var(--if-accent)]" aria-label="Speed" />
            <span className="w-8 text-[12px] text-muted">{settings.speech.rate.toFixed(1)}×</span>
          </Row>
          <Row title="Volume">
            <input type="range" min={0} max={1} step={0.05} value={settings.speech.volume} onChange={(e) => update((s) => void (s.speech.volume = Number(e.target.value)))} className="w-[200px] accent-[var(--if-accent)]" aria-label="Volume" />
          </Row>
          <Row title="Try it">
            <Button size="sm" onClick={() => api.speak("InputFlow. Your computer, your controls.").catch((e) => toast(errorText(e), "error"))}>Play sample</Button>
            <Button size="sm" variant="ghost" onClick={() => api.speechControl("stop")}>Stop</Button>
          </Row>
        </Section>
      )}

      {show("export import backup configuration") && (
        <Section title="Backup">
          <Row title="Export configuration" description="Settings, profiles, custom actions and snippets as a JSON file. Contains no clipboard history or credentials.">
            <Button size="sm" onClick={async () => download("inputflow-config.json", await api.exportConfig())}>Export…</Button>
          </Row>
          <Row title="Import configuration" description="Imported files are data only. They can't turn on clipboard history, text expansion or auto-replace for you.">
            <label className="inline-flex h-7 cursor-pointer items-center rounded-[7px] border border-line bg-panel px-2.5 text-[12px] font-medium hover:bg-hover">
              Import…
              <input type="file" accept=".json,application/json" className="hidden" onChange={async (e) => {
                const f = e.target.files?.[0];
                if (!f) return;
                try { await api.importConfig(await f.text()); toast("Configuration imported — reloading", "success"); setTimeout(() => location.reload(), 600); } catch (err) { toast(errorText(err), "error"); }
              }} />
            </label>
          </Row>
        </Section>
      )}

      {show("about updates version") && (
        <Section title="About">
          <Row title="Version" description="Free and open source. No subscription, ads, account or telemetry."><Badge>{boot.version}</Badge></Row>
          <Row title="Updates" description="Signed automatic updates are not configured in this build. Download new versions from the project's releases page." />
        </Section>
      )}
    </>
  );
}

export function PrivacyCenter() {
  const { settings, update, status, toast, setPage } = useStore();
  const run = async (p: Promise<unknown>, msg: string) => {
    try { await p; toast(msg, "success"); } catch (e) { toast(errorText(e), "error"); }
  };
  return (
    <>
      <PageHeader title="Privacy Center" subtitle="InputFlow keeps everything on this computer. Nothing is uploaded unless you run a ChatGPT action, and then only the text you chose." />
      <Section title="What's stored on this computer">
        <Row icon="clipboard-list" title="Clipboard history" description={settings.clipboard.historyEnabled ? `${status?.clipboardItems ?? 0} items, encrypted with a key kept in your Keychain` : "Off — nothing recorded"}>
          <Button size="sm" variant="danger" onClick={() => confirm("Delete all clipboard history?") && run(api.clipboardClear(false), "Clipboard history deleted")}>Clear</Button>
        </Row>
        <Row icon="history" title="Activity" description="Which actions you used and how often — never the text involved. Powers ‘Recently used’ and launcher ranking.">
          <Button size="sm" onClick={() => run(api.clearActivity(), "Activity cleared")}>Clear</Button>
        </Row>
        <Row icon="text-cursor-input" title="Snippets & settings" description={`${status?.snippets ?? 0} snippets, plus your preferences.`} />
        <Row icon="mic-off" title="Voice recordings" description="None. Voice typing isn't in this build; when added, audio will be transcribed locally and not kept." />
        <Row icon="file-text" title="Logs" description="Errors only. Never selected text, clipboard contents, generated text or credentials." />
      </Section>
      <Section title="What leaves this computer">
        <Row icon="cloud" title="ChatGPT actions" description="Only when you choose an AI action: the selected text or your prompt plus the action's instructions, sent to OpenAI with storage disabled.">
          {status?.ai.state === "connected" ? <Button size="sm" onClick={() => setPage("account")}>Manage</Button> : <Badge>Not connected</Badge>}
        </Row>
        <Row icon="ban" title="Nothing else" description="No telemetry, analytics, ads, crash reporting or cloud sync." />
      </Section>
      <Section title="Controls">
        <Row icon="power" title="Pause InputFlow" description="Releases every hook and shortcut except the emergency shortcut.">
          <Toggle label="Pause" checked={settings.general.paused} onChange={(v) => update((s) => void (s.general.paused = v))} />
        </Row>
        <Row icon="shield-check" title="Review permissions"><Button size="sm" onClick={() => setPage("permissions")}>Open</Button></Row>
        <Row icon="app-window" title="Excluded apps" description={`${settings.general.excludedApps.length} apps excluded completely`}><Button size="sm" onClick={() => setPage("profiles")}>Edit</Button></Row>
        <Row icon="trash-2" title="Delete all InputFlow data" description="Disconnects ChatGPT, deletes clipboard history, activity and snippets, and resets settings. Can't be undone.">
          <Button size="sm" variant="danger" onClick={() => confirm("Delete ALL InputFlow data and reset settings? This can't be undone.") && run(api.deleteAllData().then(() => setTimeout(() => location.reload(), 600)), "All data deleted")}>Delete everything</Button>
        </Row>
      </Section>
    </>
  );
}
