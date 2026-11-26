//! Background runtime: owns state, reacts to native events, drives the overlay
//! surfaces (context menu, preview, palette, clipboard picker, OCR result) and
//! executes actions from the registry.

use crate::ai::{AiError, AiService};
use crate::db::Db;
use crate::platform::{AppInfo, PlatformAdapter};
use minpoint_core::actions::{builtin_actions, ActionDef, ActionKind, Builtin, Capability};
use minpoint_core::menu::{self, MenuContext, MenuEntry};
use minpoint_core::settings::Settings;
use minpoint_core::snippets::{self, TriggerBuffer, VariableSource};
use minpoint_core::writing::{self, WritingOp, WritingOptions};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewWindow};

/// Context captured at the moment the user invoked MinPoint (before our UI took focus).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invocation {
    pub id: u64,
    pub app: AppInfo,
    #[serde(skip)]
    pub selection: String,
    pub selection_len: usize,
    pub selection_source: String,
    pub editable: bool,
    pub secure: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastReplacement {
    pub pid: i64,
    pub app_name: String,
}

pub struct Runtime {
    pub app: AppHandle,
    pub platform: Arc<dyn PlatformAdapter>,
    pub settings: RwLock<Settings>,
    pub settings_path: PathBuf,
    pub db: Db,
    pub ai: AiService,
    pub actions: RwLock<Vec<ActionDef>>,
    pub invocation: Mutex<Option<Invocation>>,
    pub last_replacement: Mutex<Option<LastReplacement>>,
    pub trigger_buffer: Mutex<TriggerBuffer>,
    pub current_app: Mutex<String>,
    pub tap_running: Mutex<bool>,
    pub tap_error: Mutex<Option<String>>,
    pub helper_alive: Mutex<bool>,
    ai_cancel: Mutex<Option<tokio::sync::watch::Sender<bool>>>,
    next_id: AtomicU64,
    pub last_preview: Mutex<Option<PreviewRequest>>,
}

#[derive(Debug, Clone)]
pub struct PreviewRequest {
    pub action_id: String,
    pub op: WritingOp,
    pub options: WritingOptions,
}

pub type Rt = Arc<Runtime>;

impl Runtime {
    pub fn new(app: AppHandle, platform: Arc<dyn PlatformAdapter>, settings: Settings, settings_path: PathBuf, db: Db, ai: AiService) -> Rt {
        let snippets = db.snippets().unwrap_or_default();
        let mut actions = builtin_actions();
        add_custom_actions(&mut actions, &settings);
        Arc::new(Runtime {
            app,
            platform,
            settings: RwLock::new(settings),
            settings_path,
            db,
            ai,
            actions: RwLock::new(actions),
            invocation: Mutex::new(None),
            last_replacement: Mutex::new(None),
            trigger_buffer: Mutex::new(TriggerBuffer::new(&snippets)),
            current_app: Mutex::new(String::new()),
            tap_running: Mutex::new(false),
            tap_error: Mutex::new(None),
            helper_alive: Mutex::new(true),
            ai_cancel: Mutex::new(None),
            next_id: AtomicU64::new(1),
            last_preview: Mutex::new(None),
        })
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    pub fn capabilities(&self) -> Vec<Capability> {
        let mut c = self.platform.capabilities();
        let s = self.settings.read().unwrap();
        if !s.clipboard.history_enabled {
            c.retain(|x| *x != Capability::ClipboardHistory);
        }
        if s.ai.enabled && self.ai.is_ready() {
            c.push(Capability::AiConnection);
        }
        c
    }

    pub fn save_settings(&self, mut s: Settings) -> Result<(), String> {
        s.sanitize();
        crate::util::write_atomic(&self.settings_path, s.to_json().as_bytes()).map_err(|e| e.to_string())?;
        {
            let mut actions = builtin_actions();
            add_custom_actions(&mut actions, &s);
            *self.actions.write().unwrap() = actions;
        }
        *self.settings.write().unwrap() = s;
        Ok(())
    }

    /// Apply settings that affect native hooks. Call from a blocking context.
    pub fn apply_native_settings(self: &Arc<Self>) {
        let s = self.settings();
        let active = !s.general.paused;
        let want_tap = active && (s.menu.shift_right_click || s.expander.enabled);
        let r = if want_tap {
            self.platform.configure_input_tap(true, s.menu.shift_right_click, s.expander.enabled)
        } else {
            self.platform.configure_input_tap(false, false, false).map(|_| false)
        };
        match r {
            Ok(running) => {
                *self.tap_running.lock().unwrap() = running;
                *self.tap_error.lock().unwrap() = None;
            }
            Err(e) => {
                *self.tap_running.lock().unwrap() = false;
                *self.tap_error.lock().unwrap() = Some(e);
            }
        }
        let _ = self.platform.clipboard_watch(active && s.clipboard.history_enabled);
        if s.clipboard.history_enabled {
            let _ = self.db.clip_expire(s.clipboard.retention_days);
        }
        self.trigger_buffer.lock().unwrap().reload(&self.db.snippets().unwrap_or_default());
        crate::shortcuts::register_all(&self.app, &s);
        crate::tray::refresh(&self.app, &s);
        let _ = self.app.emit("status:changed", ());
    }

    pub fn set_paused(self: &Arc<Self>, paused: bool) {
        let mut s = self.settings();
        s.general.paused = paused;
        let _ = self.save_settings(s);
        self.apply_native_settings();
    }

    fn overlay(&self) -> Option<WebviewWindow> {
        self.app.get_webview_window("overlay")
    }

    pub fn hide_overlay(&self) {
        if let Some(w) = self.overlay() {
            let _ = w.hide();
        }
    }

    /// Show the overlay at a logical-point anchor (global, top-left origin), clamped to the display.
    fn show_overlay(&self, mode: &str, payload: Value, size: (f64, f64), anchor: Option<(f64, f64)>) {
        let Some(w) = self.overlay() else { return };
        let (mut x, mut y) = anchor.or_else(|| self.platform.mouse_location().ok()).unwrap_or((200.0, 200.0));
        let (ww, wh) = size;
        if let Some((mx, my, mw, mh)) = monitor_bounds(&self.app, x, y) {
            if anchor.is_none() {
                // Centre palette-style surfaces in the upper third of the active display.
                x = mx + (mw - ww) / 2.0;
                y = my + mh * 0.22;
            } else {
                x += 4.0;
                y += 4.0;
                if x + ww > mx + mw - 8.0 {
                    x = (x - ww - 8.0).max(mx + 8.0);
                }
                if y + wh > my + mh - 8.0 {
                    y = (my + mh - wh - 8.0).max(my + 8.0);
                }
            }
        }
        let _ = w.set_size(LogicalSize::new(ww, wh));
        let _ = w.set_position(LogicalPosition::new(x, y));
        let _ = w.emit("overlay:show", json!({ "mode": mode, "payload": payload }));
        let _ = w.show();
        let _ = w.set_focus();
    }

    fn new_invocation(&self, app: AppInfo) -> Invocation {
        Invocation { id: self.next_id.fetch_add(1, Ordering::Relaxed), app, ..Default::default() }
    }

    fn excluded(&self, app: &AppInfo) -> bool {
        let s = self.settings.read().unwrap();
        s.general.paused || s.is_app_excluded(&app.bundle_id) || s.profile_for(&app.bundle_id).map_or(false, |p| p.disable_overlays)
    }

    // ---------- Context menu ----------

    /// Triggered by Shift+Right-Click (helper) or the menu shortcut. Blocking.
    pub fn open_context_menu(self: &Arc<Self>, at: Option<(f64, f64)>) {
        let focus = match self.platform.focus_info() {
            Ok(f) => f,
            Err(e) => {
                self.toast(&format!("MinPoint can't read the focused app: {e}"), "error");
                return;
            }
        };
        if self.excluded(&focus.app) || focus.app.bundle_id == self.app.config().identifier {
            return;
        }
        let mut inv = self.new_invocation(focus.app.clone());
        inv.editable = focus.editable;
        inv.secure = focus.secure;
        if !focus.secure && focus.ax_available {
            // AX first; copy-fallback only if AX didn't report a selection (explicit user action).
            match self.platform.get_selection(!focus.has_selection) {
                Ok(sel) => {
                    inv.selection_source = sel.source;
                    inv.selection = sel.text;
                }
                Err(e) => log::debug!("selection unavailable: {e}"),
            }
        }
        inv.selection_len = inv.selection.chars().count();
        let ctx = MenuContext {
            app_id: inv.app.bundle_id.clone(),
            app_name: inv.app.name.clone(),
            has_selection: !inv.selection.trim().is_empty(),
            editable: inv.editable || !inv.selection.is_empty(),
            secure_field: inv.secure,
            capabilities: self.capabilities(),
        };
        let entries = menu::resolve(&self.actions.read().unwrap(), &self.settings.read().unwrap(), &ctx);
        if entries.is_empty() {
            return;
        }
        let rows = entries.iter().filter(|e| !matches!(e, MenuEntry::Separator)).count() as f64;
        let seps = entries.iter().filter(|e| matches!(e, MenuEntry::Separator)).count() as f64;
        let compact = self.settings.read().unwrap().appearance.compact;
        let row_h = if compact { 26.0 } else { 30.0 };
        let h = (rows * row_h + seps * 9.0 + 44.0).min(560.0);
        let payload = json!({ "entries": entries, "invocation": &inv, "aiReady": self.ai.is_ready() });
        *self.invocation.lock().unwrap() = Some(inv);
        let anchor = at.or_else(|| self.platform.mouse_location().ok());
        self.show_overlay("menu", payload, (290.0, h), anchor);
    }

    pub fn toast(&self, message: &str, kind: &str) {
        let _ = self.app.emit("toast", json!({ "message": message, "kind": kind }));
        log::info!("toast[{kind}]: {message}");
    }

    fn selection(&self) -> String {
        self.invocation.lock().unwrap().as_ref().map(|i| i.selection.clone()).unwrap_or_default()
    }

    fn invocation_app(&self) -> Option<AppInfo> {
        self.invocation.lock().unwrap().as_ref().map(|i| i.app.clone())
    }

    /// Execute a registry action in the current invocation. Blocking.
    pub fn run_action(self: &Arc<Self>, id: &str, params: Value) -> Result<(), String> {
        let action = self.actions.read().unwrap().iter().find(|a| a.id == id).cloned().ok_or_else(|| format!("Unknown action {id}"))?;
        // Capability gate (never run something the platform can't do).
        let caps = self.capabilities();
        if let Some(missing) = action.capabilities.iter().find(|c| !caps.contains(c)) {
            return Err(if *missing == Capability::AiConnection {
                "Connect your ChatGPT account first (MinPoint → Connected Account).".into()
            } else {
                format!("{} isn't available on this system ({missing:?}).", action.name)
            });
        }
        self.db.record_usage(&action.id);
        let sel = self.selection();
        match &action.kind {
            ActionKind::Writing(op) => {
                let mut options = WritingOptions::default();
                if *op == WritingOp::Translate {
                    options.target_language = params.get("language").and_then(Value::as_str).map(str::to_string).or_else(|| Some(self.settings().ai.default_target_language));
                }
                if let Some(ci) = params.get("customInstructions").and_then(Value::as_str) {
                    options.custom_instructions = Some(ci.to_string());
                }
                if let Some(c) = action_custom_instructions(&self.settings(), &action.id) {
                    options.custom_instructions = Some(c);
                }
                self.start_ai_preview(&action.id, &action.name, *op, sel, options);
                Ok(())
            }
            ActionKind::Text(op) => {
                let out = minpoint_core::text::apply(*op, &sel);
                self.show_local_preview(&action.name, &sel, &out, true);
                Ok(())
            }
            ActionKind::Dev(op) => {
                match minpoint_core::devtools::apply(*op, &sel) {
                    Ok(out) => self.show_local_preview(&action.name, &sel, &out, !matches!(op, minpoint_core::devtools::DevOp::JsonValidate)),
                    Err(e) => self.show_local_preview(&action.name, &sel, &e, false),
                }
                Ok(())
            }
            ActionKind::Builtin(b) => self.run_builtin(*b, &sel, params),
        }
    }

    fn show_local_preview(&self, title: &str, original: &str, result: &str, replaceable: bool) {
        let inv = self.invocation.lock().unwrap().clone().unwrap_or_default();
        self.show_overlay(
            "preview",
            json!({ "title": title, "original": original, "result": result, "streaming": false, "ai": false, "replaceable": replaceable && has_target(&inv), "app": inv.app.name }),
            (560.0, 430.0),
            None,
        );
    }

    fn start_ai_preview(self: &Arc<Self>, action_id: &str, title: &str, op: WritingOp, text: String, options: WritingOptions) {
        let req = match writing::build_request(op, &text, &options) {
            Ok(r) => r,
            Err(e) => {
                self.toast(&e, "error");
                return;
            }
        };
        *self.last_preview.lock().unwrap() = Some(PreviewRequest { action_id: action_id.into(), op, options: options.clone() });
        let inv = self.invocation.lock().unwrap().clone().unwrap_or_default();
        let model = self.settings().ai.model.clone();
        self.show_overlay(
            "preview",
            json!({ "title": title, "original": text, "result": "", "streaming": true, "ai": true, "op": op, "replaceable": has_target(&inv) && !matches!(op, WritingOp::Explain | WritingOp::ExplainCode | WritingOp::ExplainError), "app": inv.app.name, "model": model, "language": options.target_language }),
            (560.0, 430.0),
            None,
        );
        let (tx, rx) = tokio::sync::watch::channel(false);
        if let Some(old) = self.ai_cancel.lock().unwrap().replace(tx) {
            let _ = old.send(true);
        }
        let rt = self.clone();
        tauri::async_runtime::spawn(async move {
            let app = rt.app.clone();
            let r = rt
                .ai
                .transform(model.as_deref(), &req, |d| {
                    let _ = app.emit_to("overlay", "preview:delta", d);
                }, rx)
                .await;
            match r {
                Ok(out) => {
                    let cleaned = writing::clean_output(op, &text, &out);
                    let auto = rt.settings().ai.auto_replace && has_target(&inv) && !matches!(op, WritingOp::Explain | WritingOp::ExplainCode | WritingOp::ExplainError | WritingOp::Summarize);
                    let _ = app.emit_to("overlay", "preview:done", json!({ "result": cleaned, "autoReplace": auto }));
                }
                Err(AiError { kind, message }) => {
                    if kind != "cancelled" {
                        let _ = app.emit_to("overlay", "preview:error", json!({ "kind": kind, "message": message }));
                        let _ = app.emit("status:changed", ());
                    }
                }
            }
        });
    }

    /// Run an action on text that MinPoint itself produced (e.g. OCR output).
    /// There is no insertion target, so results can only be copied.
    pub fn run_on_text(self: &Arc<Self>, id: &str, text: String, params: Value) -> Result<(), String> {
        *self.invocation.lock().unwrap() = Some(Invocation { selection_len: text.chars().count(), selection: text, selection_source: "minpoint".into(), ..Default::default() });
        self.run_action(id, params)
    }

    pub fn cancel_ai(&self) {
        if let Some(tx) = self.ai_cancel.lock().unwrap().take() {
            let _ = tx.send(true);
        }
    }

    pub fn regenerate(self: &Arc<Self>, custom_instructions: Option<String>, op_override: Option<WritingOp>, language: Option<String>) -> Result<(), String> {
        let p = self.last_preview.lock().unwrap().clone().ok_or("Nothing to regenerate.")?;
        let mut options = p.options.clone();
        if custom_instructions.as_deref().map_or(false, |s| !s.trim().is_empty()) {
            options.custom_instructions = custom_instructions;
        }
        if language.is_some() {
            options.target_language = language;
        }
        let op = op_override.unwrap_or(p.op);
        let title = self.actions.read().unwrap().iter().find(|a| a.id == p.action_id).map(|a| a.name.clone()).unwrap_or_else(|| "Rewrite".into());
        self.start_ai_preview(&p.action_id, &title, op, self.selection(), options);
        Ok(())
    }

    /// "Write with ChatGPT": generate from the user's prompt.
    pub fn generate(self: &Arc<Self>, prompt: String) -> Result<(), String> {
        if prompt.trim().is_empty() {
            return Err("Type what you want to write.".into());
        }
        self.db.record_usage("ai.write");
        self.start_ai_preview("ai.write", "Write with ChatGPT", WritingOp::Generate, prompt, WritingOptions::default());
        Ok(())
    }

    /// Insert/replace text in the app that was focused at invocation time.
    /// Refuses (and keeps the preview) if focus or selection changed.
    pub fn accept(self: &Arc<Self>, text: &str) -> Result<String, String> {
        let inv = self.invocation.lock().unwrap().clone().ok_or("The original text field is no longer available. Copy the result instead.")?;
        if inv.secure {
            return Err("The target is a protected field.".into());
        }
        self.hide_overlay();
        self.platform.activate_app(inv.app.pid)?;
        std::thread::sleep(Duration::from_millis(180));
        let front = self.platform.frontmost_app()?;
        if front.pid != inv.app.pid {
            return Err(format!("{} is no longer in front. Copy the result instead.", inv.app.name));
        }
        if inv.selection_source == "accessibility" && !inv.selection.is_empty() {
            if let Ok(now) = self.platform.get_selection(false) {
                if now.source == "accessibility" && now.text != inv.selection {
                    return Err("The selection changed while MinPoint was working. Copy the result instead.".into());
                }
            }
        }
        let method = self.platform.insert_text(text, Some(inv.app.pid)).map_err(|e| {
            if e == "FOCUS_CHANGED" {
                format!("{} is no longer in front. Copy the result instead.", inv.app.name)
            } else {
                e
            }
        })?;
        *self.last_replacement.lock().unwrap() = Some(LastReplacement { pid: inv.app.pid, app_name: inv.app.name.clone() });
        *self.invocation.lock().unwrap() = None;
        Ok(method)
    }

    pub fn undo_last(&self) -> Result<(), String> {
        let r = self.last_replacement.lock().unwrap().take().ok_or("Nothing to undo.")?;
        self.hide_overlay();
        self.platform.activate_app(r.pid)?;
        std::thread::sleep(Duration::from_millis(150));
        if self.platform.frontmost_app()?.pid != r.pid {
            return Err(format!("{} is not in front; undo skipped.", r.app_name));
        }
        self.platform.undo(r.pid)
    }

    fn run_builtin(self: &Arc<Self>, b: Builtin, sel: &str, params: Value) -> Result<(), String> {
        use Builtin::*;
        let inv_app = self.invocation_app();
        match b {
            Copy | CopyPlainText => {
                self.platform.clipboard_write(sel)?;
                self.hide_overlay();
                self.toast("Copied", "success");
            }
            PastePlainText => {
                let t = self.platform.clipboard_read()?;
                self.accept(&t)?;
            }
            InsertDate | InsertTime | UuidInsert => {
                let now = chrono::Local::now();
                let t = match b {
                    InsertDate => now.format("%Y-%m-%d").to_string(),
                    InsertTime => now.format("%H:%M").to_string(),
                    _ => minpoint_core::devtools::uuid_v4(),
                };
                self.accept(&t)?;
            }
            Calculate => {
                let res = if let Some(c) = minpoint_core::units::parse_and_convert(sel) {
                    Ok(minpoint_core::calc::format_number((c.result * 1e6).round() / 1e6))
                } else {
                    minpoint_core::calc::evaluate(sel).map(minpoint_core::calc::format_number).map_err(|e| e.to_string())
                };
                match res {
                    Ok(v) => self.show_local_preview("Calculate", sel, &v, true),
                    Err(e) => self.show_local_preview("Calculate", sel, &format!("Can't calculate: {e}"), false),
                }
            }
            TextStats => {
                let s = minpoint_core::text::stats(sel);
                let out = format!(
                    "Words: {}\nCharacters: {}\nCharacters (no spaces): {}\nLines: {}\nSentences: {}\nReading time: ~{}s",
                    s.words, s.characters, s.characters_no_spaces, s.lines, s.sentences, s.reading_seconds
                );
                self.show_local_preview("Text statistics", sel, &out, false);
            }
            ReadAloud => {
                self.hide_overlay();
                let s = self.settings().speech;
                self.platform.speak(sel, s.voice.as_deref(), s.rate, s.volume)?;
                let _ = self.app.emit("speech:started", ());
            }
            StopSpeaking => self.platform.speech_control("stop")?,
            SaveAsSnippet => {
                self.hide_overlay();
                crate::show_main(&self.app, Some("snippets"));
                let _ = self.app.emit_to("main", "snippet:new", json!({ "content": sel }));
            }
            InsertSnippet => {
                let list = self.db.snippets()?;
                self.show_overlay("snippets", json!({ "snippets": list }), (520.0, 400.0), None);
            }
            ClipboardHistory => self.open_clipboard_picker(inv_app),
            WriteWithAi => {
                let templates: Vec<Value> = writing::write_templates().into_iter().map(|(id, name, prefix)| json!({ "id": id, "name": name, "prefix": prefix })).collect();
                let inv = self.invocation.lock().unwrap().clone().unwrap_or_default();
                self.show_overlay("write", json!({ "templates": templates, "app": inv.app.name }), (560.0, 340.0), None);
            }
            QuickLauncher => self.open_launcher(),
            ScreenTextCapture => self.ocr_flow(),
            Screenshot => {
                self.hide_overlay();
                let mode = params.get("mode").and_then(Value::as_str).unwrap_or("area").to_string();
                self.screenshot_flow(&mode)?;
            }
            WindowLeftHalf | WindowRightHalf | WindowMaximize | WindowCenter | WindowTopLeft | WindowTopRight | WindowBottomLeft | WindowBottomRight | WindowNextDisplay => {
                self.hide_overlay();
                if let Some(app) = &inv_app {
                    let _ = self.platform.activate_app(app.pid);
                    std::thread::sleep(Duration::from_millis(120));
                }
                let a = match b {
                    WindowLeftHalf => "left",
                    WindowRightHalf => "right",
                    WindowMaximize => "maximize",
                    WindowCenter => "center",
                    WindowTopLeft => "top_left",
                    WindowTopRight => "top_right",
                    WindowBottomLeft => "bottom_left",
                    WindowBottomRight => "bottom_right",
                    _ => "next_display",
                };
                self.platform.window_action(a)?;
            }
            OpenApp => {
                let path = params.get("path").and_then(Value::as_str).ok_or("path required")?;
                self.platform.launch_app(path)?;
                self.hide_overlay();
            }
            PauseMinPoint => self.set_paused(true),
            StartVoiceTyping => return Err("Voice typing is not available in this build yet.".into()),
        }
        Ok(())
    }

    pub fn open_launcher(&self) {
        if let Ok(app) = self.platform.frontmost_app() {
            if app.bundle_id != self.app.config().identifier {
                let inv = self.new_invocation(app);
                *self.invocation.lock().unwrap() = Some(inv);
            }
        }
        self.show_overlay("palette", json!({}), (640.0, 420.0), None);
    }

    pub fn open_clipboard_picker(&self, target: Option<AppInfo>) {
        if !self.settings().clipboard.history_enabled {
            crate::show_main(&self.app, Some("clipboard"));
            self.toast("Clipboard history is off. Turn it on in Smart Clipboard.", "info");
            return;
        }
        let target = target.or_else(|| self.platform.frontmost_app().ok());
        if let Some(app) = target {
            if app.bundle_id != self.app.config().identifier {
                let mut inv = self.new_invocation(app);
                inv.editable = true;
                *self.invocation.lock().unwrap() = Some(inv);
            }
        }
        let items = self.db.clip_list("", 200).unwrap_or_default();
        self.show_overlay("clipboard", json!({ "items": items }), (560.0, 440.0), None);
    }

    pub fn ocr_flow(&self) {
        self.hide_overlay();
        match self.platform.ocr_capture() {
            Ok(r) => {
                self.db.record_usage("ocr.capture");
                self.show_overlay("ocr", json!({ "text": r.text, "lines": r.lines, "confidence": r.confidence, "aiReady": self.ai.is_ready() }), (560.0, 420.0), None);
            }
            Err(e) if e == "CANCELLED" => {}
            Err(e) => {
                self.toast(&format!("Screen text capture failed: {e}"), "error");
                let _ = self.platform.request_permission("screenRecording");
            }
        }
    }

    pub fn screenshot_flow(&self, mode: &str) -> Result<String, String> {
        let path = match self.platform.screenshot(mode) {
            Ok(p) => p,
            Err(e) if e == "CANCELLED" => return Ok(String::new()),
            Err(e) => return Err(e),
        };
        let s = self.settings().screenshot;
        let mut saved = String::new();
        if s.copy_to_clipboard {
            self.platform.copy_image(&path)?;
        }
        if s.save_to_disk {
            let dir = if s.save_dir.is_empty() {
                self.app.path().picture_dir().map_err(|e| e.to_string())?.join("MinPoint")
            } else {
                PathBuf::from(&s.save_dir)
            };
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let dest = dir.join(format!("Screenshot {}.png", chrono::Local::now().format("%Y-%m-%d at %H.%M.%S")));
            std::fs::copy(&path, &dest).map_err(|e| e.to_string())?;
            saved = dest.to_string_lossy().into_owned();
        }
        let _ = std::fs::remove_file(&path);
        self.db.record_usage("screenshot.area");
        self.toast(if saved.is_empty() { "Screenshot copied" } else { "Screenshot saved" }, "success");
        Ok(saved)
    }

    /// Paste text into the invocation target (clipboard picker, snippets, palette results).
    pub fn paste_into_target(self: &Arc<Self>, text: &str) -> Result<(), String> {
        if self.invocation.lock().unwrap().is_none() {
            self.platform.clipboard_write(text)?;
            self.hide_overlay();
            self.toast("Copied to clipboard", "success");
            return Ok(());
        }
        self.accept(text).map(|_| ())
    }

    // ---------- native events ----------

    pub fn on_helper_event(self: &Arc<Self>, event: &str, data: Value) {
        match event {
            "contextMenu" => {
                let at = (data["x"].as_f64().unwrap_or(0.0), data["y"].as_f64().unwrap_or(0.0));
                let rt = self.clone();
                std::thread::spawn(move || rt.open_context_menu(Some(at)));
            }
            "clipboardChanged" => {
                let s = self.settings();
                if !s.clipboard.history_enabled || s.general.paused {
                    return;
                }
                let app = data.pointer("/app/bundleId").and_then(Value::as_str).unwrap_or("");
                if s.clipboard.excluded_apps.iter().chain(s.general.excluded_apps.iter()).any(|a| a.eq_ignore_ascii_case(app)) {
                    return;
                }
                let text = data["text"].as_str().unwrap_or("");
                if text.trim().is_empty() || text.len() > 1_000_000 {
                    return;
                }
                let kind = data["kind"].as_str().unwrap_or("text");
                if self.db.clip_add(kind, text, app, s.clipboard.max_items).is_ok() {
                    let _ = self.app.emit("clipboard:changed", ());
                }
            }
            "inputChar" => {
                let s = self.settings.read().unwrap();
                if !s.expander.enabled || s.general.paused {
                    return;
                }
                drop(s);
                let Some(c) = data["c"].as_str().and_then(|x| x.chars().next()) else { return };
                let app = self.current_app.lock().unwrap().clone();
                let m = self.trigger_buffer.lock().unwrap().push(c, &app);
                if let Some(m) = m {
                    let rt = self.clone();
                    std::thread::spawn(move || rt.expand(m));
                }
            }
            "inputBackspace" => self.trigger_buffer.lock().unwrap().backspace(),
            "inputReset" => {
                self.trigger_buffer.lock().unwrap().clear();
                let rt = self.clone();
                std::thread::spawn(move || {
                    if let Ok(a) = rt.platform.frontmost_app() {
                        *rt.current_app.lock().unwrap() = a.bundle_id;
                    }
                });
            }
            "speechFinished" => {
                let _ = self.app.emit("speech:finished", ());
            }
            "speechProgress" => {
                let _ = self.app.emit("speech:progress", data);
            }
            "helperExited" => {
                *self.helper_alive.lock().unwrap() = false;
                *self.tap_running.lock().unwrap() = false;
                let _ = self.app.emit("status:changed", ());
            }
            _ => {}
        }
    }

    fn expand(self: &Arc<Self>, m: snippets::TriggerMatch) {
        let app = self.platform.frontmost_app().unwrap_or_default();
        let s = self.settings();
        if s.is_app_excluded(&app.bundle_id) || s.expander.excluded_apps.iter().any(|a| a.eq_ignore_ascii_case(&app.bundle_id)) {
            return;
        }
        let Some(sn) = self.db.snippets().unwrap_or_default().into_iter().find(|x| x.id == m.snippet_id) else { return };
        let mut counters = self.db.counters();
        let e = snippets::expand(&sn.content, &VarSrc { rt: self }, &mut counters, chrono::Local::now());
        self.db.save_counters(&counters);
        if let Err(err) = self.platform.expand_snippet(m.backspaces, &e.text) {
            self.toast(&format!("Snippet not expanded: {err}"), "error");
        } else {
            self.db.record_usage("snippet.expand");
        }
    }

    pub fn expand_snippet_text(&self, content: &str) -> String {
        let mut counters = self.db.counters();
        let e = snippets::expand(content, &VarSrc { rt: self }, &mut counters, chrono::Local::now());
        self.db.save_counters(&counters);
        e.text
    }
}

struct VarSrc<'a> {
    rt: &'a Runtime,
}

impl VariableSource for VarSrc<'_> {
    fn clipboard(&self) -> Option<String> {
        self.rt.platform.clipboard_read().ok()
    }
    fn username(&self) -> Option<String> {
        std::env::var("USER").or_else(|_| std::env::var("USERNAME")).ok()
    }
    fn user_input(&self, _label: &str) -> Option<String> {
        // Interactive prompts during auto-expansion are not supported yet; placeholder expands empty.
        None
    }
}

/// True when there is a real app/field to insert into (not OCR output or a launcher with no target).
fn has_target(inv: &Invocation) -> bool {
    inv.app.pid > 0 && !inv.secure && inv.selection_source != "minpoint" && (inv.editable || !inv.selection.is_empty())
}

fn add_custom_actions(actions: &mut Vec<ActionDef>, s: &Settings) {
    for c in &s.ai.custom_actions {
        let mut a = actions.iter().find(|a| a.id == "ai.polish").cloned().expect("polish exists");
        a.id = format!("custom.{}", c.id);
        a.name = c.name.clone();
        a.description = c.description.clone();
        a.icon = c.icon.clone();
        a.group = String::new();
        a.kind = ActionKind::Writing(WritingOp::Custom);
        if !c.in_menu {
            // Still runnable from the launcher, but not shown in the context menu.
            a.contexts.clear();
        }
        actions.push(a);
    }
}

fn action_custom_instructions(s: &Settings, id: &str) -> Option<String> {
    let cid = id.strip_prefix("custom.")?;
    s.ai.custom_actions.iter().find(|c| c.id == cid).map(|c| c.instructions.clone())
}

/// Logical bounds (x, y, w, h) of the monitor containing the logical point.
fn monitor_bounds(app: &AppHandle, x: f64, y: f64) -> Option<(f64, f64, f64, f64)> {
    let mons = app.available_monitors().ok()?;
    let mut best = None;
    for m in mons {
        let sf = m.scale_factor();
        let p = m.position();
        let s = m.size();
        let (mx, my, mw, mh) = (p.x as f64 / sf, p.y as f64 / sf, s.width as f64 / sf, s.height as f64 / sf);
        if x >= mx && x < mx + mw && y >= my && y < my + mh {
            return Some((mx, my, mw, mh));
        }
        if best.is_none() {
            best = Some((mx, my, mw, mh));
        }
    }
    best
}
