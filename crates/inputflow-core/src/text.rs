//! Local, deterministic text transformations (no AI, no network).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextOp {
    Uppercase,
    Lowercase,
    TitleCase,
    SentenceCase,
    TrimWhitespace,
    RemoveDuplicateLines,
    SortLines,
    SortLinesDesc,
    ReverseText,
    ReverseLines,
    RemoveLineBreaks,
    CamelCase,
    PascalCase,
    SnakeCase,
    KebabCase,
    ConstantCase,
}

pub fn apply(op: TextOp, s: &str) -> String {
    match op {
        TextOp::Uppercase => s.to_uppercase(),
        TextOp::Lowercase => s.to_lowercase(),
        TextOp::TitleCase => title_case(s),
        TextOp::SentenceCase => sentence_case(s),
        TextOp::TrimWhitespace => trim_whitespace(s),
        TextOp::RemoveDuplicateLines => dedupe_lines(s),
        TextOp::SortLines => sort_lines(s, false),
        TextOp::SortLinesDesc => sort_lines(s, true),
        TextOp::ReverseText => s.chars().rev().collect(),
        TextOp::ReverseLines => s.lines().rev().collect::<Vec<_>>().join("\n"),
        TextOp::RemoveLineBreaks => remove_line_breaks(s),
        TextOp::CamelCase => naming(s, Naming::Camel),
        TextOp::PascalCase => naming(s, Naming::Pascal),
        TextOp::SnakeCase => naming(s, Naming::Snake),
        TextOp::KebabCase => naming(s, Naming::Kebab),
        TextOp::ConstantCase => naming(s, Naming::Constant),
    }
}

const SMALL_WORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "but", "by", "for", "in", "nor", "of", "on", "or", "per", "the", "to", "vs", "via",
];

fn capitalize(w: &str) -> String {
    let mut c = w.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
        None => String::new(),
    }
}

pub fn title_case(s: &str) -> String {
    s.lines()
        .map(|line| {
            let words: Vec<&str> = line.split(' ').collect();
            let last = words.len().saturating_sub(1);
            words
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    let lw = w.to_lowercase();
                    if i != 0 && i != last && SMALL_WORDS.contains(&lw.as_str()) {
                        lw
                    } else {
                        capitalize(w)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn sentence_case(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut cap_next = true;
    for ch in lower.chars() {
        if cap_next && ch.is_alphabetic() {
            out.extend(ch.to_uppercase());
            cap_next = false;
        } else {
            out.push(ch);
        }
        if matches!(ch, '.' | '!' | '?' | '\n') {
            cap_next = true;
        }
    }
    // Restore standalone "i".
    out.split(' ')
        .map(|w| if w == "i" { "I" } else { w })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn trim_whitespace(s: &str) -> String {
    s.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub fn dedupe_lines(s: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    s.lines().filter(|l| seen.insert(*l)).collect::<Vec<_>>().join("\n")
}

pub fn sort_lines(s: &str, desc: bool) -> String {
    let mut v: Vec<&str> = s.lines().collect();
    v.sort_by_key(|l| l.to_lowercase());
    if desc {
        v.reverse();
    }
    v.join("\n")
}

pub fn remove_line_breaks(s: &str) -> String {
    s.split(['\n', '\r']).map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ")
}

pub fn find_replace(s: &str, find: &str, replace: &str, case_sensitive: bool, use_regex: bool) -> Result<String, String> {
    if find.is_empty() {
        return Ok(s.to_string());
    }
    let pattern = if use_regex { find.to_string() } else { regex::escape(find) };
    let re = regex::RegexBuilder::new(&pattern)
        .case_insensitive(!case_sensitive)
        .size_limit(1 << 20)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(re.replace_all(s, replace).into_owned())
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TextStats {
    pub characters: usize,
    pub characters_no_spaces: usize,
    pub words: usize,
    pub lines: usize,
    pub sentences: usize,
    pub reading_seconds: u64,
}

pub fn stats(s: &str) -> TextStats {
    let words = s.split_whitespace().count();
    TextStats {
        characters: s.chars().count(),
        characters_no_spaces: s.chars().filter(|c| !c.is_whitespace()).count(),
        words,
        lines: if s.is_empty() { 0 } else { s.lines().count() },
        sentences: s.split(['.', '!', '?']).filter(|p| p.trim().chars().any(char::is_alphanumeric)).count(),
        reading_seconds: ((words as f64 / 238.0) * 60.0).round() as u64,
    }
}

enum Naming {
    Camel,
    Pascal,
    Snake,
    Kebab,
    Constant,
}

/// Split identifiers / prose into words: handles camelCase, PascalCase, snake_case, kebab-case, spaces, acronyms.
pub fn split_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            continue;
        }
        if c.is_uppercase() && !cur.is_empty() {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).map_or(false, |n| n.is_lowercase());
            if prev.is_lowercase() || prev.is_ascii_digit() || (prev.is_uppercase() && next_lower) {
                words.push(std::mem::take(&mut cur));
            }
        }
        cur.push(c);
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words.into_iter().map(|w| w.to_lowercase()).collect()
}

fn naming(s: &str, n: Naming) -> String {
    let w = split_words(s);
    match n {
        Naming::Camel => w
            .iter()
            .enumerate()
            .map(|(i, x)| if i == 0 { x.clone() } else { capitalize(x) })
            .collect(),
        Naming::Pascal => w.iter().map(|x| capitalize(x)).collect(),
        Naming::Snake => w.join("_"),
        Naming::Kebab => w.join("-"),
        Naming::Constant => w.join("_").to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases() {
        assert_eq!(apply(TextOp::TitleCase, "the lord of the rings"), "The Lord of the Rings");
        assert_eq!(apply(TextOp::SentenceCase, "HELLO WORLD. i AM here"), "Hello world. I am here");
        assert_eq!(apply(TextOp::TrimWhitespace, "  a   b \n  c  "), "a b\nc");
        assert_eq!(apply(TextOp::RemoveDuplicateLines, "a\nb\na\nc"), "a\nb\nc");
        assert_eq!(apply(TextOp::SortLines, "b\nA\nc"), "A\nb\nc");
        assert_eq!(apply(TextOp::RemoveLineBreaks, "a\n b\r\n\nc"), "a b c");
    }

    #[test]
    fn naming_conventions() {
        assert_eq!(split_words("parseHTTPResponse"), vec!["parse", "http", "response"]);
        assert_eq!(apply(TextOp::CamelCase, "user_account id"), "userAccountId");
        assert_eq!(apply(TextOp::PascalCase, "user-account-id"), "UserAccountId");
        assert_eq!(apply(TextOp::SnakeCase, "UserAccountID"), "user_account_id");
        assert_eq!(apply(TextOp::KebabCase, "userAccountId"), "user-account-id");
        assert_eq!(apply(TextOp::ConstantCase, "max retry count"), "MAX_RETRY_COUNT");
    }

    #[test]
    fn find_and_replace() {
        assert_eq!(find_replace("Foo foo", "foo", "bar", false, false).unwrap(), "bar bar");
        assert_eq!(find_replace("a.b", ".", "-", true, false).unwrap(), "a-b");
        assert_eq!(find_replace("a1b22", r"\d+", "#", true, true).unwrap(), "a#b#");
        assert!(find_replace("x", "(", "", true, true).is_err());
    }

    #[test]
    fn counting() {
        let s = stats("Hello world. How are you?\nFine.");
        assert_eq!(s.words, 6);
        assert_eq!(s.lines, 2);
        assert_eq!(s.sentences, 3);
    }
}
