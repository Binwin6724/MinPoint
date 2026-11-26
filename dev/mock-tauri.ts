// DEVELOPMENT-ONLY fake Tauri bridge for visual UI checks in a browser. Never shipped.
const settings: any = {
  version: 1,
  general: { onboardingComplete: new URLSearchParams(location.search).get("onboarding") !== "1", launchAtLogin: false, paused: false, keepRunningOnClose: true, excludedApps: [] },
  menu: { shiftRightClick: true, disabled: ["ai.formal"], favorites: ["ai.polish", "ai.grammar"], order: [], hideUnavailable: true, showGroupsAsSubmenus: true },
  appearance: { theme: new URLSearchParams(location.search).get("theme") ?? "light", accent: "#5b6cff", compact: false, fontScale: 1, reduceMotion: false, highContrast: false, menuOpacity: 0.96 },
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
  { id: "launcher.open", name: "Quick Launcher", description: "", icon: "command", group: "", contexts: ["anywhere"], capabilities: [], permissions: [], offline: true, requiresAi: false, output: "open_surface", security: "local_text" },
];
const handlers: Record<string, (a: any) => any> = {
  bootstrap: () => ({ settings, platform: "harness", os: "macos", version: "0.1.0", actions }),
  get_status: () => ({ paused: false, platform: "harness", helperRunning: true, inputTapRunning: false, inputTapError: "Input Monitoring / Accessibility permission is required for Shift + Right Click.",
    permissions: { accessibility: "not_granted", inputMonitoring: "not_granted", screenRecording: "not_granted", microphone: "not_determined" },
    ai: { state: "disconnected", email: null, name: null, planUsageAuthorized: false, usageLimited: false, error: null, models: [], manageUsageUrl: "", showWelcome: false },
    clipboardItems: 2, snippets: 0, shortcuts: Object.entries(settings.shortcuts).map(([actionId, accelerator]) => ({ actionId, accelerator, registered: true, error: null })),
    recentActions: [], resources: { appMemoryMb: 0, appCpuPercent: 0, measuredAt: 0 }, activeApp: "Harness", activeProfile: null }),
  save_settings: (a) => Object.assign(settings, a.settings),
  clipboard_list: () => [{ id: 1, kind: "text", text: "Sample clipboard item", app: "com.apple.Safari", createdAt: Date.now() / 1000 - 120, pinned: true, favorite: false }],
  snippets_list: () => [],
  voices: () => [],
  text_stats: (a) => ({ characters: a.input.length, charactersNoSpaces: 0, words: a.input.split(/\s+/).filter(Boolean).length, lines: 1, sentences: 1, readingSeconds: 0 }),
};
let cb = 1;
(window as any).__TAURI_INTERNALS__ = {
  transformCallback: () => cb++,
  invoke: async (cmd: string, args: any) => {
    if (cmd.startsWith("plugin:event|")) return cb++;
    const h = handlers[cmd];
    if (!h) throw `harness: ${cmd} not mocked`;
    return h(args ?? {});
  },
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
};
