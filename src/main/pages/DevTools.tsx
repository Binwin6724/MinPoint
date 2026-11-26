import { useEffect, useState } from "react";
import { useStore } from "../store";
import { api, errorText, type DevOp, type TextOp } from "../../lib/api";
import { Button, Icon, PageHeader, Section } from "../../components/ui";

const GROUPS: { title: string; ops: [string, string, "dev" | "text"][] }[] = [
  { title: "JSON & markup", ops: [["json_format", "Format JSON", "dev"], ["json_minify", "Minify JSON", "dev"], ["json_validate", "Validate JSON", "dev"], ["xml_format", "Format XML", "dev"], ["csv_to_markdown_table", "CSV → Markdown", "dev"]] },
  { title: "Encode", ops: [["base64_encode", "Base64 encode", "dev"], ["base64_decode", "Base64 decode", "dev"], ["url_encode", "URL encode", "dev"], ["url_decode", "URL decode", "dev"]] },
  { title: "Hash", ops: [["sha256", "SHA-256", "dev"], ["sha1", "SHA-1", "dev"], ["md5", "MD5", "dev"]] },
  { title: "Time & colour", ops: [["timestamp_to_date", "Timestamp → date", "dev"], ["date_to_timestamp", "Date → timestamp", "dev"], ["color_convert", "Convert colour", "dev"]] },
  { title: "Naming", ops: [["camel_case", "camelCase", "text"], ["pascal_case", "PascalCase", "text"], ["snake_case", "snake_case", "text"], ["kebab_case", "kebab-case", "text"], ["constant_case", "CONSTANT_CASE", "text"]] },
  { title: "Text", ops: [["uppercase", "UPPER", "text"], ["lowercase", "lower", "text"], ["title_case", "Title Case", "text"], ["sentence_case", "Sentence case", "text"], ["trim_whitespace", "Trim spaces", "text"], ["remove_duplicate_lines", "Dedupe lines", "text"], ["sort_lines", "Sort lines", "text"], ["reverse_lines", "Reverse lines", "text"], ["remove_line_breaks", "Join lines", "text"]] },
];

export function DevTools() {
  const { toast } = useStore();
  const [input, setInput] = useState("");
  const [output, setOutput] = useState("");
  const [err, setErr] = useState(false);
  const [stats, setStats] = useState<Awaited<ReturnType<typeof api.textStats>> | null>(null);
  const [calc, setCalc] = useState("");
  const [calcOut, setCalcOut] = useState("");
  const [re, setRe] = useState("");
  const [reFlags, setReFlags] = useState(false);
  const [matches, setMatches] = useState<Awaited<ReturnType<typeof api.devRegex>> | string>([]);
  const [find, setFind] = useState("");
  const [repl, setRepl] = useState("");
  const [d1, setD1] = useState("");
  const [d2, setD2] = useState("");
  const [days, setDays] = useState("");

  useEffect(() => {
    api.textStats(input).then(setStats);
  }, [input]);
  useEffect(() => {
    if (!calc.trim()) return setCalcOut("");
    api.calculate(calc).then((v) => setCalcOut(`= ${v}`)).catch((e) => setCalcOut(errorText(e)));
  }, [calc]);
  useEffect(() => {
    if (!re) return setMatches([]);
    api.devRegex(re, input, reFlags).then(setMatches).catch((e) => setMatches(errorText(e)));
  }, [re, input, reFlags]);
  useEffect(() => {
    if (!d1 || !d2) return setDays("");
    api.devDateDiff(d1, d2).then((n) => setDays(`${n} days`)).catch(() => setDays(""));
  }, [d1, d2]);

  const run = async (op: string, kind: "dev" | "text") => {
    try {
      setOutput(kind === "dev" ? await api.devTool(op as DevOp, input) : await api.textTool(op as TextOp, input));
      setErr(false);
    } catch (e) {
      setOutput(errorText(e));
      setErr(true);
    }
  };

  return (
    <>
      <PageHeader title="Developer Tools" subtitle="Local utilities — nothing leaves your computer. The same tools are available on selected text through Shift + Right Click → Developer Tools." />
      <div className="mb-6 grid grid-cols-2 gap-3">
        <div>
          <div className="mb-1 flex justify-between px-1 text-[12px] text-muted">
            <span>Input</span>
            {stats && <span>{stats.words} words · {stats.characters} chars · {stats.lines} lines</span>}
          </div>
          <textarea className="field mono h-[220px]" value={input} onChange={(e) => setInput(e.target.value)} placeholder="Paste text, JSON, a timestamp, a colour…" spellCheck={false} />
        </div>
        <div>
          <div className="mb-1 flex justify-between px-1 text-[12px] text-muted">
            <span>Output</span>
            <span className="flex gap-2">
              <button className="hover:text-fg" onClick={() => { navigator.clipboard.writeText(output); toast("Copied", "success"); }} disabled={!output}>Copy</button>
              <button className="hover:text-fg" onClick={() => { setInput(output); setOutput(""); }} disabled={!output || err}>Use as input</button>
            </span>
          </div>
          <textarea readOnly className={`field mono h-[220px] ${err ? "text-bad" : ""}`} value={output} spellCheck={false} />
        </div>
      </div>
      {GROUPS.map((g) => (
        <div key={g.title} className="mb-3 flex items-center gap-2">
          <div className="w-[110px] shrink-0 text-[12px] font-medium text-muted">{g.title}</div>
          <div className="flex flex-wrap gap-1.5">
            {g.ops.map(([op, label, kind]) => (
              <Button key={op} size="sm" onClick={() => run(op, kind)} disabled={!input}>{label}</Button>
            ))}
          </div>
        </div>
      ))}
      <div className="mb-6 flex items-center gap-2">
        <div className="w-[110px] shrink-0 text-[12px] font-medium text-muted">Generate</div>
        <Button size="sm" onClick={async () => setOutput(await api.devUuid())}><Icon name="fingerprint-pattern" size={13} /> UUID v4</Button>
      </div>

      <div className="grid grid-cols-2 gap-5">
        <Section title="Calculator & units">
          <div className="p-3">
            <input className="field mono" value={calc} onChange={(e) => setCalc(e.target.value)} placeholder="15% of 240 · 5 km to mi · sqrt(2)" />
            <div className="selectable mt-2 min-h-[20px] text-[14px] font-medium">{calcOut}</div>
          </div>
        </Section>
        <Section title="Date difference">
          <div className="flex items-center gap-2 p-3">
            <input type="date" className="field" value={d1} onChange={(e) => setD1(e.target.value)} aria-label="Start date" />
            <Icon name="arrow-right" className="shrink-0 text-faint" />
            <input type="date" className="field" value={d2} onChange={(e) => setD2(e.target.value)} aria-label="End date" />
          </div>
          <div className="px-3 pb-3 text-[14px] font-medium">{days}</div>
        </Section>
        <Section title="Regular expression tester" description="Runs against the input above.">
          <div className="p-3">
            <div className="flex gap-2">
              <input className="field mono" value={re} onChange={(e) => setRe(e.target.value)} placeholder="(\d+)-(\w+)" />
              <label className="flex items-center gap-1 text-[12px] text-muted"><input type="checkbox" checked={reFlags} onChange={(e) => setReFlags(e.target.checked)} /> i</label>
            </div>
            <div className="mt-2 max-h-[140px] overflow-y-auto text-[12px]">
              {typeof matches === "string" ? (
                <span className="text-bad">{matches}</span>
              ) : (
                <>
                  {re && <div className="mb-1 text-muted">{matches.length} match{matches.length === 1 ? "" : "es"}</div>}
                  {matches.slice(0, 50).map((m, i) => (
                    <div key={i} className="mono selectable">[{m.start}–{m.end}] {m.text}{m.groups.length > 0 && <span className="text-muted"> · {m.groups.map((g) => g ?? "∅").join(", ")}</span>}</div>
                  ))}
                </>
              )}
            </div>
          </div>
        </Section>
        <Section title="Find & replace" description="Applies to the input above.">
          <div className="space-y-2 p-3">
            <input className="field mono" value={find} onChange={(e) => setFind(e.target.value)} placeholder="Find (regex supported)" />
            <input className="field mono" value={repl} onChange={(e) => setRepl(e.target.value)} placeholder="Replace with ($1 for groups)" />
            <div className="flex gap-2">
              <Button size="sm" disabled={!find} onClick={() => api.findReplace(input, find, repl, true, false).then(setOutput).catch((e) => setOutput(errorText(e)))}>Replace text</Button>
              <Button size="sm" disabled={!find} onClick={() => api.findReplace(input, find, repl, true, true).then(setOutput).catch((e) => setOutput(errorText(e)))}>Replace regex</Button>
            </div>
          </div>
        </Section>
      </div>
    </>
  );
}
