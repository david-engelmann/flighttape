//! Python's repr, for the values a defect message can
//! name. The validator's texts quote values with
//! {value!r}; byte-identical messages need the same
//! rendering: single-quoted strings with Python's
//! escapes, True and False and None, list and dict
//! punctuation, and float repr's own presentation,
//! which is not the ECMAScript presentation of the
//! canonical form.

use crate::identity::render_number;
use crate::value::{Text, Value};

/// repr of a text value: the quote is a double quote
/// only when the text holds a single quote and no
/// double quote. Escapes follow CPython: the named
/// escapes for backslash, the quote in use, newline,
/// carriage return, and tab; \xXX, \uXXXX, and
/// \UXXXXXXXX for the rest of the non-printable.
pub fn repr_text(text: &Text) -> String {
    let units = text.units();
    let has_single = units.contains(&(b'\'' as u16));
    let has_double = units.contains(&(b'"' as u16));
    let quote = if has_single && !has_double { '"' } else { '\'' };
    let mut out = String::new();
    out.push(quote);
    // Walk the units as scalars where they pair up, so
    // printability is judged per character; an unpaired
    // surrogate is non-printable by construction.
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        let (scalar, width) = if (0xD800..0xDC00).contains(&unit)
            && index + 1 < units.len()
            && (0xDC00..0xE000).contains(&units[index + 1])
        {
            let value =
                0x10000u32 + (((unit as u32 - 0xD800) << 10) | (units[index + 1] as u32 - 0xDC00));
            (Some(value), 2)
        } else if (0xD800..0xE000).contains(&unit) {
            (None, 1)
        } else {
            (Some(unit as u32), 1)
        };
        index += width;
        match scalar {
            None => {
                out.push_str(&format!("\\u{:04x}", unit));
            }
            Some(0x5C) => out.push_str("\\\\"),
            Some(value) if value == quote as u32 => {
                out.push('\\');
                out.push(quote);
            }
            Some(0x0A) => out.push_str("\\n"),
            Some(0x0D) => out.push_str("\\r"),
            Some(0x09) => out.push_str("\\t"),
            Some(value) => {
                let ch = char::from_u32(value).expect("scalar is a char");
                if is_printable(ch) {
                    out.push(ch);
                } else if value <= 0xFF {
                    out.push_str(&format!("\\x{:02x}", value));
                } else if value <= 0xFFFF {
                    out.push_str(&format!("\\u{:04x}", value));
                } else {
                    out.push_str(&format!("\\U{:08x}", value));
                }
            }
        }
    }
    out.push(quote);
    out
}

/// CPython's str.isprintable, approximated on the
/// categories that matter for defect text: a character
/// is non-printable when it is a separator other than
/// the plain space, a control, a format character, a
/// combining mark in the enclosing classes, an
/// unassigned code point, or a private use character.
/// The tape vocabularies and the fixtures are ASCII;
/// this table covers the common non-printable ranges
/// and treats the rest as printable.
fn is_printable(ch: char) -> bool {
    let value = ch as u32;
    if value == 0x20 {
        return true;
    }
    if value < 0x20 || (0x7F..0xA0).contains(&value) {
        return false;
    }
    !matches!(value,
        0x00AD | 0x0600..=0x0605 | 0x061C | 0x06DD | 0x070F | 0x0890..=0x0891
        | 0x08E2 | 0x180E | 0x200B..=0x200F | 0x2028..=0x202E | 0x2060..=0x2064
        | 0x2066..=0x206F | 0x3000 | 0xFEFF | 0xFFF0..=0xFFFF
        | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D182 | 0xE000..=0xF8FF
        | 0xF0000..=0xFFFFF | 0x100000..=0x10FFFF)
        && !(0x202A..=0x202E).contains(&value)
        && !matches!(value, 0x00A0 | 0x1680 | 0x2000..=0x200A | 0x202F | 0x205F)
}

/// Python's float repr: the same shortest digits as
/// the canonical renderer, presented Python's way.
/// Fixed notation outside the scientific band, a
/// trailing .0 on integral fixed values, and a signed
/// exponent of at least two digits in scientific.
pub fn repr_float(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0".to_string()
        } else {
            "0.0".to_string()
        };
    }
    // render_number yields the ECMAScript presentation;
    // take its digits and exponent back out and present
    // them Python's way. The digits are the same by
    // construction (FT-52).
    let rendered = render_number(value.abs());
    let (digits, point) = digits_and_point(&rendered);
    let sign = if value < 0.0 { "-" } else { "" };
    let count = digits.len() as i64;
    let exp10 = point - 1;
    // CPython's band: scientific when the exponent is
    // below -4 or at least 16, fixed between.
    if !(-4..16).contains(&exp10) {
        let mantissa = if count == 1 {
            digits.clone()
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        return format!("{}{}e{:+03}", sign, mantissa, exp10);
    }
    fixed(sign, &digits, point)
}

fn fixed(sign: &str, digits: &str, point: i64) -> String {
    let count = digits.len() as i64;
    if point <= 0 {
        format!("{}0.{}{}", sign, "0".repeat((-point) as usize), digits)
    } else if count <= point {
        format!(
            "{}{}{}.0",
            sign,
            digits,
            "0".repeat((point - count) as usize)
        )
    } else {
        format!(
            "{}{}.{}",
            sign,
            &digits[..point as usize],
            &digits[point as usize..]
        )
    }
}

fn digits_and_point(rendered: &str) -> (String, i64) {
    // Inverse of the ECMAScript presentation: recover
    // the shortest digits and the point (the count of
    // digits before the decimal point).
    if let Some((mantissa, exponent)) = rendered.split_once('e') {
        let exp: i64 = exponent.parse().expect("exponent is an integer");
        let digits: String = mantissa.chars().filter(|ch| *ch != '.').collect();
        return (digits, exp + 1);
    }
    if let Some((whole, fraction)) = rendered.split_once('.') {
        if whole == "0" {
            let leading = fraction.chars().take_while(|ch| *ch == '0').count();
            let digits: String = fraction.trim_start_matches('0').to_string();
            return (digits, -(leading as i64));
        }
        let digits = format!("{}{}", whole, fraction);
        return (digits, whole.len() as i64);
    }
    (rendered.to_string(), rendered.len() as i64)
}

/// repr of any tape value.
pub fn repr_value(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::Int(digits) => digits.clone(),
        Value::Float(number) => repr_float(*number),
        Value::Str(text) => repr_text(text),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(repr_value).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Object(pairs) => {
            let parts: Vec<String> = pairs
                .iter()
                .map(|(key, item)| format!("{}: {}", repr_text(key), repr_value(item)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
    }
}
