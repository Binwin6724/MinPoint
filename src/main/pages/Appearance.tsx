import { useStore } from "../store";
import { PageHeader, Row, Section, Toggle } from "../../components/ui";

const ACCENTS = ["#ff7a45", "#ff4f9a", "#2f80ff", "#7c5cff", "#14d3a0", "#f5a524", "#ef4458", "#8e8e93"];

export function Appearance() {
  const { settings, update } = useStore();
  const a = settings.appearance;
  return (
    <>
      <PageHeader title="Appearance" subtitle="Applies to the main window and to every overlay — the menu, previews, launcher and clipboard picker." />
      <Section title="Theme">
        <Row title="Appearance">
          <div className="flex rounded-[8px] border border-line p-0.5">
            {(["system", "light", "dark"] as const).map((t) => (
              <button key={t} onClick={() => update((s) => void (s.appearance.theme = t))} className={`rounded-[6px] px-3 py-1 text-[12.5px] capitalize ${a.theme === t ? "bg-accent text-accent-fg" : "hover:bg-hover"}`}>{t}</button>
            ))}
          </div>
        </Row>
        <Row title="Accent colour">
          <div className="flex gap-1.5">
            {ACCENTS.map((c) => (
              <button key={c} aria-label={`Accent ${c}`} onClick={() => update((s) => void (s.appearance.accent = c))} className={`h-6 w-6 rounded-full ${a.accent === c ? "ring-2 ring-fg ring-offset-2 ring-offset-panel" : ""}`} style={{ background: c }} />
            ))}
            <input type="color" aria-label="Custom accent" value={a.accent} onChange={(e) => update((s) => void (s.appearance.accent = e.target.value))} className="h-6 w-8 cursor-pointer rounded border-0 bg-transparent p-0" />
          </div>
        </Row>
      </Section>
      <Section title="Size & density">
        <Row title="Text size">
          <input type="range" min={0.85} max={1.3} step={0.05} value={a.fontScale} onChange={(e) => update((s) => void (s.appearance.fontScale = Number(e.target.value)))} aria-label="Text size" className="w-[200px] accent-[var(--if-accent)]" />
        </Row>
        <Row title="Compact menu" description="Tighter rows in the Shift + Right Click menu."><Toggle label="Compact menu" checked={a.compact} onChange={(v) => update((s) => void (s.appearance.compact = v))} /></Row>
      </Section>
      <Section title="Accessibility">
        <Row title="Reduce motion" description="Turns off animations. MinPoint also follows the system setting."><Toggle label="Reduce motion" checked={a.reduceMotion} onChange={(v) => update((s) => void (s.appearance.reduceMotion = v))} /></Row>
        <Row title="Increase contrast" description="Stronger borders and text."><Toggle label="Increase contrast" checked={a.highContrast} onChange={(v) => update((s) => void (s.appearance.highContrast = v))} /></Row>
      </Section>
    </>
  );
}
