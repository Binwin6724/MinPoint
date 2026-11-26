// Typed wrappers around MinPoint's Rust commands. The webview can only call
// these narrow commands — there is no generic native bridge.
import { invoke } from "@tauri-apps/api/core";

export type Theme = "system" | "light" | "dark";

export interface Settings {
  version: number;
  general: { onboardingComplete: boolean; launchAtLogin: boolean; paused: boolean; keepRunningOnClose: boolean; excludedApps: string[] };
  menu: { shiftRightClick: boolean; disabled: string[]; favorites: string[]; order: string[]; hideUnavailable: boolean; showGroupsAsSubmenus: boolean };
  appearance: { theme: Theme; accent: string; compact: boolean; fontScale: number; reduceMotion: boolean; highContrast: boolean; menuOpacity: number };
  ai: { enabled: boolean; model: string | null; autoReplace: boolean; favoriteLanguages: string[]; defaultTargetLanguage: string; customActions: CustomAiAction[] };
  clipboard: { historyEnabled: boolean; maxItems: number; retentionDays: number; captureImages: boolean; excludedApps: string[] };
  expander: { enabled: boolean; excludedApps: string[] };
  speech: { voice: string | null; rate: number; volume: number };
  screenshot: { copyToClipboard: boolean; saveToDisk: boolean; saveDir: string };
  shortcuts: Record<string, string>;
  profiles: Profile[];
}

export interface CustomAiAction { id: string; name: string; description: string; icon: string; instructions: string; inMenu: boolean; apps: string[] }
export interface Profile { id: string; name: string; apps: string[]; priority: number; disabledActions: string[]; onlyActions: string[] | null; disableOverlays: boolean }

export interface ActionDef {
  id: string; name: string; description: string; icon: string; group: string;
  contexts: ("selection" | "editable" | "anywhere")[];
  capabilities: string[]; permissions: string[]; offline: boolean; requiresAi: boolean;
  output: string; security: string;
}

export interface Bootstrap { settings: Settings; platform: string; os: string; version: string; actions: ActionDef[] }

export type PermState = "granted" | "not_granted" | "denied" | "not_determined" | "unsupported";
export interface Permissions { accessibility: PermState; inputMonitoring: PermState; screenRecording: PermState; microphone: PermState }

export interface ModelInfo { slug: string; displayName: string }
export interface AiStatus {
  state: "disconnected" | "connecting" | "connected" | "error";
  email: string | null; name: string | null; planUsageAuthorized: boolean; usageLimited: boolean;
  error: string | null; models: ModelInfo[]; manageUsageUrl: string; showWelcome: boolean;
}

export interface ShortcutStatus { actionId: string; accelerator: string; registered: boolean; error: string | null }
export interface Status {
  paused: boolean; platform: string; helperRunning: boolean; inputTapRunning: boolean; inputTapError: string | null;
  permissions: Permissions; ai: AiStatus; clipboardItems: number; snippets: number; shortcuts: ShortcutStatus[];
  recentActions: { actionId: string; name: string; count: number; lastUsed: number }[];
  resources: { appMemoryMb: number; appCpuPercent: number; measuredAt: number } | null;
  activeApp: string | null; activeProfile: string | null;
}

export interface ClipItem { id: number; kind: string; text: string; app: string; createdAt: number; pinned: boolean; favorite: boolean }
export interface Snippet { id: string; name: string; trigger: string; content: string; category: string; caseSensitive: boolean; favorite: boolean; enabled: boolean; apps: string[] }
export interface Voice { id: string; name: string; language: string; quality: string; offline: boolean }
export interface LauncherResult { kind: string; id: string; title: string; subtitle: string; icon: string; score: number }
export interface LauncherResponse { answer: { kind: string; display: string; value: string } | null; results: LauncherResult[] }

export type MenuEntry =
  | { kind: "item"; id: string; name: string; icon: string; requiresAi: boolean; available: boolean; reason: string | null }
  | { kind: "submenu"; id: string; name: string; items: MenuEntry[] }
  | { kind: "separator" };

export type TextOp = "uppercase" | "lowercase" | "title_case" | "sentence_case" | "trim_whitespace" | "remove_duplicate_lines" | "sort_lines" | "sort_lines_desc" | "reverse_text" | "reverse_lines" | "remove_line_breaks" | "camel_case" | "pascal_case" | "snake_case" | "kebab_case" | "constant_case";
export type DevOp = "json_format" | "json_minify" | "json_validate" | "base64_encode" | "base64_decode" | "url_encode" | "url_decode" | "sha256" | "sha1" | "md5" | "timestamp_to_date" | "date_to_timestamp" | "color_convert" | "csv_to_markdown_table" | "xml_format";
export type WritingOp = "polish" | "paraphrase" | "fix_grammar" | "shorten" | "expand" | "simplify" | "professional" | "casual" | "friendly" | "formal" | "translate";

export const api = {
  bootstrap: () => invoke<Bootstrap>("bootstrap"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  status: () => invoke<Status>("get_status"),
  setPaused: (paused: boolean) => invoke<void>("set_paused", { paused }),
  requestPermission: (kind: keyof Permissions) => invoke<Permissions>("request_permission", { kind }),
  openPrivacyPane: (kind: keyof Permissions) => invoke<void>("open_privacy_pane", { kind }),
  restartInput: () => invoke<void>("restart_input_services"),

  overlayRun: (actionId: string, params?: unknown) => invoke<void>("overlay_run", { actionId, params }),
  overlayRunOnText: (actionId: string, text: string, params?: unknown) => invoke<void>("overlay_run_on_text", { actionId, text, params }),
  overlayAccept: (text: string) => invoke<string>("overlay_accept", { text }),
  overlayCopy: (text: string) => invoke<void>("overlay_copy", { text }),
  overlayCancel: () => invoke<void>("overlay_cancel"),
  overlayRegenerate: (customInstructions?: string, op?: WritingOp, language?: string) => invoke<void>("overlay_regenerate", { customInstructions, op, language }),
  overlayGenerate: (prompt: string) => invoke<void>("overlay_generate", { prompt }),
  overlayUndo: () => invoke<void>("overlay_undo"),
  hasUndo: () => invoke<boolean>("has_undo"),
  pasteText: (text: string) => invoke<void>("paste_text", { text }),

  aiConnect: () => invoke<AiStatus>("ai_connect"),
  aiCancelConnect: () => invoke<void>("ai_cancel_connect"),
  aiDisconnect: () => invoke<boolean>("ai_disconnect"),
  aiStatus: () => invoke<AiStatus>("ai_status"),
  aiModels: () => invoke<ModelInfo[]>("ai_models"),
  aiDismissWelcome: () => invoke<void>("ai_dismiss_welcome"),
  openExternal: (which: "manage_usage" | "openai_help" | "chatgpt_connected_apps") => invoke<void>("open_external", { which }),

  clipboardList: (query: string) => invoke<ClipItem[]>("clipboard_list", { query }),
  clipboardPaste: (id: number) => invoke<void>("clipboard_paste", { id }),
  clipboardCopy: (id: number) => invoke<void>("clipboard_copy", { id }),
  clipboardFlag: (id: number, flag: "pinned" | "favorite", value: boolean) => invoke<void>("clipboard_flag", { id, flag, value }),
  clipboardDelete: (id: number) => invoke<void>("clipboard_delete", { id }),
  clipboardClear: (keepPinned: boolean) => invoke<void>("clipboard_clear", { keepPinned }),

  snippets: () => invoke<Snippet[]>("snippets_list"),
  snippetSave: (snippet: Snippet) => invoke<Snippet>("snippet_save", { snippet }),
  snippetDelete: (id: string) => invoke<void>("snippet_delete", { id }),
  snippetInsert: (id: string) => invoke<void>("snippet_insert", { id }),
  snippetsExport: () => invoke<string>("snippets_export"),
  snippetsImport: (json: string) => invoke<number>("snippets_import", { json }),

  launcherSearch: (query: string) => invoke<LauncherResponse>("launcher_search", { query }),
  launcherExecute: (kind: string, id: string) => invoke<void>("launcher_execute", { kind, id }),

  textTool: (op: TextOp, input: string) => invoke<string>("text_tool", { op, input }),
  textStats: (input: string) => invoke<{ characters: number; charactersNoSpaces: number; words: number; lines: number; sentences: number; readingSeconds: number }>("text_stats", { input }),
  findReplace: (input: string, find: string, replace: string, caseSensitive: boolean, regex: boolean) => invoke<string>("text_find_replace", { input, find, replace, caseSensitive, regex }),
  devTool: (op: DevOp, input: string) => invoke<string>("dev_tool", { op, input }),
  devRegex: (pattern: string, haystack: string, caseInsensitive: boolean) => invoke<{ start: number; end: number; text: string; groups: (string | null)[] }[]>("dev_regex", { pattern, haystack, caseInsensitive }),
  devUuid: () => invoke<string>("dev_uuid"),
  devDateDiff: (a: string, b: string) => invoke<number>("dev_date_diff", { a, b }),
  calculate: (expr: string) => invoke<string>("calculate", { expr }),

  voices: () => invoke<Voice[]>("voices"),
  speak: (text: string, voice?: string | null, rate?: number) => invoke<void>("speak", { text, voice, rate }),
  speechControl: (op: "stop" | "pause" | "resume") => invoke<void>("speech_control", { op }),
  ocrCapture: () => invoke<void>("ocr_capture"),
  screenshot: (mode: "area" | "window" | "full") => invoke<string>("screenshot", { mode }),
  windowAction: (action: string) => invoke<void>("window_action", { action }),

  clearActivity: () => invoke<void>("privacy_clear_activity"),
  deleteAllData: () => invoke<void>("privacy_delete_all"),
  exportConfig: () => invoke<string>("export_config"),
  importConfig: (json: string) => invoke<void>("import_config", { json }),
  setLaunchAtLogin: (enabled: boolean) => invoke<boolean>("set_launch_at_login", { enabled }),
  shortcutConflicts: (settings: Settings) => invoke<[string, string][]>("shortcut_conflicts", { settings }),
  hideMain: () => invoke<void>("hide_main"),
  logFrontend: (level: "error" | "warn" | "info", message: string) => invoke<void>("log_frontend", { level, message }).catch(() => {}),
};

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
  return "Something went wrong.";
}
