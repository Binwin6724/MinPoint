//! Snippet / text-expansion engine.
//!
//! Privacy design: the trigger buffer only ever holds the last N typed characters,
//! where N is the length of the longest *enabled* trigger. It is cleared on any
//! navigation key, mouse click, focus change or secure-input state, and it is
//! never persisted or logged. It is not a keystroke log.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: String,
    pub name: String,
    /// Abbreviation such as `;prdone`. Empty = manual insertion only.
    pub trigger: String,
    pub content: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Bundle IDs / exe names where this snippet is allowed. Empty = everywhere.
    #[serde(default)]
    pub apps: Vec<String>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, PartialEq, Clone)]
pub struct TriggerMatch {
    pub snippet_id: String,
    /// Number of characters to delete (the typed trigger) before inserting.
    pub backspaces: usize,
}

pub struct TriggerBuffer {
    buf: VecDeque<char>,
    cap: usize,
    triggers: Vec<(String, String, bool, Vec<String>)>, // (trigger, id, case_sensitive, apps)
}

impl TriggerBuffer {
    pub fn new(snippets: &[Snippet]) -> Self {
        let mut tb = TriggerBuffer { buf: VecDeque::new(), cap: 0, triggers: vec![] };
        tb.reload(snippets);
        tb
    }

    pub fn reload(&mut self, snippets: &[Snippet]) {
        self.triggers = snippets
            .iter()
            .filter(|s| s.enabled && s.trigger.chars().count() >= 2)
            .map(|s| (s.trigger.clone(), s.id.clone(), s.case_sensitive, s.apps.clone()))
            .collect();
        // Longest first so `;addr2` wins over `;addr`.
        self.triggers.sort_by_key(|t| std::cmp::Reverse(t.0.chars().count()));
        self.cap = self.triggers.iter().map(|t| t.0.chars().count()).max().unwrap_or(0);
        self.clear();
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn backspace(&mut self) {
        self.buf.pop_back();
    }

    /// Feed one printable character. `app` is the frontmost app identifier.
    pub fn push(&mut self, c: char, app: &str) -> Option<TriggerMatch> {
        if self.cap == 0 {
            return None;
        }
        self.buf.push_back(c);
        while self.buf.len() > self.cap {
            self.buf.pop_front();
        }
        let typed: String = self.buf.iter().collect();
        for (trig, id, cs, apps) in &self.triggers {
            if !apps.is_empty() && !apps.iter().any(|a| a.eq_ignore_ascii_case(app)) {
                continue;
            }
            let hit = if *cs { typed.ends_with(trig.as_str()) } else { typed.to_lowercase().ends_with(&trig.to_lowercase()) };
            if hit {
                let m = TriggerMatch { snippet_id: id.clone(), backspaces: trig.chars().count() };
                self.clear();
                return Some(m);
            }
        }
        None
    }
}

/// Values available to dynamic variables at expansion time.
pub trait VariableSource {
    fn clipboard(&self) -> Option<String>;
    fn username(&self) -> Option<String>;
    /// Ask the user for a value (returns None if cancelled or unavailable).
    fn user_input(&self, label: &str) -> Option<String>;
}

#[derive(Debug, PartialEq)]
pub struct Expansion {
    pub text: String,
    /// Character offset where the cursor should end up, if `{cursor}` was used.
    pub cursor: Option<usize>,
}

/// Expand `{date}`, `{date:%d %b}`, `{time}`, `{datetime}`, `{clipboard}`, `{username}`,
/// `{counter:name}`, `{input:Label}`, `{cursor}`. `{{` and `}}` escape braces.
/// Unknown placeholders are left untouched.
pub fn expand(content: &str, src: &dyn VariableSource, counters: &mut HashMap<String, i64>, now: chrono::DateTime<chrono::Local>) -> Expansion {
    let mut out = String::new();
    let mut cursor = None;
    let mut inputs: HashMap<String, String> = HashMap::new();
    let mut rest = content;
    while let Some(i) = rest.find(['{', '}']) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        if tail.starts_with("{{") || tail.starts_with("}}") {
            out.push(tail.chars().next().unwrap());
            rest = &tail[2..];
            continue;
        }
        if tail.starts_with('}') {
            out.push('}');
            rest = &tail[1..];
            continue;
        }
        let Some(end) = tail.find('}') else {
            out.push_str(tail);
            rest = "";
            break;
        };
        let token = &tail[1..end];
        let (name, arg) = token.split_once(':').map_or((token, None), |(a, b)| (a, Some(b)));
        let fmt = |f: &str| {
            // Guard against invalid strftime specifiers (chrono panics on Display otherwise).
            let items: Vec<_> = chrono::format::StrftimeItems::new(f).collect();
            if items.iter().any(|i| matches!(i, chrono::format::Item::Error)) {
                None
            } else {
                Some(now.format_with_items(items.into_iter()).to_string())
            }
        };
        let value: Option<String> = match name.trim() {
            "date" => fmt(arg.unwrap_or("%Y-%m-%d")),
            "time" => fmt(arg.unwrap_or("%H:%M")),
            "datetime" => fmt(arg.unwrap_or("%Y-%m-%d %H:%M")),
            "clipboard" => Some(src.clipboard().unwrap_or_default()),
            "username" => Some(src.username().unwrap_or_default()),
            "counter" => {
                let key = arg.unwrap_or("default").to_string();
                let c = counters.entry(key).or_insert(0);
                *c += 1;
                Some(c.to_string())
            }
            "input" => {
                let label = arg.unwrap_or("Value").to_string();
                if let Some(v) = inputs.get(&label) {
                    Some(v.clone())
                } else {
                    let v = src.user_input(&label).unwrap_or_default();
                    inputs.insert(label, v.clone());
                    Some(v)
                }
            }
            "cursor" => {
                cursor = Some(out.chars().count());
                Some(String::new())
            }
            _ => None,
        };
        match value {
            Some(v) => out.push_str(&v),
            None => out.push_str(&tail[..=end]),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    Expansion { text: out, cursor }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    struct Src;
    impl VariableSource for Src {
        fn clipboard(&self) -> Option<String> {
            Some("CLIP".into())
        }
        fn username(&self) -> Option<String> {
            Some("sam".into())
        }
        fn user_input(&self, label: &str) -> Option<String> {
            Some(format!("<{label}>"))
        }
    }

    fn snip(id: &str, trigger: &str) -> Snippet {
        Snippet {
            id: id.into(),
            name: id.into(),
            trigger: trigger.into(),
            content: String::new(),
            category: String::new(),
            case_sensitive: false,
            favorite: false,
            enabled: true,
            apps: vec![],
        }
    }

    #[test]
    fn buffer_detects_triggers_and_stays_bounded() {
        let mut b = TriggerBuffer::new(&[snip("pr", ";prdone"), snip("ad", ";ad"), snip("ad2", ";ad2")]);
        for c in "hello world ".chars() {
            assert!(b.push(c, "app").is_none());
            assert!(b.len() <= 7);
        }
        let mut hit = None;
        for c in ";PRdone".chars() {
            hit = b.push(c, "app");
        }
        assert_eq!(hit, Some(TriggerMatch { snippet_id: "pr".into(), backspaces: 7 }));
        assert_eq!(b.len(), 0, "buffer cleared after a match");
        // ";ad" fires before "2" can be typed: shorter trigger wins at the moment it completes.
        let r: Vec<_> = ";ad".chars().filter_map(|c| b.push(c, "app")).collect();
        assert_eq!(r[0].snippet_id, "ad");
    }

    #[test]
    fn app_scoping_and_case_sensitivity() {
        let mut s = snip("x", ";Sig");
        s.case_sensitive = true;
        s.apps = vec!["com.tinyspeck.slackmacgap".into()];
        let mut b = TriggerBuffer::new(&[s]);
        assert!(";Sig".chars().filter_map(|c| b.push(c, "com.apple.TextEdit")).next().is_none());
        assert!(";sig".chars().filter_map(|c| b.push(c, "com.tinyspeck.slackmacgap")).next().is_none());
        assert!(";Sig".chars().filter_map(|c| b.push(c, "com.tinyspeck.slackmacgap")).next().is_some());
    }

    #[test]
    fn variables() {
        let now = chrono::Local.with_ymd_and_hms(2026, 3, 4, 9, 5, 0).unwrap();
        let mut counters = HashMap::new();
        let e = expand("{date} {time} {date:%d/%m} {clipboard} {username} #{counter:t}#{counter:t} {input:Name}/{input:Name} {{x}} {nope}{cursor}!", &Src, &mut counters, now);
        assert_eq!(e.text, "2026-03-04 09:05 04/03 CLIP sam #1#2 <Name>/<Name> {x} {nope}!");
        assert_eq!(e.cursor, Some(e.text.chars().count() - 1));
        let bad = expand("{date:%Q}", &Src, &mut counters, now);
        assert_eq!(bad.text, "{date:%Q}");
    }
}
