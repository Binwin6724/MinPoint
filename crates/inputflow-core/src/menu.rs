//! Resolves which context-menu entries to show for the current situation.

use crate::actions::{group_label, ActionContext, ActionDef, Capability};
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuContext {
    pub app_id: String,
    pub app_name: String,
    pub has_selection: bool,
    pub editable: bool,
    /// Focus is in a password / secure field. Text actions are suppressed.
    pub secure_field: bool,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MenuEntry {
    #[serde(rename_all = "camelCase")]
    Item { id: String, name: String, icon: String, requires_ai: bool, available: bool, reason: Option<String> },
    #[serde(rename_all = "camelCase")]
    Submenu { id: String, name: String, items: Vec<MenuEntry> },
    Separator,
}

/// Build the menu. Returns an empty list when InputFlow is paused or the app is excluded.
pub fn resolve(actions: &[ActionDef], settings: &Settings, ctx: &MenuContext) -> Vec<MenuEntry> {
    if settings.general.paused || settings.is_app_excluded(&ctx.app_id) {
        return vec![];
    }
    let caps: HashSet<Capability> = ctx.capabilities.iter().copied().collect();
    let profile = settings.profile_for(&ctx.app_id);
    let disabled: HashSet<&str> = settings
        .menu
        .disabled
        .iter()
        .chain(profile.map(|p| p.disabled_actions.iter()).into_iter().flatten())
        .map(String::as_str)
        .collect();
    let only: Option<HashSet<&str>> = profile.and_then(|p| p.only_actions.as_ref()).map(|v| v.iter().map(String::as_str).collect());

    // Which contexts are active right now.
    let mut active = vec![ActionContext::Anywhere];
    if ctx.has_selection && !ctx.secure_field {
        active.push(ActionContext::Selection);
    }
    if ctx.editable && !ctx.secure_field {
        active.push(ActionContext::Editable);
    }
    // When text is selected, show selection actions first, then field actions, and
    // skip "anywhere" tools except the launcher to keep the menu focused.
    let primary = if ctx.has_selection && !ctx.secure_field {
        ActionContext::Selection
    } else if ctx.editable && !ctx.secure_field {
        ActionContext::Editable
    } else {
        ActionContext::Anywhere
    };

    let mut ordered: Vec<&ActionDef> = Vec::new();
    for id in &settings.menu.order {
        if let Some(a) = actions.iter().find(|a| &a.id == id) {
            ordered.push(a);
        }
    }
    for a in actions {
        if !settings.menu.order.contains(&a.id) {
            ordered.push(a);
        }
    }

    let fits = |a: &ActionDef| -> bool {
        if a.contexts.is_empty() || disabled.contains(a.id.as_str()) {
            return false;
        }
        if let Some(o) = &only {
            if !o.contains(a.id.as_str()) {
                return false;
            }
        }
        match primary {
            ActionContext::Selection => a.contexts.contains(&ActionContext::Selection),
            ActionContext::Editable => a.contexts.contains(&ActionContext::Editable),
            ActionContext::Anywhere => a.contexts.contains(&ActionContext::Anywhere),
        }
    };

    let to_item = |a: &ActionDef| -> Option<MenuEntry> {
        let missing: Vec<Capability> = a.capabilities.iter().copied().filter(|c| !caps.contains(c)).collect();
        let available = missing.is_empty() && (!a.requires_ai || settings.ai.enabled);
        if !available && settings.menu.hide_unavailable && !(a.requires_ai && settings.ai.enabled) {
            return None;
        }
        let reason = if available {
            None
        } else if missing.contains(&Capability::AiConnection) {
            Some("Connect your ChatGPT account in Settings → Connected Account".to_string())
        } else if !settings.ai.enabled && a.requires_ai {
            Some("AI features are turned off".to_string())
        } else {
            Some(format!("Not available: {}", missing.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>().join(", ")))
        };
        Some(MenuEntry::Item { id: a.id.clone(), name: a.name.clone(), icon: a.icon.clone(), requires_ai: a.requires_ai, available, reason })
    };

    let candidates: Vec<&ActionDef> = ordered.into_iter().filter(|a| fits(a)).collect();
    let favorites: Vec<MenuEntry> = settings
        .menu
        .favorites
        .iter()
        .filter_map(|id| candidates.iter().find(|a| &a.id == id))
        .filter_map(|a| to_item(a))
        .collect();
    let fav_ids: HashSet<&str> = settings.menu.favorites.iter().map(String::as_str).collect();

    let mut out = Vec::new();
    if !favorites.is_empty() {
        out.extend(favorites);
        out.push(MenuEntry::Separator);
    }
    let mut groups: Vec<(String, Vec<MenuEntry>)> = Vec::new();
    let mut top = Vec::new();
    for a in &candidates {
        if fav_ids.contains(a.id.as_str()) {
            continue;
        }
        let Some(item) = to_item(a) else { continue };
        if settings.menu.show_groups_as_submenus && !a.group.is_empty() {
            match groups.iter_mut().find(|(g, _)| g == &a.group) {
                Some((_, v)) => v.push(item),
                None => groups.push((a.group.clone(), vec![item])),
            }
        } else {
            top.push(item);
        }
    }
    out.extend(top);
    if !groups.is_empty() {
        out.push(MenuEntry::Separator);
        for (g, items) in groups {
            out.push(MenuEntry::Submenu { id: format!("group.{g}"), name: group_label(&g).to_string(), items });
        }
    }
    // Always offer the launcher as a way out when not already in "anywhere" mode.
    if primary != ActionContext::Anywhere && !disabled.contains("launcher.open") && only.is_none() {
        if let Some(l) = actions.iter().find(|a| a.id == "launcher.open").and_then(|a| to_item(a)) {
            out.push(MenuEntry::Separator);
            out.push(l);
        }
    }
    while matches!(out.last(), Some(MenuEntry::Separator)) {
        out.pop();
    }
    let _ = active;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::builtin_actions;

    fn ids(entries: &[MenuEntry]) -> Vec<String> {
        let mut v = vec![];
        for e in entries {
            match e {
                MenuEntry::Item { id, .. } => v.push(id.clone()),
                MenuEntry::Submenu { items, .. } => v.extend(ids(items)),
                MenuEntry::Separator => {}
            }
        }
        v
    }

    fn all_caps() -> Vec<Capability> {
        use Capability::*;
        vec![ReadSelection, ReplaceSelection, InsertText, Clipboard, ClipboardHistory, ScreenCapture, Ocr, Tts, WindowManagement, AiConnection]
    }

    #[test]
    fn selection_menu_shows_writing_actions() {
        let s = Settings::default();
        let ctx = MenuContext { has_selection: true, capabilities: all_caps(), ..Default::default() };
        let m = ids(&resolve(&builtin_actions(), &s, &ctx));
        assert_eq!(m[0], "ai.polish", "favorites first");
        assert!(m.contains(&"ai.translate".to_string()));
        assert!(m.contains(&"case.upper".to_string()));
        assert!(!m.contains(&"ai.formal".to_string()), "disabled by default");
        assert!(!m.contains(&"ocr.capture".to_string()));
        assert_eq!(m.last().unwrap(), "launcher.open");
    }

    #[test]
    fn no_selection_menu() {
        let s = Settings::default();
        let ctx = MenuContext { capabilities: all_caps(), ..Default::default() };
        let m = ids(&resolve(&builtin_actions(), &s, &ctx));
        assert!(m.contains(&"launcher.open".to_string()));
        assert!(m.contains(&"ocr.capture".to_string()));
        assert!(m.contains(&"window.left".to_string()));
        assert!(!m.contains(&"ai.polish".to_string()));
    }

    #[test]
    fn ai_actions_marked_unavailable_when_disconnected_and_hidden_missing_caps() {
        let s = Settings::default();
        let ctx = MenuContext { has_selection: true, capabilities: vec![Capability::ReadSelection], ..Default::default() };
        let menu = resolve(&builtin_actions(), &s, &ctx);
        let polish = menu.iter().find_map(|e| match e {
            MenuEntry::Item { id, available, reason, .. } if id == "ai.polish" => Some((*available, reason.clone())),
            _ => None,
        });
        let (avail, reason) = polish.expect("AI item shown with explanation");
        assert!(!avail);
        assert!(reason.unwrap().contains("Connect"));
        assert!(!ids(&menu).contains(&"tts.read".to_string()), "missing TTS capability hides item");
    }

    #[test]
    fn secure_fields_paused_and_excluded() {
        let mut s = Settings::default();
        let ctx = MenuContext { has_selection: true, editable: true, secure_field: true, capabilities: all_caps(), ..Default::default() };
        let m = ids(&resolve(&builtin_actions(), &s, &ctx));
        assert!(!m.iter().any(|i| i.starts_with("ai.") || i.starts_with("case.")), "no text actions in secure fields");
        s.general.excluded_apps.push("com.game".into());
        let ctx2 = MenuContext { app_id: "com.game".into(), capabilities: all_caps(), ..Default::default() };
        assert!(resolve(&builtin_actions(), &s, &ctx2).is_empty());
        s.general.paused = true;
        assert!(resolve(&builtin_actions(), &s, &MenuContext::default()).is_empty());
    }
}
