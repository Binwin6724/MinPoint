//! MinPoint desktop app (Tauri 2).

mod ai;
mod commands;
mod db;
mod platform;
mod runtime;
mod secrets;
mod shortcuts;
mod tray;
mod util;

use minpoint_core::settings::Settings;
use platform::PlatformAdapter;
use runtime::{Rt, Runtime};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};

pub fn show_main(app: &AppHandle, page: Option<&str>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        if let Some(p) = page {
            let _ = w.emit("navigate", p);
        }
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
}

pub fn quit(app: &AppHandle) {
    if let Some(rt) = app.try_state::<Rt>() {
        rt.platform.shutdown();
    }
    app.exit(0);
}

fn load_settings(path: &std::path::Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(s) => match Settings::from_json(&s) {
            Ok(st) => st,
            Err(e) => {
                // Keep the broken file for the user; never silently discard it.
                let bak = path.with_extension(format!("broken-{}.json", chrono::Utc::now().timestamp()));
                let _ = std::fs::rename(path, &bak);
                log::warn!("settings file was invalid ({e}); backed up to {}", bak.display());
                Settings::default()
            }
        },
        Err(_) => Settings::default(),
    }
}

#[cfg(target_os = "macos")]
fn helper_path() -> std::path::PathBuf {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_default();
    let bundled = exe_dir.join("minpoint-helper");
    if bundled.exists() {
        return bundled;
    }
    // Development fallback (cargo run without bundling).
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../native/macos/build/minpoint-helper")
}

fn make_platform(event_sink: Arc<OnceLock<Rt>>) -> Arc<dyn PlatformAdapter> {
    #[cfg(target_os = "macos")]
    {
        let sink = event_sink.clone();
        let handler: platform::helper::EventHandler = Arc::new(move |ev, data| {
            if let Some(rt) = sink.get() {
                rt.on_helper_event(ev, data);
            }
        });
        match platform::helper::HelperClient::spawn(helper_path(), handler) {
            Ok(h) => return Arc::new(platform::macos::MacAdapter { helper: h }),
            Err(e) => {
                log::error!("{e}");
                return Arc::new(platform::unsupported::UnsupportedAdapter { reason: format!("MinPoint's native helper could not start: {e}") });
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = event_sink;
        Arc::new(platform::unsupported::UnsupportedAdapter { reason: "This feature has not been implemented for this operating system yet.".into() })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let sink: Arc<OnceLock<Rt>> = Arc::new(OnceLock::new());
    let sink_setup = sink.clone();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app, None)))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_opener::init())
        .manage(shortcuts::ShortcutRegistry(Mutex::new(vec![])))
        .setup(move |app| {
            let handle = app.handle().clone();
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&config_dir)?;
            std::fs::create_dir_all(&data_dir)?;
            let settings_path = config_dir.join("settings.json");
            let settings = load_settings(&settings_path);

            let key = secrets::database_key().unwrap_or_else(|e| {
                log::error!("{e}; clipboard history will use a session-only key");
                let mut k = [0u8; 32];
                rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut k);
                k
            });
            let db = db::Db::open(&data_dir.join("minpoint.db"), &key).map_err(|e| Box::<dyn std::error::Error>::from(e))?;
            let ai = ai::AiService::new(&config_dir);
            let platform = make_platform(sink_setup.clone());
            let rt = Runtime::new(handle.clone(), platform, settings.clone(), settings_path, db, ai);
            let _ = sink_setup.set(rt.clone());
            app.manage(rt.clone());

            tray::create(&handle, &settings)?;
            let rt2 = rt.clone();
            std::thread::spawn(move || rt2.apply_native_settings());

            #[cfg(target_os = "macos")]
            if settings.general.onboarding_complete {
                // Live in the menu bar; the Dock icon appears when the window opens.
                let _ = handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
            } else {
                show_main(&handle, None);
            }
            #[cfg(not(target_os = "macos"))]
            if !settings.general.onboarding_complete {
                show_main(&handle, None);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                let keep = window.app_handle().try_state::<Rt>().map_or(true, |rt| rt.settings().general.keep_running_on_close);
                if keep {
                    api.prevent_close();
                    let _ = window.hide();
                    #[cfg(target_os = "macos")]
                    let _ = window.app_handle().set_activation_policy(tauri::ActivationPolicy::Accessory);
                } else {
                    quit(window.app_handle());
                }
            }
            WindowEvent::Focused(false) if window.label() == "overlay" => {
                // Overlays dismiss when the user clicks elsewhere (frontend decides for busy states).
                let _ = window.emit("overlay:blur", ());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::save_settings,
            commands::get_status,
            commands::set_paused,
            commands::request_permission,
            commands::open_privacy_pane,
            commands::restart_input_services,
            commands::overlay_run,
            commands::overlay_run_on_text,
            commands::overlay_accept,
            commands::overlay_copy,
            commands::overlay_cancel,
            commands::overlay_regenerate,
            commands::overlay_generate,
            commands::overlay_undo,
            commands::has_undo,
            commands::paste_text,
            commands::ai_connect,
            commands::ai_cancel_connect,
            commands::ai_disconnect,
            commands::ai_status,
            commands::ai_models,
            commands::ai_dismiss_welcome,
            commands::open_external,
            commands::clipboard_list,
            commands::clipboard_paste,
            commands::clipboard_copy,
            commands::clipboard_flag,
            commands::clipboard_delete,
            commands::clipboard_clear,
            commands::snippets_list,
            commands::snippet_save,
            commands::snippet_delete,
            commands::snippet_insert,
            commands::snippets_export,
            commands::snippets_import,
            commands::launcher_search,
            commands::launcher_execute,
            commands::text_tool,
            commands::text_stats,
            commands::text_find_replace,
            commands::dev_tool,
            commands::dev_regex,
            commands::dev_uuid,
            commands::dev_date_diff,
            commands::calculate,
            commands::voices,
            commands::speak,
            commands::speech_control,
            commands::ocr_capture,
            commands::screenshot,
            commands::window_action,
            commands::privacy_clear_activity,
            commands::privacy_delete_all,
            commands::export_config,
            commands::import_config,
            commands::set_launch_at_login,
            commands::shortcut_conflicts,
            commands::hide_main,
        ])
        .build(tauri::generate_context!())
        .expect("error while building MinPoint");

    app.run(|app, ev| match ev {
        RunEvent::ExitRequested { api, code, .. } => {
            // Keep running in the menu bar when the last window closes (unless quitting explicitly).
            if code.is_none() {
                api.prevent_exit();
            } else if let Some(rt) = app.try_state::<Rt>() {
                rt.platform.shutdown();
            }
        }
        RunEvent::Exit => {
            if let Some(rt) = app.try_state::<Rt>() {
                rt.platform.shutdown();
            }
        }
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => show_main(app, None),
        _ => {}
    });
}
