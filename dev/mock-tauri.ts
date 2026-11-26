// DEVELOPMENT-ONLY fake Tauri bridge for visual UI checks in a browser. Never shipped.
const q = new URLSearchParams(location.search);
if (location.hash.startsWith("#/overlay")) document.documentElement.dataset.surface = "overlay";
const settings: any = {
  version: 1,
  general: { onboardingComplete: new URLSearchParams(location.search).get("onboarding") !== "1", launchAtLogin: false, paused: false, keepRunningOnClose: true, excludedApps: [] },
  menu: { shiftRightClick: true, disabled: ["ai.formal"], favorites: ["ai.polish", "ai.grammar"], order: [], hideUnavailable: true, showGroupsAsSubmenus: true },
  appearance: { theme: new URLSearchParams(location.search).get("theme") ?? "light", accent: "#ff7a45", compact: false, fontScale: 1, reduceMotion: false, highContrast: false, menuOpacity: 0.96 },
  ai: { enabled: true, model: null, autoReplace: false, favoriteLanguages: ["English", "Spanish", "Malayalam"], defaultTargetLanguage: "English", customActions: [] },
  clipboard: { historyEnabled: true, maxItems: 200, retentionDays: 7, captureImages: false, excludedApps: ["com.1password.1password"] },
  expander: { enabled: false, excludedApps: [] },
  speech: { voice: null, rate: 1, volume: 1 },
  screenshot: { copyToClipboard: true, saveToDisk: true, saveDir: "" },
  shortcuts: { "launcher.open": "Alt+Space", "menu.open": "Ctrl+Alt+Space", "ocr.capture": "Ctrl+Alt+Shift+KeyT", "clip.history": "Ctrl+Alt+KeyV", "app.emergency_disable": "Ctrl+Alt+Shift+Escape" },
  profiles: [],
};
const actions = [
  { id: "ai.polish", name: "Polish Text", description: "Improve clarity and flow.", icon: "sparkles", group: "", contexts: ["selection"], capabilities: ["ai_connection"], permissions: [], offline: false, requiresAi: true, output: "preview", security: "external_ai" },
  { id: "ai.grammar", name: "Fix Grammar", description: "Correct grammar only.", icon: "spell-check", group: "", contexts: ["selection"], capabilities: ["ai_connection"], permissions: [], offline: false, requiresAi: true, output: "preview", security: "external_ai" },
  { id: "case.upper", name: "UPPERCASE", description: "", icon: "case-upper", group: "case", contexts: ["selection"], capabilities: [], permissions: [], offline: true, requiresAi: false, output: "preview", security: "local_text" },
  { id: "ocr.capture", name: "Screen Text Capture", description: "", icon: "scan-text", group: "", contexts: ["anywhere"], capabilities: [], permissions: [], offline: true, requiresAi: false, output: "open_surface", security: "screen_read" },
  { id: "clip.history", name: "Clipboard History", description: "", icon: "clipboard-list", group: "", contexts: ["editable"], capabilities: [], permissions: [], offline: true, requiresAi: false, output: "open_surface", security: "local_text" },
  { id: "launcher.open", name: "Quick Launcher", description: "", icon: "command", group: "", contexts: ["anywhere"], capabilities: [], permissions: [], offline: true, requiresAi: false, output: "open_surface", security: "local_text" },
];
// ?demo=1 shows a fully set-up machine (sample data, for README screenshots only).
const demo = q.get("demo") === "1";
const g = (v: string) => (demo ? "granted" : v);
const handlers: Record<string, (a: any) => any> = {
  has_undo: () => false,
  bootstrap: () => ({ settings, platform: "harness", os: "macos", version: "0.1.0", actions }),
  get_status: () => ({ paused: false, platform: "harness", helperRunning: true, inputTapRunning: demo, inputTapError: demo ? null : "Input Monitoring / Accessibility permission is required for Shift + Right Click.",
    permissions: { accessibility: g("not_granted"), inputMonitoring: g("not_granted"), screenRecording: g("not_granted"), microphone: "not_determined" },
    ai: demo
      ? { state: "connected", email: "you@example.com", name: "Sample User", planUsageAuthorized: true, usageLimited: false, error: null, models: [{ slug: "sample-model", displayName: "Default model" }], manageUsageUrl: "", showWelcome: false }
      : { state: "disconnected", email: null, name: null, planUsageAuthorized: false, usageLimited: false, error: null, models: [], manageUsageUrl: "", showWelcome: false },
    clipboardItems: demo ? 48 : 2, snippets: demo ? 3 : 0, shortcuts: Object.entries(settings.shortcuts).map(([actionId, accelerator]) => ({ actionId, accelerator, registered: true, error: null })),
    recentActions: demo ? [{ actionId: "ai.polish", name: "Polish Text", count: 12, lastUsed: 0 }, { actionId: "ai.grammar", name: "Fix Grammar", count: 7, lastUsed: 0 }, { actionId: "ocr.capture", name: "Screen Text Capture", count: 3, lastUsed: 0 }] : [],
    resources: null, activeApp: demo ? "Slack" : "Harness", activeProfile: demo ? "Slack" : null }),
  save_settings: (a) => Object.assign(settings, a.settings),
  clipboard_list: () => [{ id: 1, kind: "text", text: "Sample clipboard item", app: "com.apple.Safari", createdAt: Date.now() / 1000 - 120, pinned: true, favorite: false }],
  snippets_list: () => (demo ? [
    { id: "1", name: "PR done", trigger: ";prdone", content: "I've completed the changes and raised the pull request. Please review it when you have a chance.", category: "Work", caseSensitive: false, favorite: true, enabled: true, apps: [] },
    { id: "2", name: "Meeting link", trigger: ";meet", content: "Here's the link for our call: {clipboard}", category: "Work", caseSensitive: false, favorite: false, enabled: true, apps: [] },
    { id: "3", name: "Sign-off", trigger: ";sig", content: "Thanks,\n{username}", category: "Personal", caseSensitive: false, favorite: false, enabled: true, apps: [] },
  ] : []),
  voices: () => [],
  calculate: (a) => (a.expr.includes("km") ? "3.106856 mi" : "36"),
  dev_tool: () => '{\n  "name": "MinPoint",\n  "local": true\n}',
  text_stats: (a) => ({ characters: a.input.length, charactersNoSpaces: 0, words: a.input.split(/\s+/).filter(Boolean).length, lines: 1, sentences: 1, readingSeconds: 0 }),
};
let cb = 1;
const callbacks = new Map<number, (e: any) => void>();
// Sample overlay payloads so the floating surfaces can be inspected.
const overlaySamples: Record<string, any> = {
  menu: { mode: "menu", payload: { aiReady: true, invocation: { selectionLen: 64, editable: true, app: { name: "Slack" } }, entries: [
    { kind: "item", id: "ai.polish", name: "Polish Text", icon: "sparkles", requiresAi: true, available: true, reason: null },
    { kind: "item", id: "ai.grammar", name: "Fix Grammar", icon: "spell-check", requiresAi: true, available: true, reason: null },
    { kind: "separator" },
    { kind: "item", id: "ai.paraphrase", name: "Paraphrase", icon: "repeat", requiresAi: true, available: true, reason: null },
    { kind: "item", id: "ai.shorten", name: "Shorten", icon: "shrink", requiresAi: true, available: true, reason: null },
    { kind: "item", id: "ai.translate", name: "Translate", icon: "languages", requiresAi: true, available: true, reason: null },
    { kind: "item", id: "tts.read", name: "Read Aloud", icon: "volume-2", requiresAi: false, available: true, reason: null },
    { kind: "separator" },
    { kind: "submenu", id: "group.tone", name: "Change Tone", items: [] },
    { kind: "submenu", id: "group.case", name: "Change Text Case", items: [] },
    { kind: "submenu", id: "group.dev", name: "Developer Tools", items: [] },
    { kind: "separator" },
    { kind: "item", id: "launcher.open", name: "Quick Launcher", icon: "command", requiresAi: false, available: true, reason: null },
  ] } },
  preview: { mode: "preview", payload: { title: "Polish Text", ai: true, streaming: false, replaceable: true, app: "Slack",
    original: "hey team, i finished the changes and raised the PR, pls review when u get time",
    result: "Hi team, I've finished the changes and raised the PR. Please review it when you get a chance." } },
};
(window as any).__TAURI_INTERNALS__ = {
  transformCallback: (fn: (e: any) => void) => { const id = cb++; callbacks.set(id, fn); return id; },
  invoke: async (cmd: string, args: any) => {
    if (cmd === "plugin:event|listen") {
      const sample = overlaySamples[q.get("overlay") ?? ""];
      if (args.event === "overlay:show" && sample) setTimeout(() => callbacks.get(args.handler)?.({ event: args.event, id: 0, payload: sample }), 50);
      return cb++;
    }
    if (cmd.startsWith("plugin:event|")) return cb++;
    const h = handlers[cmd];
    if (!h) throw `harness: ${cmd} not mocked`;
    return h(args ?? {});
  },
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
};
