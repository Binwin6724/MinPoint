//! macOS adapter backed by the Swift helper (Accessibility, CGEventTap, Vision, AVFoundation).

use super::helper::HelperClient;
use super::*;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

pub struct MacAdapter {
    pub helper: Arc<HelperClient>,
}

const FAST: Duration = Duration::from_secs(3);
const SLOW: Duration = Duration::from_secs(10);
const INTERACTIVE: Duration = Duration::from_secs(180);

impl MacAdapter {
    fn call(&self, m: &str, p: Value, t: Duration) -> PResult<Value> {
        self.helper.call(m, p, t)
    }
}

impl PlatformAdapter for MacAdapter {
    fn name(&self) -> &'static str {
        "macos"
    }

    fn capabilities(&self) -> Vec<Capability> {
        use Capability::*;
        // Reported from what is actually implemented in the helper. Permission
        // state is checked separately at invocation time.
        vec![ReadSelection, ReplaceSelection, InsertText, Clipboard, ClipboardHistory, ScreenCapture, Ocr, Tts, WindowManagement, AppLaunch]
    }

    fn permissions(&self) -> PResult<Permissions> {
        from_value(self.call("permissions", json!({}), FAST)?)
    }
    fn request_permission(&self, kind: &str) -> PResult<Permissions> {
        from_value(self.call("requestPermission", json!({ "kind": kind }), INTERACTIVE)?)
    }
    fn open_privacy_pane(&self, kind: &str) -> PResult<()> {
        self.call("openPrivacyPane", json!({ "kind": kind }), FAST).map(|_| ())
    }
    fn frontmost_app(&self) -> PResult<AppInfo> {
        from_value(self.call("frontmostApp", json!({}), FAST)?)
    }
    fn focus_info(&self) -> PResult<FocusInfo> {
        from_value(self.call("focusInfo", json!({}), FAST)?)
    }
    fn get_selection(&self, allow_copy_fallback: bool) -> PResult<Selection> {
        from_value(self.call("getSelection", json!({ "allowCopyFallback": allow_copy_fallback }), SLOW)?)
    }
    fn insert_text(&self, text: &str, expect_pid: Option<i64>) -> PResult<String> {
        let v = self.call("insertText", json!({ "text": text, "pid": expect_pid }), SLOW)?;
        Ok(v.get("method").and_then(Value::as_str).unwrap_or("unknown").to_string())
    }
    fn expand_snippet(&self, backspaces: usize, text: &str) -> PResult<()> {
        self.call("expandSnippet", json!({ "backspaces": backspaces, "text": text }), SLOW).map(|_| ())
    }
    fn undo(&self, pid: i64) -> PResult<()> {
        self.call("undo", json!({ "pid": pid }), FAST).map(|_| ())
    }
    fn clipboard_read(&self) -> PResult<String> {
        Ok(self.call("clipboardRead", json!({}), FAST)?.get("text").and_then(Value::as_str).unwrap_or_default().to_string())
    }
    fn clipboard_write(&self, text: &str) -> PResult<()> {
        self.call("clipboardWrite", json!({ "text": text }), FAST).map(|_| ())
    }
    fn clipboard_watch(&self, enabled: bool) -> PResult<()> {
        self.call("clipboardWatch", json!({ "enabled": enabled }), FAST).map(|_| ())
    }
    fn configure_input_tap(&self, enabled: bool, shift_right_click: bool, key_events: bool) -> PResult<bool> {
        let v = self.call("tapConfigure", json!({ "enabled": enabled, "shiftRightClick": shift_right_click, "keyEvents": key_events }), FAST)?;
        Ok(v.get("running").and_then(Value::as_bool).unwrap_or(false))
    }
    fn ocr_capture(&self) -> PResult<OcrResult> {
        from_value(self.call("ocr", json!({ "mode": "area" }), INTERACTIVE)?)
    }
    fn screenshot(&self, mode: &str) -> PResult<String> {
        let v = self.call("capture", json!({ "mode": mode }), INTERACTIVE)?;
        v.get("path").and_then(Value::as_str).map(str::to_string).ok_or_else(|| "no capture path".into())
    }
    fn copy_image(&self, path: &str) -> PResult<()> {
        self.call("copyImageToClipboard", json!({ "path": path }), FAST).map(|_| ())
    }
    fn voices(&self) -> PResult<Vec<Voice>> {
        from_value(self.call("voices", json!({}), FAST)?)
    }
    fn speak(&self, text: &str, voice: Option<&str>, rate: f32, volume: f32) -> PResult<()> {
        self.call("speak", json!({ "text": text, "voice": voice, "rate": rate, "volume": volume }), FAST).map(|_| ())
    }
    fn speech_control(&self, op: &str) -> PResult<()> {
        let m = match op {
            "stop" => "stopSpeaking",
            "pause" => "pauseSpeaking",
            "resume" => "continueSpeaking",
            _ => return Err("unknown speech op".into()),
        };
        self.call(m, json!({}), FAST).map(|_| ())
    }
    fn window_action(&self, action: &str) -> PResult<()> {
        self.call("window", json!({ "action": action }), FAST).map(|_| ())
    }
    fn mouse_location(&self) -> PResult<(f64, f64)> {
        let v = self.call("mouseLocation", json!({}), FAST)?;
        Ok((v["x"].as_f64().unwrap_or(0.0), v["y"].as_f64().unwrap_or(0.0)))
    }
    fn activate_app(&self, pid: i64) -> PResult<()> {
        self.call("activate", json!({ "pid": pid }), FAST).map(|_| ())
    }
    fn launch_app(&self, path: &str) -> PResult<()> {
        self.call("launchApp", json!({ "path": path }), FAST).map(|_| ())
    }

    fn installed_apps(&self) -> Vec<InstalledApp> {
        let mut dirs = vec![
            std::path::PathBuf::from("/Applications"),
            std::path::PathBuf::from("/Applications/Utilities"),
            std::path::PathBuf::from("/System/Applications"),
            std::path::PathBuf::from("/System/Applications/Utilities"),
        ];
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(std::path::PathBuf::from(home).join("Applications"));
        }
        let mut out = Vec::new();
        for d in dirs {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("app") {
                    let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
                    out.push(InstalledApp { name, path: p.to_string_lossy().into_owned() });
                }
            }
        }
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out.dedup_by(|a, b| a.name == b.name);
        out
    }

    fn shutdown(&self) {
        self.helper.shutdown();
    }
}
