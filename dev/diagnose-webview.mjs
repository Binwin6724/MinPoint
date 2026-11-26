// CI diagnostic: attach to a running Tauri/WebView2 app over the Chrome DevTools
// Protocol (WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222),
// reload each page, and record console output, exceptions, failed requests,
// the rendered DOM and a screenshot. Development tool only; never shipped.
import { writeFileSync, mkdirSync } from "node:fs";

const out = process.argv[2] ?? "diag";
mkdirSync(out, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let targets = [];
for (let i = 0; i < 60; i++) {
  try {
    targets = (await (await fetch("http://127.0.0.1:9222/json/list")).json()).filter((t) => t.type === "page");
    if (targets.length) break;
  } catch {}
  await sleep(1000);
}
const report = { targets: targets.map((t) => ({ url: t.url, title: t.title })), pages: [] };
console.log("targets:", JSON.stringify(report.targets));

for (const [n, t] of targets.entries()) {
  const ws = new WebSocket(t.webSocketDebuggerUrl);
  await new Promise((r, j) => { ws.onopen = r; ws.onerror = j; });
  let id = 0;
  const pending = new Map();
  const events = [];
  ws.onmessage = (m) => {
    const d = JSON.parse(m.data);
    if (d.id && pending.has(d.id)) { pending.get(d.id)(d); pending.delete(d.id); return; }
    if (d.method === "Runtime.consoleAPICalled") events.push({ console: d.params.type, text: d.params.args.map((a) => a.value ?? a.description ?? a.type).join(" ") });
    else if (d.method === "Runtime.exceptionThrown") events.push({ exception: d.params.exceptionDetails.exception?.description ?? d.params.exceptionDetails.text });
    else if (d.method === "Log.entryAdded") events.push({ log: d.params.entry.level, text: d.params.entry.text, url: d.params.entry.url });
    else if (d.method === "Network.loadingFailed") events.push({ networkFailed: d.params.errorText, blocked: d.params.blockedReason, requestId: d.params.requestId });
    else if (d.method === "Network.requestWillBeSent") events.push({ request: d.params.request.url.slice(0, 200) });
  };
  const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
  for (const d of ["Runtime.enable", "Log.enable", "Page.enable", "Network.enable"]) await send(d);
  await send("Page.reload", { ignoreCache: true });
  await sleep(12000);
  const ev = async (expr) => (await send("Runtime.evaluate", { expression: expr, returnByValue: true })).result?.result?.value;
  const page = {
    url: t.url,
    location: await ev("location.href"),
    readyState: await ev("document.readyState"),
    rootHtmlLength: await ev("document.getElementById('root')?.innerHTML.length ?? -1"),
    bodyText: await ev("document.body.innerText.slice(0, 1500)"),
    scripts: await ev("[...document.scripts].map(s => s.src || '(inline)')"),
    hasTauriInternals: await ev("typeof window.__TAURI_INTERNALS__"),
    events,
  };
  const shot = await send("Page.captureScreenshot", { format: "png" });
  if (shot.result?.data) writeFileSync(`${out}/page-${n}.png`, Buffer.from(shot.result.data, "base64"));
  report.pages.push(page);
  ws.close();
}
writeFileSync(`${out}/report.json`, JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
