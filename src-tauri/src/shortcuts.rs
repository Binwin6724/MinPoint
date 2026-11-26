//! Global shortcut registration from settings (action id -> accelerator).

use crate::runtime::Rt;
use inputflow_core::settings::Settings;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub action_id: String,
    pub accelerator: String,
    pub registered: bool,
    pub error: Option<String>,
}

pub struct ShortcutRegistry(pub Mutex<Vec<ShortcutStatus>>);

/// Accelerators reserved by macOS / Windows that we refuse to bind.
const RESERVED: &[&str] = &["cmd+space", "super+space", "cmd+tab", "super+tab", "cmd+q", "super+q", "cmd+c", "super+c", "cmd+v", "super+v", "cmd+x", "cmd+z", "ctrl+alt+delete", "alt+f4", "alt+tab", "super+l"];

pub fn conflicts(s: &Settings) -> Vec<(String, String)> {
    let mut out = vec![];
    let mut seen: Vec<(String, &String)> = vec![];
    for (id, acc) in &s.shortcuts {
        let norm = acc.to_lowercase().replace("commandorcontrol", "cmd").replace("command", "cmd").replace("option", "alt");
        if RESERVED.contains(&norm.as_str()) {
            out.push((id.clone(), format!("{acc} is reserved by the operating system")));
        }
        if let Some((_, other)) = seen.iter().find(|(n, _)| *n == norm) {
            out.push((id.clone(), format!("{acc} is also assigned to {other}")));
        }
        seen.push((norm, id));
    }
    out
}

pub fn register_all(app: &AppHandle, s: &Settings) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let conflicts = conflicts(s);
    let mut statuses = vec![];
    for (action_id, acc) in &s.shortcuts {
        if acc.trim().is_empty() {
            continue;
        }
        if let Some((_, why)) = conflicts.iter().find(|(id, _)| id == action_id) {
            statuses.push(ShortcutStatus { action_id: action_id.clone(), accelerator: acc.clone(), registered: false, error: Some(why.clone()) });
            continue;
        }
        // The emergency shortcut stays active even while paused; everything else is skipped.
        if s.general.paused && action_id != "app.emergency_disable" {
            statuses.push(ShortcutStatus { action_id: action_id.clone(), accelerator: acc.clone(), registered: false, error: Some("InputFlow is paused".into()) });
            continue;
        }
        let id = action_id.clone();
        let r = gs.on_shortcut(acc.as_str(), move |app, _sc, ev| {
            if ev.state != ShortcutState::Pressed {
                return;
            }
            let Some(rt) = app.try_state::<Rt>().map(|s| s.inner().clone()) else { return };
            let id = id.clone();
            std::thread::spawn(move || dispatch(&rt, &id));
        });
        statuses.push(ShortcutStatus {
            action_id: action_id.clone(),
            accelerator: acc.clone(),
            registered: r.is_ok(),
            error: r.err().map(|e| format!("Could not register (in use by another app?): {e}")),
        });
    }
    if let Some(reg) = app.try_state::<ShortcutRegistry>() {
        *reg.0.lock().unwrap() = statuses;
    }
}

fn dispatch(rt: &Rt, id: &str) {
    match id {
        "app.emergency_disable" => {
            let paused = !rt.settings().general.paused;
            rt.set_paused(paused);
            rt.toast(if paused { "InputFlow paused — all remaps and hooks released" } else { "InputFlow resumed" }, "info");
        }
        "menu.open" => rt.open_context_menu(None),
        "launcher.open" => rt.open_launcher(),
        "clip.history" => rt.open_clipboard_picker(None),
        "ocr.capture" => rt.ocr_flow(),
        other => {
            // Any other registry action runs against the frontmost app (e.g. window snapping).
            if let Ok(app) = rt.platform.frontmost_app() {
                let mut inv = crate::runtime::Invocation { app, ..Default::default() };
                if let Ok(sel) = rt.platform.get_selection(false) {
                    inv.selection_len = sel.text.chars().count();
                    inv.selection_source = sel.source;
                    inv.selection = sel.text;
                }
                *rt.invocation.lock().unwrap() = Some(inv);
            }
            if let Err(e) = rt.run_action(other, serde_json::Value::Null) {
                rt.toast(&e, "error");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_conflicts() {
        let mut s = Settings::default();
        assert!(conflicts(&s).is_empty(), "defaults must not conflict");
        s.shortcuts.insert("window.left".into(), "Alt+Space".into());
        s.shortcuts.insert("window.right".into(), "Cmd+Space".into());
        let c = conflicts(&s);
        assert!(c.iter().any(|(id, why)| id == "window.right" && why.contains("reserved")));
        assert!(c.iter().any(|(_, why)| why.contains("also assigned")));
    }
}
