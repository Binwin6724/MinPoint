//! Platform adapters. Every OS-specific capability goes through `PlatformAdapter`.
//! macOS is implemented via the signed Swift helper; Windows currently reports no
//! native capabilities (so dependent features are hidden, never faked).

pub mod helper;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod unsupported;

use inputflow_core::actions::Capability;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub bundle_id: String,
    pub name: String,
    pub pid: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusInfo {
    #[serde(flatten)]
    pub app: AppInfo,
    pub editable: bool,
    pub has_selection: bool,
    pub secure: bool,
    pub ax_available: bool,
    #[serde(default)]
    pub role: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub text: String,
    /// "accessibility" | "copy" | "none"
    pub source: String,
    pub app: AppInfo,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    pub accessibility: String,
    pub input_monitoring: String,
    pub screen_recording: String,
    pub microphone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub language: String,
    pub quality: String,
    pub offline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult {
    pub text: String,
    pub lines: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    pub name: String,
    pub path: String,
}

pub type PResult<T> = Result<T, String>;

/// Blocking interface; call from `spawn_blocking` or a worker thread.
pub trait PlatformAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> Vec<Capability>;
    fn permissions(&self) -> PResult<Permissions>;
    fn request_permission(&self, kind: &str) -> PResult<Permissions>;
    fn open_privacy_pane(&self, kind: &str) -> PResult<()>;
    fn frontmost_app(&self) -> PResult<AppInfo>;
    fn focus_info(&self) -> PResult<FocusInfo>;
    fn get_selection(&self, allow_copy_fallback: bool) -> PResult<Selection>;
    /// Replace selection / insert at caret. `expect_pid` guards against focus changes.
    fn insert_text(&self, text: &str, expect_pid: Option<i64>) -> PResult<String>;
    fn expand_snippet(&self, backspaces: usize, text: &str) -> PResult<()>;
    /// Send the app's native Undo to `pid` (only if it is frontmost).
    fn undo(&self, pid: i64) -> PResult<()>;
    fn clipboard_read(&self) -> PResult<String>;
    fn clipboard_write(&self, text: &str) -> PResult<()>;
    fn clipboard_watch(&self, enabled: bool) -> PResult<()>;
    fn configure_input_tap(&self, enabled: bool, shift_right_click: bool, key_events: bool) -> PResult<bool>;
    fn ocr_capture(&self) -> PResult<OcrResult>;
    fn screenshot(&self, mode: &str) -> PResult<String>;
    fn copy_image(&self, path: &str) -> PResult<()>;
    fn voices(&self) -> PResult<Vec<Voice>>;
    fn speak(&self, text: &str, voice: Option<&str>, rate: f32, volume: f32) -> PResult<()>;
    fn speech_control(&self, op: &str) -> PResult<()>;
    fn window_action(&self, action: &str) -> PResult<()>;
    fn mouse_location(&self) -> PResult<(f64, f64)>;
    fn activate_app(&self, pid: i64) -> PResult<()>;
    fn launch_app(&self, path: &str) -> PResult<()>;
    fn installed_apps(&self) -> Vec<InstalledApp>;
    fn shutdown(&self);
}

pub(crate) fn from_value<T: serde::de::DeserializeOwned>(v: Value) -> PResult<T> {
    serde_json::from_value(v).map_err(|e| format!("unexpected helper response: {e}"))
}
