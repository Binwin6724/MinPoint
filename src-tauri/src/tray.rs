//! Menu bar (macOS) / system tray (Windows).

use crate::runtime::Rt;
use minpoint_core::settings::Settings;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

fn build_menu(app: &AppHandle, s: &Settings) -> tauri::Result<Menu<tauri::Wry>> {
    let paused = s.general.paused;
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", "Open MinPoint", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "launcher", "Quick Launcher", !paused, None::<&str>)?,
            &MenuItem::with_id(app, "clipboard", "Clipboard History", !paused, None::<&str>)?,
            &MenuItem::with_id(app, "ocr", "Screen Text Capture", !paused, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "pause", if paused { "Resume MinPoint" } else { "Pause MinPoint" }, true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "Quit MinPoint", true, None::<&str>)?,
        ],
    )
}

pub fn create(app: &AppHandle, s: &Settings) -> tauri::Result<()> {
    let icon = Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("MinPoint")
        .menu(&build_menu(app, s)?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, ev| {
            let Some(rt) = app.try_state::<Rt>().map(|s| s.inner().clone()) else { return };
            let id = ev.id().as_ref().to_string();
            std::thread::spawn(move || match id.as_str() {
                "open" => crate::show_main(&rt.app, None),
                "launcher" => rt.open_launcher(),
                "clipboard" => rt.open_clipboard_picker(None),
                "ocr" => rt.ocr_flow(),
                "pause" => {
                    let p = !rt.settings().general.paused;
                    rt.set_paused(p);
                }
                "quit" => crate::quit(&rt.app),
                _ => {}
            });
        })
        .build(app)?;
    Ok(())
}

pub fn refresh(app: &AppHandle, s: &Settings) {
    if let Some(t) = app.tray_by_id("main") {
        if let Ok(m) = build_menu(app, s) {
            let _ = t.set_menu(Some(m));
        }
        let _ = t.set_tooltip(Some(if s.general.paused { "MinPoint (paused)" } else { "MinPoint" }));
    }
}
