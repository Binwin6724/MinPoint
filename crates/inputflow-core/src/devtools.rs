//! Developer utilities. All local and deterministic.

use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DevOp {
    JsonFormat,
    JsonMinify,
    JsonValidate,
    Base64Encode,
    Base64Decode,
    UrlEncode,
    UrlDecode,
    Sha256,
    Sha1,
    Md5,
    TimestampToDate,
    DateToTimestamp,
    ColorConvert,
    CsvToMarkdownTable,
    XmlFormat,
}

pub fn apply(op: DevOp, s: &str) -> Result<String, String> {
    match op {
        DevOp::JsonFormat => json_format(s, true),
        DevOp::JsonMinify => json_format(s, false),
        DevOp::JsonValidate => match serde_json::from_str::<serde_json::Value>(s) {
            Ok(_) => Ok("Valid JSON".into()),
            Err(e) => Err(format!("Invalid JSON at line {}, column {}: {}", e.line(), e.column(), e)),
        },
        DevOp::Base64Encode => Ok(base64::engine::general_purpose::STANDARD.encode(s.as_bytes())),
        DevOp::Base64Decode => {
            let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&cleaned)
                .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(cleaned.trim_end_matches('=')))
                .map_err(|e| format!("Invalid Base64: {e}"))?;
            String::from_utf8(bytes).map_err(|_| "Decoded data is not UTF-8 text".into())
        }
        DevOp::UrlEncode => Ok(urlencoding::encode(s).into_owned()),
        DevOp::UrlDecode => urlencoding::decode(s).map(|c| c.into_owned()).map_err(|e| e.to_string()),
        DevOp::Sha256 => Ok(hex(&<sha2::Sha256 as sha2::Digest>::digest(s.as_bytes()))),
        DevOp::Sha1 => Ok(hex(&<sha1::Sha1 as sha1::Digest>::digest(s.as_bytes()))),
        DevOp::Md5 => Ok(hex(&<md5::Md5 as md5::Digest>::digest(s.as_bytes()))),
        DevOp::TimestampToDate => timestamp_to_date(s),
        DevOp::DateToTimestamp => date_to_timestamp(s),
        DevOp::ColorConvert => color_convert(s),
        DevOp::CsvToMarkdownTable => Ok(csv_to_markdown(s)),
        DevOp::XmlFormat => xml_format(s),
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn json_format(s: &str, pretty: bool) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(s).map_err(|e| format!("Invalid JSON: {e}"))?;
    if pretty {
        serde_json::to_string_pretty(&v).map_err(|e| e.to_string())
    } else {
        serde_json::to_string(&v).map_err(|e| e.to_string())
    }
}

pub fn uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn timestamp_to_date(s: &str) -> Result<String, String> {
    let n: i64 = s.trim().parse().map_err(|_| "Not a Unix timestamp".to_string())?;
    // Heuristic: 13 digits => milliseconds.
    let (secs, label) = if n.abs() > 99_999_999_999 { (n / 1000, "ms") } else { (n, "s") };
    let dt = chrono::DateTime::from_timestamp(secs, 0).ok_or("Timestamp out of range")?;
    let local = dt.with_timezone(&chrono::Local);
    Ok(format!("{} UTC\n{} local\n(interpreted as {label})", dt.format("%Y-%m-%d %H:%M:%S"), local.format("%Y-%m-%d %H:%M:%S %:z")))
}

fn date_to_timestamp(s: &str) -> Result<String, String> {
    let t = s.trim();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(t) {
        return Ok(dt.timestamp().to_string());
    }
    for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M"] {
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(t, fmt) {
            return Ok(ndt.and_utc().timestamp().to_string() + " (assuming UTC)");
        }
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(t, "%Y-%m-%d") {
        return Ok(d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp().to_string() + " (00:00 UTC)");
    }
    Err("Unrecognised date. Use RFC 3339 or YYYY-MM-DD [HH:MM[:SS]].".into())
}

/// Difference between two dates (YYYY-MM-DD) in days.
pub fn date_difference(a: &str, b: &str) -> Result<i64, String> {
    let p = |x: &str| chrono::NaiveDate::parse_from_str(x.trim(), "%Y-%m-%d").map_err(|e| e.to_string());
    Ok((p(b)? - p(a)?).num_days())
}

fn color_convert(s: &str) -> Result<String, String> {
    let t = s.trim().to_lowercase();
    let (r, g, b) = if let Some(h) = t.strip_prefix('#') {
        let h = if h.len() == 3 { h.chars().flat_map(|c| [c, c]).collect::<String>() } else { h.to_string() };
        if h.len() != 6 {
            return Err("Hex colours must have 3 or 6 digits".into());
        }
        let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).map_err(|e| e.to_string());
        (p(0)?, p(2)?, p(4)?)
    } else if let Some(inner) = t.strip_prefix("rgb(").and_then(|x| x.strip_suffix(')')) {
        let v: Vec<u8> = inner.split(',').map(|x| x.trim().parse::<u8>()).collect::<Result<_, _>>().map_err(|e| e.to_string())?;
        if v.len() != 3 {
            return Err("rgb() needs three components".into());
        }
        (v[0], v[1], v[2])
    } else {
        return Err("Enter a colour like #3b82f6 or rgb(59, 130, 246)".into());
    };
    let (h, s_, l) = rgb_to_hsl(r, g, b);
    Ok(format!("#{r:02x}{g:02x}{b:02x}\nrgb({r}, {g}, {b})\nhsl({h:.0}, {:.0}%, {:.0}%)", s_ * 100.0, l * 100.0))
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let (r, g, b) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max == min {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) * 60.0
    } else if max == g {
        ((b - r) / d + 2.0) * 60.0
    } else {
        ((r - g) / d + 4.0) * 60.0
    };
    (h, s, l)
}

/// Minimal CSV (RFC 4180 quoting) to a Markdown table.
fn csv_to_markdown(s: &str) -> String {
    let rows: Vec<Vec<String>> = s.lines().filter(|l| !l.trim().is_empty()).map(parse_csv_line).collect();
    if rows.is_empty() {
        return String::new();
    }
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let fmt = |r: &Vec<String>| {
        let cells: Vec<String> = (0..cols).map(|i| r.get(i).cloned().unwrap_or_default().replace('|', "\\|")).collect();
        format!("| {} |", cells.join(" | "))
    };
    let mut out = vec![fmt(&rows[0]), format!("|{}|", vec![" --- "; cols].join("|"))];
    out.extend(rows[1..].iter().map(fmt));
    out.join("\n")
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            ('"', _) => quoted = !quoted,
            (',', false) => out.push(std::mem::take(&mut cur).trim().to_string()),
            _ => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

/// Very small XML pretty-printer (indentation only; does not validate schemas).
fn xml_format(s: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut depth: i32 = 0;
    let mut rest = s.trim();
    while !rest.is_empty() {
        if let Some(start) = rest.find('<') {
            let text = rest[..start].trim();
            if !text.is_empty() {
                out.push_str(&"  ".repeat(depth.max(0) as usize));
                out.push_str(text);
                out.push('\n');
            }
            let end = rest[start..].find('>').ok_or("Unclosed tag")? + start;
            let tag = &rest[start..=end];
            if tag.starts_with("</") {
                depth -= 1;
            }
            out.push_str(&"  ".repeat(depth.max(0) as usize));
            out.push_str(tag);
            out.push('\n');
            if !tag.starts_with("</") && !tag.ends_with("/>") && !tag.starts_with("<?") && !tag.starts_with("<!") {
                depth += 1;
            }
            rest = &rest[end + 1..];
        } else {
            out.push_str(rest.trim());
            break;
        }
    }
    if depth != 0 {
        return Err("Unbalanced tags".into());
    }
    // Collapse <a>\n  text\n</a> into one line for leaf elements.
    let re = regex::Regex::new(r"(?m)^(\s*)(<[^/!?][^>]*[^/]>|<[^/!?]>)\n\s*([^<\n]+)\n\s*(</[^>]+>)$").unwrap();
    Ok(re.replace_all(out.trim_end(), "$1$2$3$4").into_owned())
}

/// Regex tester: returns all matches with capture groups.
#[derive(Debug, Serialize)]
pub struct RegexMatch {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub groups: Vec<Option<String>>,
}

pub fn regex_test(pattern: &str, haystack: &str, case_insensitive: bool) -> Result<Vec<RegexMatch>, String> {
    let re = regex::RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .size_limit(1 << 20)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(re
        .captures_iter(haystack)
        .take(500)
        .map(|c| {
            let m = c.get(0).unwrap();
            RegexMatch {
                start: m.start(),
                end: m.end(),
                text: m.as_str().to_string(),
                groups: c.iter().skip(1).map(|g| g.map(|g| g.as_str().to_string())).collect(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json() {
        assert_eq!(apply(DevOp::JsonMinify, "{ \"a\": [1, 2] }").unwrap(), "{\"a\":[1,2]}");
        assert!(apply(DevOp::JsonFormat, "{\"a\":1}").unwrap().contains("\n  \"a\": 1"));
        assert!(apply(DevOp::JsonValidate, "{bad}").is_err());
    }

    #[test]
    fn encodings_and_hashes() {
        assert_eq!(apply(DevOp::Base64Encode, "hello").unwrap(), "aGVsbG8=");
        assert_eq!(apply(DevOp::Base64Decode, "aGVsbG8=").unwrap(), "hello");
        assert_eq!(apply(DevOp::UrlEncode, "a b&c").unwrap(), "a%20b%26c");
        assert_eq!(apply(DevOp::UrlDecode, "a%20b").unwrap(), "a b");
        assert_eq!(apply(DevOp::Sha256, "abc").unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(apply(DevOp::Md5, "abc").unwrap(), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(uuid_v4().len(), 36);
    }

    #[test]
    fn time_and_color() {
        assert!(apply(DevOp::TimestampToDate, "0").unwrap().starts_with("1970-01-01 00:00:00 UTC"));
        assert_eq!(apply(DevOp::DateToTimestamp, "1970-01-02T00:00:00Z").unwrap(), "86400");
        assert_eq!(date_difference("2026-01-01", "2026-03-01").unwrap(), 59);
        let c = apply(DevOp::ColorConvert, "#fff").unwrap();
        assert!(c.contains("rgb(255, 255, 255)") && c.contains("hsl(0, 0%, 100%)"));
    }

    #[test]
    fn csv_xml_regex() {
        let md = apply(DevOp::CsvToMarkdownTable, "name,qty\n\"a, b\",2").unwrap();
        assert_eq!(md, "| name | qty |\n| --- | --- |\n| a, b | 2 |");
        assert_eq!(apply(DevOp::XmlFormat, "<a><b>x</b><c/></a>").unwrap(), "<a>\n  <b>x</b>\n  <c/>\n</a>");
        let m = regex_test(r"(\d+)", "a1 b22", false).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[1].groups[0].as_deref(), Some("22"));
    }
}
