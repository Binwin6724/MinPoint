//! MinPoint desktop app (Tauri 2).

mod ai;
mod commands;
mod db;
mod logging;
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

/// Windows are created only after the runtime state is managed. If the webviews were
/// created up front (from tauri.conf.json), a slow startup — e.g. security software
/// scanning a freshly installed app — let the UI call commands before `Rt` existed,
/// failing with "state not managed".
fn create_windows(app: &AppHandle) -> tauri::Result<()> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    let main = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("MinPoint")
        .inner_size(1120.0, 760.0)
        .min_inner_size(880.0, 560.0)
        .visible(false);
    #[cfg(target_os = "macos")]
    let main = main.title_bar_style(tauri::TitleBarStyle::Overlay).hidden_title(true);
    main.build()?;
    WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html#/overlay".into()))
        .title("MinPoint Overlay")
        .inner_size(300.0, 420.0)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(true)
        .shadow(true)
        .visible_on_all_workspaces(true)
        .build()?;
    Ok(())
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
            if let Ok(dir) = app.path().app_log_dir() {
                logging::init(&dir);
            }
            log::info!("MinPoint {} starting on {} {}", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH);
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&config_dir)?;
            std::fs::create_dir_all(&data_dir)?;
            let settings_path = config_dir.join("settings.json");
            let settings = load_settings(&settings_path);

            log::info!("settings loaded (onboarding complete: {})", settings.general.onboarding_complete);
            // No credential-store access here: it can block (Keychain prompt, slow Credential
            // Manager on managed PCs) and would leave the window blank. Keys load lazily.
            let key_provider = || -> db::KeyProvider { Box::new(|| secrets::database_key().inspect_err(|e| log::error!("database key: {e}"))) };
            let db = match db::Db::open(&data_dir.join("minpoint.db"), key_provider()) {
                Ok(db) => db,
                Err(e) => {
                    // Keep the app usable (settings, tools) instead of exiting with no window.
                    log::error!("database could not be opened ({e}); using a temporary in-memory database");
                    db::Db::open_temporary(key_provider())
                }
            };
            log::info!("database opened");
            let ai = ai::AiService::new(&config_dir);
            let platform = make_platform(sink_setup.clone());
            let rt = Runtime::new(handle.clone(), platform, settings.clone(), settings_path, db, ai);
            let _ = sink_setup.set(rt.clone());
            app.manage(rt.clone());

            log::info!("runtime ready; platform adapter: {}", rt.platform.name());

            if let Err(e) = tray::create(&handle, &settings) {
                log::error!("tray icon could not be created: {e}");
            }
            let rt2 = rt.clone();
            std::thread::spawn(move || {
                rt2.ai.load_stored();
                rt2.apply_native_settings();
                log::info!("native services configured");
                let _ = tauri::Emitter::emit(&rt2.app, "status:changed", ());
            });

            // Create the windows once setup has returned and the event loop is running.
            // Building WebView2 windows *inside* setup blocks until the loop runs (~14 s on
            // Windows); creating them before `Rt` is managed races the UI's first command.
            let h = handle.clone();
            let onboarding_done = settings.general.onboarding_complete;
            tauri::async_runtime::spawn(async move {
                let h2 = h.clone();
                let _ = h.run_on_main_thread(move || {
                    if let Err(e) = create_windows(&h2) {
                        log::error!("could not create windows: {e}");
                        return;
                    }
                    log::info!("windows created");
                    #[cfg(target_os = "macos")]
                    if onboarding_done {
                        // Live in the menu bar; the Dock icon appears when the window opens.
                        let _ = h2.set_activation_policy(tauri::ActivationPolicy::Accessory);
                    } else {
                        show_main(&h2, None);
                    }
                    #[cfg(not(target_os = "macos"))]
                    if !onboarding_done {
                        show_main(&h2, None);
                    }
                });
            });
            log::info!("setup finished");
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
            commands::log_frontend,
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
