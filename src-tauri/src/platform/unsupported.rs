//! Adapter used where a native integration has not been implemented yet
//! (currently Windows, and macOS if the helper fails to start). It reports no
//! capabilities, so the menu engine hides dependent actions instead of faking them.

use super::*;

pub struct UnsupportedAdapter {
    pub reason: String,
}

impl UnsupportedAdapter {
    fn err<T>(&self) -> PResult<T> {
        Err(self.reason.clone())
    }
}

impl PlatformAdapter for UnsupportedAdapter {
    fn name(&self) -> &'static str {
        "unsupported"
    }
    fn capabilities(&self) -> Vec<Capability> {
        vec![]
    }
    fn permissions(&self) -> PResult<Permissions> {
        Ok(Permissions {
            accessibility: "unsupported".into(),
            input_monitoring: "unsupported".into(),
            screen_recording: "unsupported".into(),
            microphone: "unsupported".into(),
        })
    }
    fn request_permission(&self, _: &str) -> PResult<Permissions> {
        self.err()
    }
    fn open_privacy_pane(&self, _: &str) -> PResult<()> {
        self.err()
    }
    fn frontmost_app(&self) -> PResult<AppInfo> {
        self.err()
    }
    fn focus_info(&self) -> PResult<FocusInfo> {
        self.err()
    }
    fn get_selection(&self, _: bool) -> PResult<Selection> {
        self.err()
    }
    fn insert_text(&self, _: &str, _: Option<i64>) -> PResult<String> {
        self.err()
    }
    fn expand_snippet(&self, _: usize, _: &str) -> PResult<()> {
        self.err()
    }
    fn undo(&self, _: i64) -> PResult<()> {
        self.err()
    }
    fn clipboard_read(&self) -> PResult<String> {
        self.err()
    }
    fn clipboard_write(&self, _: &str) -> PResult<()> {
        self.err()
    }
    fn clipboard_watch(&self, _: bool) -> PResult<()> {
        self.err()
    }
    fn configure_input_tap(&self, _: bool, _: bool, _: bool) -> PResult<bool> {
        self.err()
    }
    fn ocr_capture(&self) -> PResult<OcrResult> {
        self.err()
    }
    fn screenshot(&self, _: &str) -> PResult<String> {
        self.err()
    }
    fn copy_image(&self, _: &str) -> PResult<()> {
        self.err()
    }
    fn voices(&self) -> PResult<Vec<Voice>> {
        Ok(vec![])
    }
    fn speak(&self, _: &str, _: Option<&str>, _: f32, _: f32) -> PResult<()> {
        self.err()
    }
    fn speech_control(&self, _: &str) -> PResult<()> {
        self.err()
    }
    fn window_action(&self, _: &str) -> PResult<()> {
        self.err()
    }
    fn mouse_location(&self) -> PResult<(f64, f64)> {
        self.err()
    }
    fn activate_app(&self, _: i64) -> PResult<()> {
        self.err()
    }
    fn launch_app(&self, _: &str) -> PResult<()> {
        self.err()
    }
    fn installed_apps(&self) -> Vec<InstalledApp> {
        vec![]
    }
    fn shutdown(&self) {}
}
