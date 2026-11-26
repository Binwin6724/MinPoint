//! Local unit conversion: `"5 km to mi"`, `"72 f in c"`, `"2.5 GB to MB"`.

use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Dim {
    Length,
    Mass,
    Volume,
    Time,
    Data,
    Speed,
    Area,
    Temperature,
}

struct Unit {
    names: &'static [&'static str],
    dim: Dim,
    /// Factor to the base unit (m, kg, L, s, byte, m/s, m²). Unused for temperature.
    factor: f64,
}

const UNITS: &[Unit] = &[
    Unit { names: &["mm", "millimeter", "millimeters", "millimetre"], dim: Dim::Length, factor: 0.001 },
    Unit { names: &["cm", "centimeter", "centimeters", "centimetre"], dim: Dim::Length, factor: 0.01 },
    Unit { names: &["m", "meter", "meters", "metre", "metres"], dim: Dim::Length, factor: 1.0 },
    Unit { names: &["km", "kilometer", "kilometers", "kilometre"], dim: Dim::Length, factor: 1000.0 },
    Unit { names: &["in", "inch", "inches", "\""], dim: Dim::Length, factor: 0.0254 },
    Unit { names: &["ft", "foot", "feet", "'"], dim: Dim::Length, factor: 0.3048 },
    Unit { names: &["yd", "yard", "yards"], dim: Dim::Length, factor: 0.9144 },
    Unit { names: &["mi", "mile", "miles"], dim: Dim::Length, factor: 1609.344 },
    Unit { names: &["nmi", "nauticalmile"], dim: Dim::Length, factor: 1852.0 },
    Unit { names: &["mg", "milligram", "milligrams"], dim: Dim::Mass, factor: 1e-6 },
    Unit { names: &["g", "gram", "grams"], dim: Dim::Mass, factor: 0.001 },
    Unit { names: &["kg", "kilogram", "kilograms", "kilo", "kilos"], dim: Dim::Mass, factor: 1.0 },
    Unit { names: &["t", "tonne", "tonnes"], dim: Dim::Mass, factor: 1000.0 },
    Unit { names: &["oz", "ounce", "ounces"], dim: Dim::Mass, factor: 0.028349523125 },
    Unit { names: &["lb", "lbs", "pound", "pounds"], dim: Dim::Mass, factor: 0.45359237 },
    Unit { names: &["st", "stone"], dim: Dim::Mass, factor: 6.35029318 },
    Unit { names: &["ml", "milliliter", "milliliters", "millilitre"], dim: Dim::Volume, factor: 0.001 },
    Unit { names: &["l", "liter", "liters", "litre", "litres"], dim: Dim::Volume, factor: 1.0 },
    Unit { names: &["gal", "gallon", "gallons"], dim: Dim::Volume, factor: 3.785411784 },
    Unit { names: &["qt", "quart", "quarts"], dim: Dim::Volume, factor: 0.946352946 },
    Unit { names: &["pt", "pint", "pints"], dim: Dim::Volume, factor: 0.473176473 },
    Unit { names: &["cup", "cups"], dim: Dim::Volume, factor: 0.2365882365 },
    Unit { names: &["floz"], dim: Dim::Volume, factor: 0.0295735295625 },
    Unit { names: &["ms", "millisecond", "milliseconds"], dim: Dim::Time, factor: 0.001 },
    Unit { names: &["s", "sec", "secs", "second", "seconds"], dim: Dim::Time, factor: 1.0 },
    Unit { names: &["min", "mins", "minute", "minutes"], dim: Dim::Time, factor: 60.0 },
    Unit { names: &["h", "hr", "hrs", "hour", "hours"], dim: Dim::Time, factor: 3600.0 },
    Unit { names: &["d", "day", "days"], dim: Dim::Time, factor: 86400.0 },
    Unit { names: &["wk", "week", "weeks"], dim: Dim::Time, factor: 604800.0 },
    Unit { names: &["b", "byte", "bytes"], dim: Dim::Data, factor: 1.0 },
    Unit { names: &["kb", "kilobyte", "kilobytes"], dim: Dim::Data, factor: 1e3 },
    Unit { names: &["mb", "megabyte", "megabytes"], dim: Dim::Data, factor: 1e6 },
    Unit { names: &["gb", "gigabyte", "gigabytes"], dim: Dim::Data, factor: 1e9 },
    Unit { names: &["tb", "terabyte", "terabytes"], dim: Dim::Data, factor: 1e12 },
    Unit { names: &["kib"], dim: Dim::Data, factor: 1024.0 },
    Unit { names: &["mib"], dim: Dim::Data, factor: 1048576.0 },
    Unit { names: &["gib"], dim: Dim::Data, factor: 1073741824.0 },
    Unit { names: &["tib"], dim: Dim::Data, factor: 1099511627776.0 },
    Unit { names: &["m/s", "mps"], dim: Dim::Speed, factor: 1.0 },
    Unit { names: &["km/h", "kmh", "kph"], dim: Dim::Speed, factor: 1000.0 / 3600.0 },
    Unit { names: &["mph"], dim: Dim::Speed, factor: 0.44704 },
    Unit { names: &["kn", "knot", "knots"], dim: Dim::Speed, factor: 1852.0 / 3600.0 },
    Unit { names: &["m2", "sqm"], dim: Dim::Area, factor: 1.0 },
    Unit { names: &["km2", "sqkm"], dim: Dim::Area, factor: 1e6 },
    Unit { names: &["ft2", "sqft"], dim: Dim::Area, factor: 0.09290304 },
    Unit { names: &["acre", "acres"], dim: Dim::Area, factor: 4046.8564224 },
    Unit { names: &["ha", "hectare", "hectares"], dim: Dim::Area, factor: 10000.0 },
    Unit { names: &["c", "°c", "celsius"], dim: Dim::Temperature, factor: 0.0 },
    Unit { names: &["f", "°f", "fahrenheit"], dim: Dim::Temperature, factor: 1.0 },
    Unit { names: &["k", "kelvin"], dim: Dim::Temperature, factor: 2.0 },
];

fn find(name: &str) -> Option<&'static Unit> {
    let n = name.trim().to_lowercase();
    UNITS.iter().find(|u| u.names.contains(&n.as_str()))
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Conversion {
    pub value: f64,
    pub from: String,
    pub to: String,
    pub result: f64,
}

fn to_celsius(v: f64, which: f64) -> f64 {
    match which as u8 {
        0 => v,
        1 => (v - 32.0) * 5.0 / 9.0,
        _ => v - 273.15,
    }
}
fn from_celsius(c: f64, which: f64) -> f64 {
    match which as u8 {
        0 => c,
        1 => c * 9.0 / 5.0 + 32.0,
        _ => c + 273.15,
    }
}

/// Convert between two named units of the same dimension.
pub fn convert(value: f64, from: &str, to: &str) -> Option<f64> {
    let a = find(from)?;
    let b = find(to)?;
    if a.dim != b.dim {
        return None;
    }
    if a.dim == Dim::Temperature {
        return Some(from_celsius(to_celsius(value, a.factor), b.factor));
    }
    Some(value * a.factor / b.factor)
}

/// Parse phrases like `"5 km to mi"` / `"5km in miles"` / `"100 f = c"`.
pub fn parse_and_convert(input: &str) -> Option<Conversion> {
    let lower = input.trim().to_lowercase();
    let (lhs, rhs) = [" to ", " in ", " as ", " = ", " -> "]
        .iter()
        .find_map(|sep| lower.split_once(sep))?;
    let lhs = lhs.trim();
    let split = lhs
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == '.' || *c == '-' || *c == ',' || *c == ' '))
        .map(|(i, _)| i)?;
    let num: f64 = lhs[..split].replace([',', ' '], "").parse().ok()?;
    let from = lhs[split..].trim();
    let to = rhs.trim();
    let result = convert(num, from, to)?;
    Some(Conversion { value: num, from: from.to_string(), to: to.to_string(), result })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        let c = parse_and_convert("5 km to mi").unwrap();
        assert!((c.result - 3.106855961).abs() < 1e-6);
        assert!((parse_and_convert("212 f in c").unwrap().result - 100.0).abs() < 1e-9);
        assert!((parse_and_convert("1 GiB to MB").unwrap().result - 1073.741824).abs() < 1e-6);
        assert!((parse_and_convert("90 min to hours").unwrap().result - 1.5).abs() < 1e-12);
        assert!(parse_and_convert("5 km to kg").is_none());
        assert!(parse_and_convert("hello").is_none());
    }
}
