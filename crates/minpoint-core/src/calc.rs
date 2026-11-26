//! Safe arithmetic expression evaluator.
//!
//! A hand-written recursive-descent parser. It never executes code: it only
//! understands numbers, a fixed set of operators, named constants and a fixed
//! allow-list of math functions.
//!
//! Grammar (lowest to highest precedence):
//! ```text
//! expr    := term (('+' | '-') term)*
//! term    := unary (('*' | '/' | '%' | implicit-mul) unary)*
//! unary   := ('-' | '+') unary | power
//! power   := postfix ('^' unary)?
//! postfix := primary ('!' | '%')*
//! primary := number | ident '(' expr (',' expr)* ')' | ident | '(' expr ')'
//! ```
//! Also supports the phrase form `X% of Y`.

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum CalcError {
    #[error("unexpected character '{0}'")]
    UnexpectedChar(char),
    #[error("unexpected end of expression")]
    UnexpectedEnd,
    #[error("unexpected token")]
    UnexpectedToken,
    #[error("unknown identifier '{0}'")]
    UnknownIdent(String),
    #[error("wrong number of arguments for '{0}'")]
    Arity(String),
    #[error("division by zero")]
    DivisionByZero,
    #[error("result is not a finite number")]
    NotFinite,
    #[error("expression too long")]
    TooLong,
}

const MAX_LEN: usize = 512;
const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
    Comma,
}

fn tokenize(s: &str) -> Result<Vec<Tok>, CalcError> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).map_or(false, |d| d.is_ascii_digit())) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == ',' && chars.get(i + 1).map_or(false, |d| d.is_ascii_digit()) && is_thousands_sep(&chars, i)) {
                i += 1;
            }
            // exponent
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    i = j;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let text: String = chars[start..i].iter().filter(|c| **c != ',').collect();
            let n: f64 = text.parse().map_err(|_| CalcError::UnexpectedChar(c))?;
            out.push(Tok::Num(n));
            continue;
        }
        if c.is_alphabetic() {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric()) {
                i += 1;
            }
            let word = chars[start..i].iter().collect::<String>().to_lowercase();
            // "3 x 4" means multiplication.
            out.push(if word == "x" { Tok::Op('*') } else { Tok::Ident(word) });
            continue;
        }
        let t = match c {
            '+' | '-' | '/' | '^' | '%' | '!' => Tok::Op(c),
            '*' | '×' => Tok::Op('*'),
            '÷' => Tok::Op('/'),
            '−' => Tok::Op('-'),
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            ',' => Tok::Comma,
            other => return Err(CalcError::UnexpectedChar(other)),
        };
        out.push(t);
        i += 1;
    }
    Ok(out)
}

/// A comma between digits counts as a thousands separator only when followed by exactly 3 digits.
fn is_thousands_sep(chars: &[char], i: usize) -> bool {
    let after: Vec<&char> = chars[i + 1..].iter().take(4).collect();
    after.len() >= 3 && after[..3].iter().all(|c| c.is_ascii_digit()) && after.get(3).map_or(true, |c| !c.is_ascii_digit())
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    fn enter(&mut self) -> Result<(), CalcError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(CalcError::TooLong);
        }
        Ok(())
    }

    fn expr(&mut self) -> Result<f64, CalcError> {
        self.enter()?;
        let mut v = self.term()?;
        while let Some(Tok::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.pos += 1;
            let rhs = self.term()?;
            v = if op == '+' { v + rhs } else { v - rhs };
        }
        self.depth -= 1;
        Ok(v)
    }

    fn term(&mut self) -> Result<f64, CalcError> {
        let mut v = self.unary()?;
        loop {
            match self.peek().cloned() {
                Some(Tok::Op(op @ ('*' | '/'))) => {
                    self.pos += 1;
                    let rhs = self.unary()?;
                    if op == '*' {
                        v *= rhs
                    } else {
                        if rhs == 0.0 {
                            return Err(CalcError::DivisionByZero);
                        }
                        v /= rhs
                    }
                }
                // implicit multiplication: 2(3), 2pi, (1+2)(3)
                Some(Tok::LParen) | Some(Tok::Ident(_)) | Some(Tok::Num(_)) => {
                    let rhs = self.unary()?;
                    v *= rhs;
                }
                _ => break,
            }
        }
        Ok(v)
    }

    fn unary(&mut self) -> Result<f64, CalcError> {
        self.enter()?;
        let r = match self.peek() {
            Some(Tok::Op('-')) => {
                self.pos += 1;
                Ok(-self.unary()?)
            }
            Some(Tok::Op('+')) => {
                self.pos += 1;
                self.unary()
            }
            _ => self.power(),
        };
        self.depth -= 1;
        r
    }

    fn power(&mut self) -> Result<f64, CalcError> {
        let base = self.postfix()?;
        if let Some(Tok::Op('^')) = self.peek() {
            self.pos += 1;
            let exp = self.unary()?; // right-associative
            return Ok(base.powf(exp));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> Result<f64, CalcError> {
        let mut v = self.primary()?;
        loop {
            match self.peek() {
                Some(Tok::Op('!')) => {
                    self.pos += 1;
                    v = factorial(v)?;
                }
                Some(Tok::Op('%')) => {
                    // "%" followed by "of" is handled in primary/expr via evaluate(); here it's percent.
                    self.pos += 1;
                    if let Some(Tok::Ident(id)) = self.peek() {
                        if id == "of" {
                            self.pos += 1;
                            let rhs = self.unary()?;
                            v = v / 100.0 * rhs;
                            continue;
                        }
                    }
                    v /= 100.0;
                }
                _ => break,
            }
        }
        Ok(v)
    }

    fn primary(&mut self) -> Result<f64, CalcError> {
        match self.next() {
            Some(Tok::Num(n)) => Ok(n),
            Some(Tok::LParen) => {
                let v = self.expr()?;
                match self.next() {
                    Some(Tok::RParen) => Ok(v),
                    None => Err(CalcError::UnexpectedEnd),
                    _ => Err(CalcError::UnexpectedToken),
                }
            }
            Some(Tok::Ident(name)) => {
                if let Some(Tok::LParen) = self.peek() {
                    self.pos += 1;
                    let mut args = vec![self.expr()?];
                    while let Some(Tok::Comma) = self.peek() {
                        self.pos += 1;
                        args.push(self.expr()?);
                    }
                    match self.next() {
                        Some(Tok::RParen) => {}
                        None => return Err(CalcError::UnexpectedEnd),
                        _ => return Err(CalcError::UnexpectedToken),
                    }
                    call(&name, &args)
                } else {
                    constant(&name)
                }
            }
            None => Err(CalcError::UnexpectedEnd),
            _ => Err(CalcError::UnexpectedToken),
        }
    }
}

fn factorial(v: f64) -> Result<f64, CalcError> {
    if v < 0.0 || v.fract() != 0.0 || v > 170.0 {
        return Err(CalcError::NotFinite);
    }
    Ok((1..=v as u64).fold(1.0, |acc, k| acc * k as f64))
}

fn constant(name: &str) -> Result<f64, CalcError> {
    match name {
        "pi" | "π" => Ok(std::f64::consts::PI),
        "e" => Ok(std::f64::consts::E),
        "tau" => Ok(std::f64::consts::TAU),
        _ => Err(CalcError::UnknownIdent(name.to_string())),
    }
}

fn call(name: &str, a: &[f64]) -> Result<f64, CalcError> {
    let one = |f: fn(f64) -> f64| -> Result<f64, CalcError> {
        if a.len() != 1 {
            return Err(CalcError::Arity(name.to_string()));
        }
        Ok(f(a[0]))
    };
    match name {
        "sqrt" => one(f64::sqrt),
        "cbrt" => one(f64::cbrt),
        "abs" => one(f64::abs),
        "sin" => one(f64::sin),
        "cos" => one(f64::cos),
        "tan" => one(f64::tan),
        "asin" => one(f64::asin),
        "acos" => one(f64::acos),
        "atan" => one(f64::atan),
        "ln" => one(f64::ln),
        "log" | "log10" => one(f64::log10),
        "log2" => one(f64::log2),
        "exp" => one(f64::exp),
        "floor" => one(f64::floor),
        "ceil" => one(f64::ceil),
        "round" => one(f64::round),
        "deg" => one(f64::to_degrees),
        "rad" => one(f64::to_radians),
        "min" if !a.is_empty() => Ok(a.iter().cloned().fold(f64::INFINITY, f64::min)),
        "max" if !a.is_empty() => Ok(a.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
        "pow" if a.len() == 2 => Ok(a[0].powf(a[1])),
        "min" | "max" | "pow" => Err(CalcError::Arity(name.to_string())),
        _ => Err(CalcError::UnknownIdent(name.to_string())),
    }
}

/// Evaluate an arithmetic expression safely.
pub fn evaluate(input: &str) -> Result<f64, CalcError> {
    if input.len() > MAX_LEN {
        return Err(CalcError::TooLong);
    }
    let trimmed = input.trim().trim_end_matches('=').trim();
    let toks = tokenize(trimmed)?;
    if toks.is_empty() {
        return Err(CalcError::UnexpectedEnd);
    }
    let mut p = Parser { toks, pos: 0, depth: 0 };
    let v = p.expr()?;
    if p.pos != p.toks.len() {
        return Err(CalcError::UnexpectedToken);
    }
    if !v.is_finite() {
        return Err(CalcError::NotFinite);
    }
    Ok(v)
}

/// Format a number for display: integers without decimals, otherwise up to 10 significant decimals.
pub fn format_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    let s = format!("{:.10}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str) -> f64 {
        evaluate(s).unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn arithmetic() {
        assert_eq!(ev("1 + 2 * 3"), 7.0);
        assert_eq!(ev("(1 + 2) * 3"), 9.0);
        assert_eq!(ev("2 ^ 3 ^ 2"), 512.0);
        assert_eq!(ev("-2 ^ 2"), -4.0);
        assert_eq!(ev("10 / 4"), 2.5);
        assert_eq!(ev("2(3+4)"), 14.0);
        assert_eq!(ev("5!"), 120.0);
        assert_eq!(ev("1,000 + 1"), 1001.0);
        assert_eq!(ev("1.5e3"), 1500.0);
        assert_eq!(ev("12 × 3 ="), 36.0);
    }

    #[test]
    fn percent() {
        assert_eq!(ev("20% of 150"), 30.0);
        assert_eq!(ev("50%"), 0.5);
        assert_eq!(ev("200 * 15%"), 30.0);
    }

    #[test]
    fn functions_and_constants() {
        assert!((ev("sqrt(16)") - 4.0).abs() < 1e-12);
        assert!((ev("2pi") - std::f64::consts::TAU).abs() < 1e-12);
        assert_eq!(ev("max(1, 9, 3)"), 9.0);
        assert!((ev("log(1000)") - 3.0).abs() < 1e-12);
    }

    #[test]
    fn errors() {
        assert_eq!(evaluate("1/0"), Err(CalcError::DivisionByZero));
        assert!(matches!(evaluate("rm -rf"), Err(CalcError::UnknownIdent(_))));
        assert!(evaluate("2 +").is_err());
        assert!(evaluate("(((1)").is_err());
        assert!(evaluate(&"(".repeat(200)).is_err());
        assert!(evaluate("system(1)").is_err());
    }

    #[test]
    fn formatting() {
        assert_eq!(format_number(3.0), "3");
        assert_eq!(format_number(0.1 + 0.2), "0.3");
    }
}
