import { Circle } from "lucide-react";
import { ICONS } from "./icons.gen";
import { useEffect, useRef, useState, type ReactNode, type ButtonHTMLAttributes } from "react";

export function Icon({ name, size = 15, className = "", strokeWidth = 1.8 }: { name: string; size?: number; className?: string; strokeWidth?: number }) {
  const C = (ICONS as Record<string, typeof Circle>)[name] ?? Circle;
  return <C size={size} strokeWidth={strokeWidth} className={className} aria-hidden />;
}

type BtnVariant = "primary" | "secondary" | "ghost" | "danger";
export function Button({ variant = "secondary", size = "md", className = "", children, ...rest }: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: BtnVariant; size?: "sm" | "md" }) {
  const base = "inline-flex items-center justify-center gap-1.5 rounded-[7px] font-medium transition-colors disabled:opacity-45 disabled:pointer-events-none whitespace-nowrap";
  const sizes = { sm: "h-7 px-2.5 text-[12px]", md: "h-8 px-3 text-[13px]" };
  const variants: Record<BtnVariant, string> = {
    primary: "bg-accent text-accent-fg hover:brightness-110 shadow-sm",
    secondary: "bg-panel border border-line text-fg hover:bg-hover shadow-[0_1px_0_rgba(0,0,0,0.03)]",
    ghost: "text-fg hover:bg-hover",
    danger: "bg-panel border border-line text-bad hover:bg-bad/10",
  };
  return (
    <button className={`${base} ${sizes[size]} ${variants[variant]} ${className}`} {...rest}>
      {children}
    </button>
  );
}

export function Toggle({ checked, onChange, disabled, label }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean; label: string }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`relative inline-flex h-[22px] w-[38px] shrink-0 rounded-full transition-colors disabled:opacity-40 ${checked ? "bg-accent" : "bg-line"}`}
    >
      <span className={`absolute top-[2px] h-[18px] w-[18px] rounded-full bg-white shadow transition-transform ${checked ? "translate-x-[18px]" : "translate-x-[2px]"}`} />
    </button>
  );
}

export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: string; actions?: ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-6 pb-5">
      <div>
        <h1 className="text-[20px] font-semibold tracking-[-0.01em]">{title}</h1>
        {subtitle && <p className="mt-1 max-w-[620px] text-[13px] text-muted">{subtitle}</p>}
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </div>
  );
}

export function Section({ title, description, children, right }: { title?: string; description?: string; children: ReactNode; right?: ReactNode }) {
  return (
    <section className="mb-6">
      {(title || right) && (
        <div className="mb-2 flex items-end justify-between px-1">
          <div>
            {title && <h2 className="text-[12px] font-semibold uppercase tracking-wide text-muted">{title}</h2>}
            {description && <p className="mt-0.5 text-[12px] text-faint">{description}</p>}
          </div>
          {right}
        </div>
      )}
      <div className="overflow-hidden rounded-[10px] border border-line bg-panel">{children}</div>
    </section>
  );
}

export function Row({ title, description, children, icon }: { title: ReactNode; description?: ReactNode; children?: ReactNode; icon?: string }) {
  return (
    <div className="flex items-center gap-3 border-b border-line px-4 py-3 last:border-b-0">
      {icon && (
        <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-[7px] bg-raised text-muted">
          <Icon name={icon} size={15} />
        </div>
      )}
      <div className="min-w-0 flex-1">
        <div className="text-[13px] font-medium">{title}</div>
        {description && <div className="mt-0.5 text-[12px] leading-snug text-muted">{description}</div>}
      </div>
      {children && <div className="flex shrink-0 items-center gap-2">{children}</div>}
    </div>
  );
}

export function Badge({ tone = "neutral", children }: { tone?: "neutral" | "ok" | "warn" | "bad" | "accent"; children: ReactNode }) {
  const tones = {
    neutral: "bg-raised text-muted border-line",
    ok: "bg-ok/10 text-ok border-ok/25",
    warn: "bg-warn/10 text-warn border-warn/25",
    bad: "bg-bad/10 text-bad border-bad/25",
    accent: "bg-accent/10 text-accent border-accent/25",
  };
  return <span className={`inline-flex items-center gap-1 rounded-full border px-2 py-[1px] text-[11px] font-medium ${tones[tone]}`}>{children}</span>;
}

export function Empty({ icon, title, children }: { icon: string; title: string; children?: ReactNode }) {
  return (
    <div className="flex flex-col items-center px-6 py-10 text-center">
      <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-raised text-muted">
        <Icon name={icon} size={18} />
      </div>
      <div className="text-[13px] font-medium">{title}</div>
      {children && <div className="mt-1 max-w-[360px] text-[12px] text-muted">{children}</div>}
    </div>
  );
}

/** Small "local" vs "ChatGPT" indicator used everywhere an action is listed. */
export function OnlineDot({ ai }: { ai: boolean }) {
  return ai ? (
    <span title="Uses your connected ChatGPT account (online)" className="inline-flex items-center gap-1 text-[10.5px] font-medium text-accent">
      <Icon name="cloud" size={11} /> ChatGPT
    </span>
  ) : (
    <span title="Runs entirely on this computer" className="inline-flex items-center gap-1 text-[10.5px] font-medium text-faint">
      <Icon name="hard-drive" size={11} /> Local
    </span>
  );
}

const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);

export function prettyAccelerator(acc: string): string {
  if (!acc) return "—";
  return acc
    .split("+")
    .map((k) => {
      const l = k.toLowerCase();
      if (isMac) {
        if (["cmd", "command", "super", "meta"].includes(l)) return "⌘";
        if (["alt", "option"].includes(l)) return "⌥";
        if (["ctrl", "control"].includes(l)) return "⌃";
        if (l === "shift") return "⇧";
      }
      if (l.startsWith("key")) return k.slice(3).toUpperCase();
      if (l.startsWith("digit")) return k.slice(5);
      if (l === "escape") return "Esc";
      if (l === "space") return isMac ? "Space" : "Space";
      return k.length === 1 ? k.toUpperCase() : k;
    })
    .join(isMac ? "" : "+");
}

/** Records a key combination and produces a Tauri global-shortcut accelerator. */
export function ShortcutRecorder({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [recording, setRecording] = useState(false);
  const ref = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape" && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey) {
        setRecording(false);
        return;
      }
      if (["Meta", "Control", "Alt", "Shift"].includes(e.key)) return;
      const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && (isMac ? "Cmd" : "Super")].filter(Boolean) as string[];
      if (mods.length === 0 && !/^F\d+$/.test(e.code)) return; // require a modifier except for F-keys
      onChange([...mods, e.code].join("+"));
      setRecording(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange]);
  return (
    <div className="flex items-center gap-1">
      <button
        ref={ref}
        onClick={() => setRecording((r) => !r)}
        className={`h-7 min-w-[110px] rounded-[7px] border px-2.5 text-[12px] font-medium ${recording ? "border-accent bg-accent/10 text-accent" : "border-line bg-raised text-fg hover:bg-hover"}`}
        aria-label={recording ? "Press a key combination" : `Shortcut ${value || "not set"}; click to change`}
      >
        {recording ? "Press keys…" : prettyAccelerator(value)}
      </button>
      {value && !recording && (
        <button onClick={() => onChange("")} className="rounded p-1 text-faint hover:text-fg" aria-label="Clear shortcut">
          <Icon name="x" size={13} />
        </button>
      )}
    </div>
  );
}

export function useTheme(settings: { theme: string; accent: string; fontScale: number; reduceMotion: boolean; highContrast: boolean } | undefined) {
  useEffect(() => {
    if (!settings) return;
    const root = document.documentElement;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = settings.theme === "dark" || (settings.theme === "system" && mq.matches);
      root.dataset.theme = dark ? "dark" : "light";
    };
    apply();
    root.style.setProperty("--if-accent", settings.accent);
    root.style.setProperty("--if-font-scale", String(settings.fontScale));
    root.dataset.motion = settings.reduceMotion ? "reduce" : "full";
    root.dataset.contrast = settings.highContrast ? "high" : "normal";
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [settings?.theme, settings?.accent, settings?.fontScale, settings?.reduceMotion, settings?.highContrast]);
}
