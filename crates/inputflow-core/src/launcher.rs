//! Quick-launcher search: fuzzy scoring and instant answers (calculator / units).

use serde::Serialize;

/// Subsequence fuzzy score. Higher is better; None = no match.
/// Rewards prefix matches, word-boundary matches and consecutive runs.
pub fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    let q: Vec<char> = query.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
    if q.is_empty() {
        return Some(0);
    }
    let c: Vec<char> = candidate.to_lowercase().chars().collect();
    let orig: Vec<char> = candidate.chars().collect();
    let mut score = 0;
    let mut qi = 0;
    let mut last: Option<usize> = None;
    for (i, ch) in c.iter().enumerate() {
        if qi < q.len() && *ch == q[qi] {
            let boundary = i == 0 || !c[i - 1].is_alphanumeric() || (orig[i].is_uppercase() && orig.get(i.wrapping_sub(1)).map_or(false, |p| p.is_lowercase()));
            score += 10;
            if boundary {
                score += 15;
            }
            if i == 0 {
                score += 20;
            }
            if let Some(l) = last {
                if l + 1 == i {
                    score += 12;
                } else {
                    score -= (i - l) as i32;
                }
            }
            last = Some(i);
            qi += 1;
        }
    }
    if qi < q.len() {
        return None;
    }
    // Prefer shorter candidates.
    Some(score - (c.len() as i32 / 4))
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstantAnswer {
    pub kind: &'static str,
    pub display: String,
    /// Value to copy when the user hits Enter.
    pub value: String,
}

pub fn instant_answer(query: &str) -> Option<InstantAnswer> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }
    if let Some(c) = crate::units::parse_and_convert(q) {
        let v = crate::calc::format_number((c.result * 1e6).round() / 1e6);
        return Some(InstantAnswer { kind: "unit", display: format!("{} {} = {} {}", crate::calc::format_number(c.value), c.from, v, c.to), value: v });
    }
    // Only treat as a calculation if it contains an operator/function, so plain numbers or words don't trigger.
    let looks_math = q.chars().any(|c| "+-*/^%×÷!(".contains(c)) || q.contains(" of ");
    if looks_math {
        if let Ok(v) = crate::calc::evaluate(q) {
            let s = crate::calc::format_number(v);
            return Some(InstantAnswer { kind: "calc", display: format!("= {s}"), value: s });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy() {
        assert!(fuzzy_score("vsc", "Visual Studio Code").is_some());
        assert!(fuzzy_score("xyz", "Safari").is_none());
        let a = fuzzy_score("saf", "Safari").unwrap();
        let b = fuzzy_score("saf", "Microsoft Defender").unwrap_or(i32::MIN);
        assert!(a > b);
        assert!(fuzzy_score("pol", "Polish Text").unwrap() > fuzzy_score("pol", "Explain Code Logic").unwrap_or(i32::MIN));
    }

    #[test]
    fn answers() {
        assert_eq!(instant_answer("2+2").unwrap().value, "4");
        assert_eq!(instant_answer("10 km to mi").unwrap().kind, "unit");
        assert!(instant_answer("safari").is_none());
        assert!(instant_answer("42").is_none());
    }
}
