import { useStore } from "./store";
import { Icon, useTheme } from "../components/ui";
import { Overview } from "./pages/Overview";
import { QuickActions } from "./pages/QuickActions";
import { Account } from "./pages/Account";
import { Writing } from "./pages/Writing";
import { Clipboard } from "./pages/Clipboard";
import { Expander } from "./pages/Expander";
import { ScreenOcr, Screenshots } from "./pages/Capture";
import { WindowManager } from "./pages/WindowManager";
import { Launcher } from "./pages/Launcher";
import { DevTools } from "./pages/DevTools";
import { Profiles } from "./pages/Profiles";
import { Appearance } from "./pages/Appearance";
import { PermissionsPage } from "./pages/Permissions";
import { SettingsPage, PrivacyCenter } from "./pages/Settings";
import { Planned } from "./pages/Planned";
import { Onboarding } from "./Onboarding";

interface NavItem { id: string; label: string; icon: string; planned?: boolean }
const NAV: { title?: string; items: NavItem[] }[] = [
  { items: [{ id: "overview", label: "Overview", icon: "layout-dashboard" }, { id: "actions", label: "Quick Actions", icon: "mouse-pointer-click" }] },
  {
    title: "Input",
    items: [
      { id: "keyboard", label: "Keyboard Studio", icon: "keyboard", planned: true },
      { id: "mouse", label: "Mouse Studio", icon: "mouse", planned: true },
      { id: "trackpad", label: "Trackpad Studio", icon: "hand", planned: true },
    ],
  },
  {
    title: "Write & Speak",
    items: [
      { id: "writing", label: "Writing Assistant", icon: "sparkles" },
      { id: "voice", label: "Voice Anywhere", icon: "mic", planned: true },
      { id: "clipboard", label: "Smart Clipboard", icon: "clipboard-list" },
      { id: "snippets", label: "Text Expander", icon: "text-cursor-input" },
    ],
  },
  {
    title: "Screen & Windows",
    items: [
      { id: "ocr", label: "Screen OCR", icon: "scan-text" },
      { id: "screenshots", label: "Screenshot Tools", icon: "camera" },
      { id: "windows", label: "Window Manager", icon: "layout-panel-left" },
    ],
  },
  {
    title: "Power Tools",
    items: [
      { id: "automation", label: "Automation Studio", icon: "workflow", planned: true },
      { id: "launcher", label: "Quick Launcher", icon: "command" },
      { id: "devtools", label: "Developer Tools", icon: "braces" },
      { id: "profiles", label: "Application Profiles", icon: "app-window" },
    ],
  },
  {
    title: "InputFlow",
    items: [
      { id: "account", label: "Connected Account", icon: "circle-user" },
      { id: "appearance", label: "Appearance", icon: "palette" },
      { id: "permissions", label: "Permissions", icon: "shield-check" },
      { id: "privacy", label: "Privacy Center", icon: "lock" },
      { id: "settings", label: "Settings", icon: "settings" },
    ],
  },
];

function Page({ id }: { id: string }) {
  switch (id) {
    case "overview": return <Overview />;
    case "actions": return <QuickActions />;
    case "account": return <Account />;
    case "writing": return <Writing />;
    case "clipboard": return <Clipboard />;
    case "snippets": return <Expander />;
    case "ocr": return <ScreenOcr />;
    case "screenshots": return <Screenshots />;
    case "windows": return <WindowManager />;
    case "launcher": return <Launcher />;
    case "devtools": return <DevTools />;
    case "profiles": return <Profiles />;
    case "appearance": return <Appearance />;
    case "permissions": return <PermissionsPage />;
    case "privacy": return <PrivacyCenter />;
    case "settings": return <SettingsPage />;
    case "keyboard": case "mouse": case "trackpad": case "voice": case "automation": return <Planned id={id} />;
    default: return <Overview />;
  }
}

export function MainApp() {
  const { settings, status, page, setPage, toasts, update } = useStore();
  useTheme(settings.appearance);
  if (!settings.general.onboardingComplete) return <Onboarding />;
  const paused = status?.paused ?? settings.general.paused;
  return (
    <div className="flex h-full">
      <aside className="flex w-[230px] shrink-0 flex-col border-r border-line bg-sidebar">
        <div className="drag-region h-[38px] shrink-0" />
        <div className="flex items-center gap-2 px-4 pb-3">
          <img src="/app-icon.png" alt="" className="h-7 w-7" />
          <div className="leading-tight">
            <div className="text-[14px] font-semibold">InputFlow</div>
            <div className="text-[11px] text-muted">Your Computer. Your Controls.</div>
          </div>
        </div>
        <nav className="flex-1 overflow-y-auto px-2 pb-3" aria-label="Sections">
          {NAV.map((g, gi) => (
            <div key={gi} className="mb-2">
              {g.title && <div className="px-2 pb-1 pt-2 text-[11px] font-semibold text-faint">{g.title}</div>}
              {g.items.map((it) => (
                <button
                  key={it.id}
                  onClick={() => setPage(it.id)}
                  aria-current={page === it.id ? "page" : undefined}
                  className={`flex w-full items-center gap-2.5 rounded-[7px] px-2 py-[5px] text-left text-[13px] ${page === it.id ? "bg-accent text-accent-fg" : "text-fg hover:bg-hover"}`}
                >
                  <Icon name={it.icon} size={15} className={page === it.id ? "" : "text-muted"} />
                  <span className="flex-1 truncate">{it.label}</span>
                  {it.planned && <span className={`text-[10px] ${page === it.id ? "text-accent-fg/80" : "text-faint"}`}>Planned</span>}
                </button>
              ))}
            </div>
          ))}
        </nav>
        <div className="border-t border-line p-3">
          <button
            onClick={() => update((s) => void (s.general.paused = !s.general.paused))}
            className="flex w-full items-center gap-2 rounded-[7px] px-2 py-1.5 text-[12.5px] hover:bg-hover"
          >
            <span className={`h-2 w-2 rounded-full ${paused ? "bg-warn" : status?.helperRunning ? "bg-ok" : "bg-bad"}`} />
            <span className="flex-1 text-left">{paused ? "Paused" : status?.helperRunning ? "Running" : status ? "Native service unavailable" : "Starting…"}</span>
            <span className="text-[11px] text-accent">{paused ? "Resume" : "Pause"}</span>
          </button>
        </div>
      </aside>
      <main className="flex min-w-0 flex-1 flex-col">
        <div className="drag-region h-[38px] shrink-0" />
        <div className="flex-1 overflow-y-auto px-8 pb-10">
          <div className="mx-auto max-w-[860px]">
            <Page id={page} />
          </div>
        </div>
      </main>
      <div className="pointer-events-none fixed bottom-4 right-4 z-50 flex flex-col gap-2" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className="anim-pop pointer-events-auto flex max-w-[380px] items-start gap-2 rounded-[9px] border border-line bg-panel px-3 py-2 text-[12.5px] shadow-lg">
            <Icon name={t.kind === "error" ? "circle-alert" : t.kind === "success" ? "circle-check" : "info"} size={15} className={t.kind === "error" ? "text-bad" : t.kind === "success" ? "text-ok" : "text-accent"} />
            <span>{t.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
