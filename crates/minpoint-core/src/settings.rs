//! Versioned local configuration. Stored as JSON in the app config directory.
//! Every field has a default so older/partial files load cleanly; `migrate`
//! upgrades older schema versions in place.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub version: u32,
    pub general: General,
    pub menu: MenuConfig,
    pub appearance: Appearance,
    pub ai: AiSettings,
    pub clipboard: ClipboardSettings,
    pub expander: ExpanderSettings,
    pub speech: SpeechSettings,
    pub screenshot: ScreenshotSettings,
    /// action id -> accelerator (Tauri syntax, e.g. "Alt+Space").
    pub shortcuts: BTreeMap<String, String>,
    pub profiles: Vec<Profile>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut shortcuts = BTreeMap::new();
        shortcuts.insert("launcher.open".into(), "Alt+Space".into());
        shortcuts.insert("menu.open".into(), "Ctrl+Alt+Space".into());
        shortcuts.insert("ocr.capture".into(), "Ctrl+Alt+Shift+T".into());
        shortcuts.insert("clip.history".into(), "Ctrl+Alt+V".into());
        shortcuts.insert("app.emergency_disable".into(), "Ctrl+Alt+Shift+Escape".into());
        Settings {
            version: SCHEMA_VERSION,
            general: General::default(),
            menu: MenuConfig::default(),
            appearance: Appearance::default(),
            ai: AiSettings::default(),
            clipboard: ClipboardSettings::default(),
            expander: ExpanderSettings::default(),
            speech: SpeechSettings::default(),
            screenshot: ScreenshotSettings::default(),
            shortcuts,
            profiles: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct General {
    pub onboarding_complete: bool,
    pub launch_at_login: bool,
    pub paused: bool,
    /// Keep running in the menu bar when the window is closed.
    pub keep_running_on_close: bool,
    /// Bundle IDs / exe names where MinPoint is completely disabled.
    pub excluded_apps: Vec<String>,
}

impl Default for General {
    fn default() -> Self {
        General { onboarding_complete: false, launch_at_login: false, paused: false, keep_running_on_close: true, excluded_apps: vec![] }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct MenuConfig {
    /// Shift + right click opens the menu (requires Input Monitoring + Accessibility on macOS).
    pub shift_right_click: bool,
    pub disabled: Vec<String>,
    pub favorites: Vec<String>,
    /// Custom order of action ids; ids not listed keep registry order after these.
    pub order: Vec<String>,
    pub hide_unavailable: bool,
    pub show_groups_as_submenus: bool,
}

impl Default for MenuConfig {
    fn default() -> Self {
        MenuConfig {
            shift_right_click: true,
            disabled: vec![
                // Keep the default menu compact; users can enable these.
                "ai.formal".into(),
                "ai.technical".into(),
                "ai.docs".into(),
                "text.reverse".into(),
            ],
            favorites: vec!["ai.polish".into(), "ai.grammar".into()],
            order: vec![],
            hide_unavailable: true,
            show_groups_as_submenus: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    /// "system" | "light" | "dark"
    pub theme: String,
    pub accent: String,
    pub compact: bool,
    pub font_scale: f32,
    pub reduce_motion: bool,
    pub high_contrast: bool,
    pub menu_opacity: f32,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance { theme: "system".into(), accent: "#ff7a45".into(), compact: false, font_scale: 1.0, reduce_motion: false, high_contrast: false, menu_opacity: 0.96 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AiSettings {
    pub enabled: bool,
    pub model: Option<String>,
    /// Preview-first is the default and recommended mode.
    pub auto_replace: bool,
    pub favorite_languages: Vec<String>,
    pub default_target_language: String,
    pub custom_actions: Vec<CustomAiAction>,
}

impl Default for AiSettings {
    fn default() -> Self {
        AiSettings {
            enabled: true,
            model: None,
            auto_replace: false,
            favorite_languages: vec!["English".into(), "Spanish".into(), "French".into(), "Hindi".into(), "Malayalam".into(), "Tamil".into()],
            default_target_language: "English".into(),
            custom_actions: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomAiAction {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    pub instructions: String,
    #[serde(default)]
    pub in_menu: bool,
    #[serde(default)]
    pub apps: Vec<String>,
}

fn default_icon() -> String {
    "wand-sparkles".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClipboardSettings {
    /// Opt-in: history is off until the user enables it.
    pub history_enabled: bool,
    pub max_items: u32,
    pub retention_days: u32,
    pub capture_images: bool,
    pub excluded_apps: Vec<String>,
}

impl Default for ClipboardSettings {
    fn default() -> Self {
        ClipboardSettings {
            history_enabled: false,
            max_items: 200,
            retention_days: 7,
            capture_images: false,
            excluded_apps: vec![
                "com.agilebits.onepassword7".into(),
                "com.1password.1password".into(),
                "com.bitwarden.desktop".into(),
                "com.apple.keychainaccess".into(),
                "com.apple.Passwords".into(),
                "org.keepassxc.keepassxc".into(),
                "com.lastpass.LastPass".into(),
                "com.dashlane.dashlanephonefinal".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ExpanderSettings {
    pub enabled: bool,
    pub excluded_apps: Vec<String>,
}

impl Default for ExpanderSettings {
    fn default() -> Self {
        ExpanderSettings { enabled: false, excluded_apps: vec![] }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SpeechSettings {
    pub voice: Option<String>,
    /// 0.5 – 2.0 multiplier of the default speaking rate.
    pub rate: f32,
    pub volume: f32,
}

impl Default for SpeechSettings {
    fn default() -> Self {
        SpeechSettings { voice: None, rate: 1.0, volume: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ScreenshotSettings {
    pub copy_to_clipboard: bool,
    pub save_to_disk: bool,
    /// Empty = ~/Pictures/MinPoint
    pub save_dir: String,
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        ScreenshotSettings { copy_to_clipboard: true, save_to_disk: true, save_dir: String::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    /// Bundle IDs / exe names that activate this profile.
    pub apps: Vec<String>,
    #[serde(default)]
    pub priority: i32,
    /// Extra action ids to hide in this profile's menu.
    #[serde(default)]
    pub disabled_actions: Vec<String>,
    /// If set, only these actions are shown (gaming-style minimal profile).
    #[serde(default)]
    pub only_actions: Option<Vec<String>>,
    #[serde(default)]
    pub disable_overlays: bool,
}

impl Settings {
    /// Parse settings, migrating older versions. Unknown/invalid input yields an error
    /// so the caller can back up the file instead of silently overwriting it.
    pub fn from_json(s: &str) -> Result<Settings, serde_json::Error> {
        let mut v: serde_json::Value = serde_json::from_str(s)?;
        migrate(&mut v);
        let mut st: Settings = serde_json::from_value(v)?;
        st.version = SCHEMA_VERSION;
        st.sanitize();
        Ok(st)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("settings serialise")
    }

    /// Clamp values from hand-edited or imported files into safe ranges.
    pub fn sanitize(&mut self) {
        self.clipboard.max_items = self.clipboard.max_items.clamp(10, 5000);
        self.clipboard.retention_days = self.clipboard.retention_days.clamp(1, 365);
        self.speech.rate = self.speech.rate.clamp(0.5, 2.0);
        self.speech.volume = self.speech.volume.clamp(0.0, 1.0);
        self.appearance.font_scale = self.appearance.font_scale.clamp(0.8, 1.5);
        self.appearance.menu_opacity = self.appearance.menu_opacity.clamp(0.6, 1.0);
        if !matches!(self.appearance.theme.as_str(), "system" | "light" | "dark") {
            self.appearance.theme = "system".into();
        }
        let hex = regex::Regex::new(r"^#[0-9a-fA-F]{6}$").unwrap();
        if !hex.is_match(&self.appearance.accent) {
            self.appearance.accent = Appearance::default().accent;
        }
    }

    /// The profile that applies to `app`, highest priority first.
    pub fn profile_for(&self, app: &str) -> Option<&Profile> {
        self.profiles
            .iter()
            .filter(|p| p.apps.iter().any(|a| a.eq_ignore_ascii_case(app)))
            .max_by_key(|p| p.priority)
    }

    pub fn is_app_excluded(&self, app: &str) -> bool {
        self.general.excluded_apps.iter().any(|a| a.eq_ignore_ascii_case(app))
    }
}

fn migrate(v: &mut serde_json::Value) {
    let ver = v.get("version").and_then(|x| x.as_u64()).unwrap_or(0);
    if ver == 0 {
        // v0 (pre-release) stored `menu.trigger: "shift_right_click"`.
        if let Some(menu) = v.get_mut("menu").and_then(|m| m.as_object_mut()) {
            if let Some(t) = menu.remove("trigger") {
                menu.insert("shiftRightClick".into(), serde_json::Value::Bool(t == "shift_right_click"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_privacy_preserving() {
        let s = Settings::default();
        assert!(!s.clipboard.history_enabled, "clipboard history must be opt-in");
        assert!(!s.expander.enabled, "text expansion must be opt-in");
        assert!(!s.ai.auto_replace, "preview-first must be default");
        assert!(s.menu.shift_right_click);
    }

    #[test]
    fn roundtrip_partial_and_migration() {
        let s = Settings::default();
        assert_eq!(Settings::from_json(&s.to_json()).unwrap(), s);
        let partial = Settings::from_json(r#"{"clipboard":{"historyEnabled":true}}"#).unwrap();
        assert!(partial.clipboard.history_enabled);
        assert_eq!(partial.clipboard.max_items, 200);
        let old = Settings::from_json(r#"{"menu":{"trigger":"none"}}"#).unwrap();
        assert!(!old.menu.shift_right_click);
        let clamped = Settings::from_json(r##"{"speech":{"rate":9},"appearance":{"accent":"red","theme":"neon"}}"##).unwrap();
        assert_eq!(clamped.speech.rate, 2.0);
        assert_eq!(clamped.appearance.accent, "#ff7a45");
        assert_eq!(clamped.appearance.theme, "system");
        assert!(Settings::from_json("not json").is_err());
    }

    #[test]
    fn profiles() {
        let mut s = Settings::default();
        s.profiles = vec![
            Profile { id: "a".into(), name: "A".into(), apps: vec!["com.x".into()], priority: 1, disabled_actions: vec![], only_actions: None, disable_overlays: false },
            Profile { id: "b".into(), name: "B".into(), apps: vec!["com.x".into()], priority: 5, disabled_actions: vec![], only_actions: None, disable_overlays: false },
        ];
        assert_eq!(s.profile_for("COM.X").unwrap().id, "b");
        assert!(s.profile_for("com.y").is_none());
    }
}
