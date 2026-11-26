import { useState } from "react";
import { useStore } from "./store";
import { api, errorText } from "../lib/api";
import { Badge, Button, Icon, useTheme, prettyAccelerator } from "../components/ui";
import { PERMS } from "./pages/Permissions";

const STEPS = ["welcome", "menu", "permissions", "ai", "done"] as const;

export function Onboarding() {
  const { settings, update, status, refresh, toast } = useStore();
  useTheme(settings.appearance);
  const [i, setI] = useState(0);
  const step = STEPS[i];
  const next = () => setI((x) => Math.min(x + 1, STEPS.length - 1));

  return (
    <div className="flex h-full flex-col">
      <div className="drag-region h-[38px] shrink-0" />
      <div className="flex flex-1 items-center justify-center px-10 pb-10">
        <div className="anim-pop w-full max-w-[560px]" key={step}>
          {step === "welcome" && (
            <div className="text-center">
              <img src="/app-icon.png" alt="" className="mx-auto mb-5 h-20 w-20" />
              <h1 className="text-[26px] font-semibold tracking-tight">Welcome to InputFlow</h1>
              <p className="mt-1 text-[15px] text-muted">Your Computer. Your Controls.</p>
              <p className="mx-auto mt-5 max-w-[440px] text-[13.5px] leading-relaxed text-muted">
                One lightweight app for writing help, clipboard history, text expansion, screen text capture, window snapping and a quick launcher — free, private and offline-first. ChatGPT is optional.
              </p>
            </div>
          )}
          {step === "menu" && (
            <div>
              <h2 className="text-[20px] font-semibold">Shift + Right Click, anywhere</h2>
              <p className="mt-2 text-[13.5px] text-muted">Hold Shift and right-click to open InputFlow's menu. It adapts to what you're doing:</p>
              <div className="mt-5 space-y-2">
                {[
                  ["text-select", "Text selected", "Polish, fix grammar, translate, change case, read aloud…"],
                  ["text-cursor", "Typing in a field", "Insert snippets, paste from history, write with ChatGPT…"],
                  ["mouse-pointer-2", "Anywhere else", "Launcher, screen text capture, screenshots, window snapping…"],
                ].map(([icon, t, d]) => (
                  <div key={t} className="flex items-center gap-3 rounded-[10px] border border-line bg-panel p-3">
                    <Icon name={icon} className="text-accent" />
                    <div><div className="text-[13px] font-medium">{t}</div><div className="text-[12px] text-muted">{d}</div></div>
                  </div>
                ))}
              </div>
              <p className="mt-4 text-[12.5px] text-muted">
                A normal right-click still opens your app's own menu. Press <kbd className="rounded border border-line bg-raised px-1">{prettyAccelerator(settings.shortcuts["app.emergency_disable"] ?? "")}</kbd> any time to pause everything. You can also pause from the menu bar icon.
              </p>
            </div>
          )}
          {step === "permissions" && (
            <div>
              <h2 className="text-[20px] font-semibold">Permissions, only when needed</h2>
              <p className="mt-2 text-[13.5px] text-muted">The context menu needs the first two. Everything is optional — you can skip this and grant later.</p>
              <div className="mt-5 overflow-hidden rounded-[10px] border border-line bg-panel">
                {PERMS.filter((p) => p.key !== "microphone").map((p) => {
                  const s = status?.permissions[p.key];
                  return (
                    <div key={p.key} className="flex items-center gap-3 border-b border-line px-4 py-3 last:border-b-0">
                      <Icon name={p.icon} className="text-muted" />
                      <div className="flex-1"><div className="text-[13px] font-medium">{p.title}</div><div className="text-[12px] text-muted">{p.why}</div></div>
                      {s === "granted" ? <Badge tone="ok">Granted</Badge> : s === "unsupported" ? <Badge>N/A</Badge> : (
                        <Button size="sm" onClick={async () => { try { await api.requestPermission(p.key); await api.openPrivacyPane(p.key); } catch (e) { toast(errorText(e), "error"); } refresh(); }}>Grant…</Button>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          )}
          {step === "ai" && (
            <div>
              <h2 className="text-[20px] font-semibold">Optional: use your ChatGPT plan</h2>
              <p className="mt-2 text-[13.5px] text-muted">
                Connect your ChatGPT account to rewrite, translate and explain text. You'll sign in on OpenAI's website — no API key needed. Requests use your ChatGPT plan's allowance. Skip this and everything else still works offline.
              </p>
              <div className="mt-5 flex items-center gap-3">
                {status?.ai.state === "connected" ? (
                  <Badge tone="ok">Connected{status.ai.email ? ` as ${status.ai.email}` : ""}</Badge>
                ) : (
                  <button
                    onClick={() => api.aiConnect().then(refresh).catch((e) => { toast(errorText(e), "error"); refresh(); })}
                    className="inline-flex h-9 items-center gap-2 rounded-full bg-black px-4 text-[13px] font-medium text-white hover:bg-black/85 dark:bg-white dark:text-black"
                  >
                    <Icon name="sparkle" size={13} /> Continue with ChatGPT
                  </button>
                )}
                {status?.ai.state === "connecting" && <span className="text-[12.5px] text-muted">Finish signing in in your browser…</span>}
              </div>
            </div>
          )}
          {step === "done" && (
            <div className="text-center">
              <div className="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-full bg-ok/10 text-ok"><Icon name="check" size={26} /></div>
              <h2 className="text-[20px] font-semibold">You're set</h2>
              <p className="mx-auto mt-2 max-w-[420px] text-[13.5px] text-muted">
                InputFlow lives in your menu bar. Try selecting some text and pressing Shift + Right Click, or press {prettyAccelerator(settings.shortcuts["launcher.open"] ?? "")} for the launcher.
              </p>
            </div>
          )}

          <div className="mt-8 flex items-center justify-between">
            <div className="flex gap-1.5">
              {STEPS.map((s, k) => <span key={s} className={`h-1.5 w-1.5 rounded-full ${k === i ? "bg-accent" : "bg-line"}`} />)}
            </div>
            <div className="flex gap-2">
              {i > 0 && step !== "done" && <Button variant="ghost" onClick={() => setI(i - 1)}>Back</Button>}
              {step === "done" ? (
                <Button variant="primary" onClick={() => update((s) => void (s.general.onboardingComplete = true))}>Open InputFlow</Button>
              ) : (
                <Button variant="primary" onClick={next}>{step === "welcome" ? "Get started" : step === "permissions" || step === "ai" ? "Continue" : "Next"}</Button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
