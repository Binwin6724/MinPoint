import { useStore } from "../store";
import { api, errorText } from "../../lib/api";
import { Badge, Button, Icon, PageHeader, Row, Section, ShortcutRecorder, Toggle } from "../../components/ui";

export function ScreenOcr() {
  const { settings, update, status, toast } = useStore();
  const granted = status?.permissions.screenRecording === "granted";
  return (
    <>
      <PageHeader
        title="Screen OCR"
        subtitle="Copy text from anything on screen — images, videos, slides, scanned PDFs, error dialogs. Recognition runs on this Mac with Apple Vision; nothing is uploaded."
        actions={<Button variant="primary" onClick={() => api.ocrCapture().catch((e) => toast(errorText(e), "error"))}><Icon name="scan-text" size={14} /> Capture text</Button>}
      />
      <Section title="Capture">
        <Row icon="keyboard" title="Screen Text Capture shortcut" description="Drag to select a region. Press Esc to cancel.">
          <ShortcutRecorder value={settings.shortcuts["ocr.capture"] ?? ""} onChange={(v) => update((s) => void (s.shortcuts["ocr.capture"] = v))} />
        </Row>
        <Row icon="monitor" title="Screen Recording permission" description="Required by macOS to read pixels of other apps. Screens are captured only when you start a capture — never in the background.">
          {granted ? <Badge tone="ok">Granted</Badge> : <Button size="sm" onClick={() => api.requestPermission("screenRecording").catch((e) => toast(errorText(e), "error"))}>Grant…</Button>}
        </Row>
      </Section>
      <Section title="After capture">
        <Row icon="copy" title="Copy, edit, read aloud or translate" description="Recognised text opens in a small window. Translate and Polish use ChatGPT only if you choose them." />
        <Row icon="languages" title="Languages" description="Apple Vision detects the language automatically. Supported languages include English, French, German, Spanish, Italian, Portuguese, Chinese, Japanese, Korean, Russian, Ukrainian, Arabic, Hindi, Thai, Vietnamese and more (varies by macOS version)." />
      </Section>
    </>
  );
}

export function Screenshots() {
  const { settings, update, toast } = useStore();
  const take = async (mode: "area" | "window" | "full") => {
    try {
      const path = await api.screenshot(mode);
      if (path) toast(`Saved to ${path}`, "success");
    } catch (e) {
      toast(errorText(e), "error");
    }
  };
  return (
    <>
      <PageHeader title="Screenshot Tools" subtitle="Quick, local screenshots using the system capture interface." />
      <Section title="Take a screenshot">
        <div className="grid grid-cols-3 gap-3 p-4">
          {([["area", "Selected area", "square-dashed"], ["window", "Window", "app-window"], ["full", "Full screen", "monitor"]] as const).map(([m, label, icon]) => (
            <button key={m} onClick={() => take(m)} className="flex flex-col items-center gap-2 rounded-[10px] border border-line py-5 text-[13px] hover:bg-hover">
              <Icon name={icon} size={20} className="text-muted" />
              {label}
            </button>
          ))}
        </div>
      </Section>
      <Section title="After capture">
        <Row title="Copy to clipboard"><Toggle label="Copy to clipboard" checked={settings.screenshot.copyToClipboard} onChange={(v) => update((s) => void (s.screenshot.copyToClipboard = v))} /></Row>
        <Row title="Save to disk" description={settings.screenshot.saveDir || "Pictures/MinPoint"}><Toggle label="Save to disk" checked={settings.screenshot.saveToDisk} onChange={(v) => update((s) => void (s.screenshot.saveToDisk = v))} /></Row>
        <Row title="Save folder" description="Leave empty to use Pictures/MinPoint.">
          <input className="field w-[280px]" value={settings.screenshot.saveDir} placeholder="~/Pictures/MinPoint" onChange={(e) => update((s) => void (s.screenshot.saveDir = e.target.value))} aria-label="Save folder" />
        </Row>
      </Section>
      <Section title="Annotation">
        <Row icon="pencil" title="Arrows, boxes, highlights and redaction" description="Not in this build yet. Screenshots are saved and copied as captured." >
          <Badge>Planned</Badge>
        </Row>
      </Section>
    </>
  );
}
