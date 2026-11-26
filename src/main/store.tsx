import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorText, type ActionDef, type Bootstrap, type Settings, type Status } from "../lib/api";

interface Toast { id: number; message: string; kind: string }

interface Store {
  boot: Bootstrap;
  settings: Settings;
  status: Status | null;
  actions: ActionDef[];
  update: (fn: (s: Settings) => void) => Promise<void>;
  refresh: () => Promise<void>;
  toast: (message: string, kind?: "success" | "error" | "info") => void;
  toasts: Toast[];
  page: string;
  setPage: (p: string) => void;
}

const Ctx = createContext<Store | null>(null);

export function useStore() {
  const s = useContext(Ctx);
  if (!s) throw new Error("store missing");
  return s;
}

export function StoreProvider({ children }: { children: (ready: boolean) => ReactNode }) {
  const [boot, setBoot] = useState<Bootstrap | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState<Status | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [page, setPage] = useState(() => new URLSearchParams(location.search).get("page") ?? "overview");
  const tid = useRef(1);
  const [bootError, setBootError] = useState<string | null>(null);

  const toast = useCallback((message: string, kind: string = "info") => {
    const id = tid.current++;
    setToasts((t) => [...t.slice(-3), { id, message, kind }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 4200);
  }, []);

  const refresh = useCallback(async () => {
    try {
      setStatus(await api.status());
    } catch (e) {
      console.error(e);
    }
  }, []);

  useEffect(() => {
    api
      .bootstrap()
      .then((b) => {
        setBoot(b);
        setSettings(b.settings);
      })
      .catch((e) => setBootError(errorText(e)));
    const bootTimer = setTimeout(
      () => setBoot((b) => (b ? b : (setBootError("No response from the background service after 10 seconds."), null))),
      10_000,
    );
    refresh();
    const unsubs = [
      listen("status:changed", () => refresh()),
      listen<{ message: string; kind: string }>("toast", (e) => toast(e.payload.message, e.payload.kind)),
      listen<string>("navigate", (e) => setPage(e.payload)),
    ];
    const iv = setInterval(() => {
      if (document.visibilityState === "visible") refresh();
    }, 5000);
    return () => {
      clearTimeout(bootTimer);
      clearInterval(iv);
      unsubs.forEach((p) => p.then((f) => f()));
    };
  }, [refresh, toast]);

  const update = useCallback(
    async (fn: (s: Settings) => void) => {
      if (!settings) return;
      const next: Settings = structuredClone(settings);
      fn(next);
      setSettings(next); // optimistic
      try {
        const saved = await api.saveSettings(next);
        setSettings(saved);
        const b = await api.bootstrap();
        setBoot(b);
        refresh();
      } catch (e) {
        setSettings(settings);
        toast(errorText(e), "error");
      }
    },
    [settings, refresh, toast],
  );

  if (bootError) throw new Error(`MinPoint couldn't start its background service: ${bootError}`);
  if (!boot || !settings) return <>{children(false)}</>;
  return (
    <Ctx.Provider value={{ boot, settings, status, actions: boot.actions, update, refresh, toast, toasts, page, setPage }}>
      {children(true)}
    </Ctx.Provider>
  );
}
