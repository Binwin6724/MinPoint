import { useStore } from "../store";
import { PageHeader, Row, Section, ShortcutRecorder } from "../../components/ui";

export function Launcher() {
  const { settings, update } = useStore();
  return (
    <>
      <PageHeader title="Quick Launcher" subtitle="A fast command palette for apps, MinPoint actions, snippets, settings, calculations and unit conversions." />
      <Section title="Open">
        <Row icon="command" title="Launcher shortcut" description="Alt + Space by default. Esc closes it.">
          <ShortcutRecorder value={settings.shortcuts["launcher.open"] ?? ""} onChange={(v) => update((s) => void (s.shortcuts["launcher.open"] = v))} />
        </Row>
      </Section>
      <Section title="What you can type">
        <Row icon="app-window" title="App names" description="“saf” → Safari. Fuzzy matching; frequently used results rise to the top." />
        <Row icon="calculator" title="Calculations" description="“(1200 * 1.18) / 3”, “15% of 240”, “sqrt(2)”, “5!”. Evaluated by a safe local parser — no code is executed." />
        <Row icon="ruler" title="Unit conversions" description="“5 km to mi”, “72 f in c”, “2.5 GB to MB”, “90 min to hours”." />
        <Row icon="text-cursor-input" title="Snippets" description="Search by name or abbreviation and press Enter to insert." />
        <Row icon="zap" title="Actions & settings" description="“clipboard”, “screen text”, “left half”, “pause”, “permissions”…" />
      </Section>
    </>
  );
}
