//! Typed IPC commands exposed to the webview. Each command is narrow and
//! validated: the frontend can never issue arbitrary native calls, and no
//! command ever returns tokens or credentials.

use crate::ai::{AiStatus, ModelInfo, MANAGE_USAGE_URL};
use crate::db::ClipItem;
use crate::platform::{InstalledApp, Permissions, Voice};
use crate::runtime::Rt;
use crate::shortcuts::{ShortcutRegistry, ShortcutStatus};
use minpoint_core::actions::ActionDef;
use minpoint_core::devtools::{self, DevOp};
use minpoint_core::launcher::{fuzzy_score, instant_answer, InstantAnswer};
use minpoint_core::settings::Settings;
use minpoint_core::snippets::Snippet;
use minpoint_core::text::{self, TextOp, TextStats};
use minpoint_core::writing::WritingOp;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt;

type R<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(rt: &Rt, f: impl FnOnce(Rt) -> R<T> + Send + 'static) -> R<T> {
    let rt = rt.clone();
    tauri::async_runtime::spawn_blocking(move || f(rt)).await.map_err(|e| e.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    settings: Settings,
    platform: &'static str,
    os: &'static str,
    version: &'static str,
    actions: Vec<ActionDef>,
}

#[tauri::command]
pub fn bootstrap(rt: State<'_, Rt>) -> Bootstrap {
    Bootstrap {
        settings: rt.settings(),
        platform: rt.platform.name(),
        os: std::env::consts::OS,
        version: env!("CARGO_PKG_VERSION"),
        actions: rt.actions.read().unwrap().clone(),
    }
}

#[tauri::command]
pub async fn save_settings(rt: State<'_, Rt>, settings: Settings) -> R<Settings> {
    rt.save_settings(settings)?;
    blocking(&rt, |rt| {
        rt.apply_native_settings();
        Ok(rt.settings())
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    paused: bool,
    platform: &'static str,
    helper_running: bool,
    input_tap_running: bool,
    input_tap_error: Option<String>,
    permissions: Permissions,
    ai: AiStatus,
    clipboard_items: i64,
    snippets: usize,
    shortcuts: Vec<ShortcutStatus>,
    recent_actions: Vec<RecentAction>,
    resources: Option<Resources>,
    active_app: Option<String>,
    active_profile: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentAction {
    action_id: String,
    name: String,
    count: i64,
    last_used: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resources {
    app_memory_mb: f64,
    app_cpu_percent: f64,
    measured_at: i64,
}

#[tauri::command]
pub async fn get_status(rt: State<'_, Rt>, app: AppHandle) -> R<Status> {
    let shortcuts = app.state::<ShortcutRegistry>().0.lock().unwrap().clone();
    blocking(&rt, move |rt| {
        let s = rt.settings();
        let permissions = rt.platform.permissions().unwrap_or_default();
        let names: std::collections::HashMap<String, String> = rt.actions.read().unwrap().iter().map(|a| (a.id.clone(), a.name.clone())).collect();
        let recent_actions = rt
            .db
            .usage(8)
            .into_iter()
            .map(|(id, count, last_used)| RecentAction { name: names.get(&id).cloned().unwrap_or_else(|| id.clone()), action_id: id, count, last_used })
            .collect();
        let resources = crate::util::process_usage(std::process::id()).map(|(m, c)| Resources { app_memory_mb: (m * 10.0).round() / 10.0, app_cpu_percent: c, measured_at: chrono::Utc::now().timestamp() });
        let front = rt.platform.frontmost_app().ok();
        let active_profile = front.as_ref().and_then(|a| s.profile_for(&a.bundle_id)).map(|p| p.name.clone());
        Ok(Status {
            paused: s.general.paused,
            platform: rt.platform.name(),
            helper_running: *rt.helper_alive.lock().unwrap() && rt.platform.name() != "unsupported",
            input_tap_running: *rt.tap_running.lock().unwrap(),
            input_tap_error: rt.tap_error.lock().unwrap().clone(),
            permissions,
            ai: rt.ai.status(),
            clipboard_items: rt.db.clip_count(),
            snippets: rt.db.snippets().map(|v| v.len()).unwrap_or(0),
            shortcuts,
            recent_actions,
            resources,
            active_app: front.map(|a| a.name),
            active_profile,
        })
    })
    .await
}

#[tauri::command]
pub async fn set_paused(rt: State<'_, Rt>, paused: bool) -> R<()> {
    blocking(&rt, move |rt| {
        rt.set_paused(paused);
        Ok(())
    })
    .await
}

// ---------- permissions ----------

fn check_perm_kind(kind: &str) -> R<()> {
    match kind {
        "accessibility" | "inputMonitoring" | "screenRecording" | "microphone" => Ok(()),
        _ => Err("unknown permission".into()),
    }
}

#[tauri::command]
pub async fn request_permission(rt: State<'_, Rt>, kind: String) -> R<Permissions> {
    check_perm_kind(&kind)?;
    blocking(&rt, move |rt| {
        let p = rt.platform.request_permission(&kind)?;
        rt.apply_native_settings();
        Ok(p)
    })
    .await
}

#[tauri::command]
pub async fn open_privacy_pane(rt: State<'_, Rt>, kind: String) -> R<()> {
    check_perm_kind(&kind)?;
    blocking(&rt, move |rt| rt.platform.open_privacy_pane(&kind)).await
}

#[tauri::command]
pub async fn restart_input_services(rt: State<'_, Rt>) -> R<()> {
    blocking(&rt, |rt| {
        rt.apply_native_settings();
        Ok(())
    })
    .await
}

// ---------- overlay ----------

#[tauri::command]
pub async fn overlay_run(rt: State<'_, Rt>, action_id: String, params: Option<Value>) -> R<()> {
    blocking(&rt, move |rt| rt.run_action(&action_id, params.unwrap_or(Value::Null))).await
}

#[tauri::command]
pub async fn overlay_run_on_text(rt: State<'_, Rt>, action_id: String, text: String, params: Option<Value>) -> R<()> {
    if !(action_id.starts_with("ai.") || action_id.starts_with("tts.") || action_id.starts_with("custom.")) {
        return Err("not allowed".into());
    }
    blocking(&rt, move |rt| rt.run_on_text(&action_id, text, params.unwrap_or(Value::Null))).await
}

#[tauri::command]
pub async fn overlay_accept(rt: State<'_, Rt>, text: String) -> R<String> {
    blocking(&rt, move |rt| rt.accept(&text)).await
}

#[tauri::command]
pub async fn overlay_copy(rt: State<'_, Rt>, text: String) -> R<()> {
    blocking(&rt, move |rt| {
        rt.platform.clipboard_write(&text)?;
        rt.hide_overlay();
        rt.toast("Copied to clipboard", "success");
        Ok(())
    })
    .await
}

#[tauri::command]
pub fn overlay_cancel(rt: State<'_, Rt>) {
    rt.cancel_ai();
    rt.hide_overlay();
}

#[tauri::command]
pub async fn overlay_regenerate(rt: State<'_, Rt>, custom_instructions: Option<String>, op: Option<WritingOp>, language: Option<String>) -> R<()> {
    blocking(&rt, move |rt| rt.regenerate(custom_instructions, op, language)).await
}

#[tauri::command]
pub async fn overlay_generate(rt: State<'_, Rt>, prompt: String) -> R<()> {
    blocking(&rt, move |rt| rt.generate(prompt)).await
}

#[tauri::command]
pub async fn overlay_undo(rt: State<'_, Rt>) -> R<()> {
    blocking(&rt, |rt| rt.undo_last()).await
}

#[tauri::command]
pub fn has_undo(rt: State<'_, Rt>) -> bool {
    rt.last_replacement.lock().unwrap().is_some()
}

#[tauri::command]
pub async fn paste_text(rt: State<'_, Rt>, text: String) -> R<()> {
    blocking(&rt, move |rt| rt.paste_into_target(&text)).await
}

// ---------- AI ----------

#[tauri::command]
pub async fn ai_connect(rt: State<'_, Rt>, app: AppHandle) -> R<AiStatus> {
    let rt = rt.inner().clone();
    let opener = app.clone();
    let r = rt
        .ai
        .connect(move |url| opener.opener().open_url(url, None::<&str>).map_err(|e| format!("Could not open the browser: {e}")))
        .await;
    let _ = tauri::Emitter::emit(&app, "status:changed", ());
    r
}

#[tauri::command]
pub fn ai_cancel_connect(rt: State<'_, Rt>) {
    rt.ai.cancel_connect();
}

#[tauri::command]
pub async fn ai_disconnect(rt: State<'_, Rt>) -> R<bool> {
    rt.inner().clone().ai.disconnect().await
}

#[tauri::command]
pub fn ai_status(rt: State<'_, Rt>) -> AiStatus {
    rt.ai.status()
}

#[tauri::command]
pub async fn ai_models(rt: State<'_, Rt>) -> R<Vec<ModelInfo>> {
    rt.inner().clone().ai.refresh_models().await.map_err(|e| e.message)
}

#[tauri::command]
pub fn ai_dismiss_welcome(rt: State<'_, Rt>) {
    rt.ai.dismiss_welcome();
}

/// Only a fixed allow-list of external URLs can be opened from the UI.
#[tauri::command]
pub fn open_external(app: AppHandle, which: String) -> R<()> {
    let url = match which.as_str() {
        "manage_usage" => MANAGE_USAGE_URL,
        "openai_help" => "https://help.openai.com",
        "chatgpt_connected_apps" => "https://chatgpt.com/#settings/ConnectedApps",
        _ => return Err("not allowed".into()),
    };
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

// ---------- clipboard ----------

#[tauri::command]
pub fn clipboard_list(rt: State<'_, Rt>, query: String) -> R<Vec<ClipItem>> {
    rt.db.clip_list(&query, 300)
}

#[tauri::command]
pub async fn clipboard_paste(rt: State<'_, Rt>, id: i64) -> R<()> {
    blocking(&rt, move |rt| {
        let t = rt.db.clip_get(id)?.ok_or("Item not found")?;
        rt.paste_into_target(&t)
    })
    .await
}

#[tauri::command]
pub async fn clipboard_copy(rt: State<'_, Rt>, id: i64) -> R<()> {
    blocking(&rt, move |rt| {
        let t = rt.db.clip_get(id)?.ok_or("Item not found")?;
        rt.platform.clipboard_write(&t)
    })
    .await
}

#[tauri::command]
pub fn clipboard_flag(rt: State<'_, Rt>, id: i64, flag: String, value: bool) -> R<()> {
    rt.db.clip_set_flag(id, &flag, value)
}

#[tauri::command]
pub fn clipboard_delete(rt: State<'_, Rt>, id: i64) -> R<()> {
    rt.db.clip_delete(id)
}

#[tauri::command]
pub fn clipboard_clear(rt: State<'_, Rt>, keep_pinned: bool) -> R<()> {
    rt.db.clip_clear(keep_pinned)
}

// ---------- snippets ----------

#[tauri::command]
pub fn snippets_list(rt: State<'_, Rt>) -> R<Vec<Snippet>> {
    rt.db.snippets()
}

fn validate_snippet(s: &mut Snippet) -> R<()> {
    s.name = s.name.trim().to_string();
    s.trigger = s.trigger.trim().to_string();
    if s.name.is_empty() {
        return Err("Give the snippet a name.".into());
    }
    if s.trigger.chars().any(char::is_whitespace) {
        return Err("Triggers can't contain spaces.".into());
    }
    if !s.trigger.is_empty() && s.trigger.chars().count() < 2 {
        return Err("Triggers need at least 2 characters.".into());
    }
    if s.content.len() > 100_000 {
        return Err("Snippet is too long.".into());
    }
    if s.id.is_empty() {
        s.id = uuid::Uuid::new_v4().to_string();
    }
    Ok(())
}

#[tauri::command]
pub fn snippet_save(rt: State<'_, Rt>, mut snippet: Snippet) -> R<Snippet> {
    validate_snippet(&mut snippet)?;
    let existing = rt.db.snippets()?;
    if !snippet.trigger.is_empty() {
        if let Some(dup) = existing.iter().find(|x| x.id != snippet.id && x.trigger.eq_ignore_ascii_case(&snippet.trigger)) {
            return Err(format!("The trigger {} is already used by “{}”.", snippet.trigger, dup.name));
        }
    }
    rt.db.snippet_upsert(&snippet)?;
    rt.trigger_buffer.lock().unwrap().reload(&rt.db.snippets()?);
    Ok(snippet)
}

#[tauri::command]
pub fn snippet_delete(rt: State<'_, Rt>, id: String) -> R<()> {
    rt.db.snippet_delete(&id)?;
    rt.trigger_buffer.lock().unwrap().reload(&rt.db.snippets()?);
    Ok(())
}

#[tauri::command]
pub async fn snippet_insert(rt: State<'_, Rt>, id: String) -> R<()> {
    blocking(&rt, move |rt| {
        let s = rt.db.snippets()?.into_iter().find(|s| s.id == id).ok_or("Snippet not found")?;
        let text = rt.expand_snippet_text(&s.content);
        rt.db.record_usage("snippet.insert");
        rt.paste_into_target(&text)
    })
    .await
}

#[tauri::command]
pub fn snippets_export(rt: State<'_, Rt>) -> R<String> {
    serde_json::to_string_pretty(&serde_json::json!({ "format": "minpoint-snippets", "version": 1, "snippets": rt.db.snippets()? })).map_err(|e| e.to_string())
}

/// Import is data-only: snippets are plain text and never execute anything.
#[tauri::command]
pub fn snippets_import(rt: State<'_, Rt>, json: String) -> R<usize> {
    let v: Value = serde_json::from_str(&json).map_err(|e| format!("Invalid file: {e}"))?;
    let arr = v.get("snippets").cloned().unwrap_or(v);
    let list: Vec<Snippet> = serde_json::from_value(arr).map_err(|e| format!("Invalid snippets: {e}"))?;
    let mut n = 0;
    for mut s in list.into_iter().take(5000) {
        if validate_snippet(&mut s).is_ok() {
            rt.db.snippet_upsert(&s)?;
            n += 1;
        }
    }
    rt.trigger_buffer.lock().unwrap().reload(&rt.db.snippets()?);
    Ok(n)
}

// ---------- launcher ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherResult {
    kind: &'static str,
    id: String,
    title: String,
    subtitle: String,
    icon: String,
    score: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherResponse {
    answer: Option<InstantAnswer>,
    results: Vec<LauncherResult>,
}

static APPS: std::sync::OnceLock<std::sync::Mutex<(std::time::Instant, Vec<InstalledApp>)>> = std::sync::OnceLock::new();

fn installed_apps(rt: &Rt) -> Vec<InstalledApp> {
    let cell = APPS.get_or_init(|| std::sync::Mutex::new((std::time::Instant::now(), rt.platform.installed_apps())));
    let mut g = cell.lock().unwrap();
    if g.0.elapsed().as_secs() > 300 {
        *g = (std::time::Instant::now(), rt.platform.installed_apps());
    }
    g.1.clone()
}

#[tauri::command]
pub async fn launcher_search(rt: State<'_, Rt>, query: String) -> R<LauncherResponse> {
    blocking(&rt, move |rt| {
        let q = query.trim().to_string();
        let mut results = vec![];
        let usage: std::collections::HashMap<String, i64> = rt.db.usage(50).into_iter().map(|(id, c, _)| (id, c)).collect();
        let caps = rt.capabilities();
        for a in rt.actions.read().unwrap().iter() {
            // Selection-only actions need a selection; they're reachable from the context menu.
            let needs_sel = a.capabilities.contains(&minpoint_core::actions::Capability::ReadSelection) && a.contexts == [minpoint_core::actions::ActionContext::Selection];
            if needs_sel || a.capabilities.iter().any(|c| !caps.contains(c)) || a.contexts.is_empty() && !a.id.starts_with("custom.") {
                continue;
            }
            if let Some(s) = fuzzy_score(&q, &a.name) {
                results.push(LauncherResult { kind: "action", id: a.id.clone(), title: a.name.clone(), subtitle: if a.requires_ai { "ChatGPT".into() } else { "MinPoint action".into() }, icon: a.icon.clone(), score: s + *usage.get(&a.id).unwrap_or(&0) as i32 * 3 });
            }
        }
        if !q.is_empty() {
            for app in installed_apps(&rt) {
                if let Some(s) = fuzzy_score(&q, &app.name) {
                    results.push(LauncherResult { kind: "app", id: app.path.clone(), title: app.name.clone(), subtitle: "Application".into(), icon: "app-window".into(), score: s + 5 });
                }
            }
            for sn in rt.db.snippets().unwrap_or_default() {
                let hay = format!("{} {}", sn.name, sn.trigger);
                if let Some(s) = fuzzy_score(&q, &hay) {
                    results.push(LauncherResult { kind: "snippet", id: sn.id.clone(), title: sn.name.clone(), subtitle: if sn.trigger.is_empty() { "Snippet".into() } else { format!("Snippet · {}", sn.trigger) }, icon: "text-cursor-input".into(), score: s });
                }
            }
            for (id, title) in [("settings", "MinPoint Settings"), ("snippets", "Text Expander"), ("clipboard", "Smart Clipboard"), ("account", "Connected Account"), ("permissions", "Permissions"), ("privacy", "Privacy Center")] {
                if let Some(s) = fuzzy_score(&q, title) {
                    results.push(LauncherResult { kind: "page", id: id.into(), title: title.into(), subtitle: "Open in MinPoint".into(), icon: "settings".into(), score: s - 5 });
                }
            }
            let paused = rt.settings().general.paused;
            if let Some(s) = fuzzy_score(&q, if paused { "Resume MinPoint" } else { "Pause MinPoint" }) {
                results.push(LauncherResult { kind: "toggle", id: "pause".into(), title: if paused { "Resume MinPoint".into() } else { "Pause MinPoint".into() }, subtitle: "Service".into(), icon: "power".into(), score: s });
            }
        }
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(40);
        Ok(LauncherResponse { answer: instant_answer(&q), results })
    })
    .await
}

#[tauri::command]
pub async fn launcher_execute(rt: State<'_, Rt>, kind: String, id: String) -> R<()> {
    blocking(&rt, move |rt| match kind.as_str() {
        "app" => {
            if !installed_apps(&rt).iter().any(|a| a.path == id) {
                return Err("Unknown application".into());
            }
            rt.db.record_usage("launcher.app");
            rt.hide_overlay();
            rt.platform.launch_app(&id)
        }
        "action" => rt.run_action(&id, Value::Null),
        "snippet" => {
            let s = rt.db.snippets()?.into_iter().find(|s| s.id == id).ok_or("Snippet not found")?;
            let text = rt.expand_snippet_text(&s.content);
            rt.paste_into_target(&text)
        }
        "page" => {
            rt.hide_overlay();
            crate::show_main(&rt.app, Some(&id));
            Ok(())
        }
        "toggle" => {
            rt.hide_overlay();
            let p = !rt.settings().general.paused;
            rt.set_paused(p);
            Ok(())
        }
        "answer" => {
            rt.platform.clipboard_write(&id)?;
            rt.hide_overlay();
            rt.toast("Copied result", "success");
            Ok(())
        }
        _ => Err("unknown result type".into()),
    })
    .await
}

// ---------- local tools ----------

#[tauri::command]
pub fn text_tool(op: TextOp, input: String) -> String {
    text::apply(op, &input)
}

#[tauri::command]
pub fn text_stats(input: String) -> TextStats {
    text::stats(&input)
}

#[tauri::command]
pub fn text_find_replace(input: String, find: String, replace: String, case_sensitive: bool, regex: bool) -> R<String> {
    text::find_replace(&input, &find, &replace, case_sensitive, regex)
}

#[tauri::command]
pub fn dev_tool(op: DevOp, input: String) -> R<String> {
    devtools::apply(op, &input)
}

#[tauri::command]
pub fn dev_regex(pattern: String, haystack: String, case_insensitive: bool) -> R<Vec<devtools::RegexMatch>> {
    devtools::regex_test(&pattern, &haystack, case_insensitive)
}

#[tauri::command]
pub fn dev_uuid() -> String {
    devtools::uuid_v4()
}

#[tauri::command]
pub fn dev_date_diff(a: String, b: String) -> R<i64> {
    devtools::date_difference(&a, &b)
}

#[tauri::command]
pub fn calculate(expr: String) -> R<String> {
    if let Some(c) = minpoint_core::units::parse_and_convert(&expr) {
        return Ok(format!("{} {}", minpoint_core::calc::format_number((c.result * 1e6).round() / 1e6), c.to));
    }
    minpoint_core::calc::evaluate(&expr).map(minpoint_core::calc::format_number).map_err(|e| e.to_string())
}

// ---------- speech / OCR / screenshots / windows ----------

#[tauri::command]
pub async fn voices(rt: State<'_, Rt>) -> R<Vec<Voice>> {
    blocking(&rt, |rt| rt.platform.voices()).await
}

#[tauri::command]
pub async fn speak(rt: State<'_, Rt>, text: String, voice: Option<String>, rate: Option<f32>) -> R<()> {
    blocking(&rt, move |rt| {
        let s = rt.settings().speech;
        rt.platform.speak(&text, voice.as_deref().or(s.voice.as_deref()), rate.unwrap_or(s.rate), s.volume)
    })
    .await
}

#[tauri::command]
pub async fn speech_control(rt: State<'_, Rt>, op: String) -> R<()> {
    blocking(&rt, move |rt| rt.platform.speech_control(&op)).await
}

#[tauri::command]
pub async fn ocr_capture(rt: State<'_, Rt>) -> R<()> {
    blocking(&rt, |rt| {
        if let Some(w) = rt.app.get_webview_window("main") {
            let _ = w.hide();
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        rt.ocr_flow();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn screenshot(rt: State<'_, Rt>, mode: String) -> R<String> {
    if !["area", "window", "full"].contains(&mode.as_str()) {
        return Err("invalid mode".into());
    }
    blocking(&rt, move |rt| {
        if let Some(w) = rt.app.get_webview_window("main") {
            let _ = w.hide();
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
        let r = rt.screenshot_flow(&mode);
        crate::show_main(&rt.app, None);
        r
    })
    .await
}

#[tauri::command]
pub async fn window_action(rt: State<'_, Rt>, action: String) -> R<()> {
    blocking(&rt, move |rt| rt.platform.window_action(&action)).await
}

// ---------- privacy ----------

#[tauri::command]
pub fn privacy_clear_activity(rt: State<'_, Rt>) -> R<()> {
    rt.db.clear_usage()
}

#[tauri::command]
pub async fn privacy_delete_all(rt: State<'_, Rt>) -> R<()> {
    let rt = rt.inner().clone();
    let _ = rt.ai.disconnect().await;
    rt.db.clip_clear(false)?;
    rt.db.clear_usage()?;
    for s in rt.db.snippets()? {
        rt.db.snippet_delete(&s.id)?;
    }
    rt.save_settings(Settings::default())?;
    let rt2 = rt.clone();
    tauri::async_runtime::spawn_blocking(move || rt2.apply_native_settings()).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    format: String,
    version: u32,
    settings: Settings,
    snippets: Vec<Snippet>,
}

#[tauri::command]
pub fn export_config(rt: State<'_, Rt>) -> R<String> {
    serde_json::to_string_pretty(&ExportBundle { format: "minpoint-config".into(), version: 1, settings: rt.settings(), snippets: rt.db.snippets()? }).map_err(|e| e.to_string())
}

/// Importing configuration never enables privileged features silently:
/// clipboard history, text expansion and AI auto-replace keep their current values.
#[tauri::command]
pub async fn import_config(rt: State<'_, Rt>, json: String) -> R<()> {
    let b: ExportBundle = serde_json::from_str(&json).map_err(|e| format!("Invalid configuration file: {e}"))?;
    if b.format != "minpoint-config" {
        return Err("This isn't an MinPoint configuration file.".into());
    }
    let cur = rt.settings();
    let mut s = Settings::from_json(&serde_json::to_string(&b.settings).unwrap()).map_err(|e| e.to_string())?;
    s.clipboard.history_enabled = cur.clipboard.history_enabled;
    s.expander.enabled = cur.expander.enabled;
    s.ai.auto_replace = cur.ai.auto_replace;
    s.general.onboarding_complete = cur.general.onboarding_complete;
    rt.save_settings(s)?;
    for mut sn in b.snippets {
        if validate_snippet(&mut sn).is_ok() {
            rt.db.snippet_upsert(&sn)?;
        }
    }
    blocking(&rt, |rt| {
        rt.apply_native_settings();
        Ok(())
    })
    .await
}

#[tauri::command]
pub fn set_launch_at_login(app: AppHandle, rt: State<'_, Rt>, enabled: bool) -> R<bool> {
    let al = app.autolaunch();
    if enabled { al.enable() } else { al.disable() }.map_err(|e| e.to_string())?;
    let mut s = rt.settings();
    s.general.launch_at_login = enabled;
    rt.save_settings(s)?;
    al.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn shortcut_conflicts(settings: Settings) -> Vec<(String, String)> {
    crate::shortcuts::conflicts(&settings)
}

#[tauri::command]
pub fn hide_main(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}
