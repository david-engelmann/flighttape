//! Python's repr, for the values a defect message can
//! name. The validator's texts quote values with
//! {value!r}; byte-identical messages need the same
//! rendering: single-quoted strings with Python's
//! escapes, True and False and None, list and dict
//! punctuation, and float repr's own presentation,
//! which is not the ECMAScript presentation of the
//! canonical form.

use crate::identity::shortest_digits;
use crate::printable::is_printable;
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
    // The digits are the shortest computation's own
    // (FT-58): a presentation is not a digit sequence,
    // and the ECMAScript integer presentation pads an
    // integral value with zeros past its shortest
    // digits, zeros a repr in the scientific band must
    // not inherit.
    let (digits, exp10) = shortest_digits(value.abs());
    let point = exp10 + 1;
    let sign = if value < 0.0 { "-" } else { "" };
    let count = digits.len() as i64;
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

#[cfg(test)]
mod tests {
    use super::{repr_float, repr_text, repr_value};
    use crate::dumps::{dumps_indent, dumps_sorted};
    use crate::printable::is_printable;
    use crate::value::{Text, Value};

    #[test]
    fn the_repr_corpus_agrees_with_cpython() {
        // FT-58: the corpus is generated from the
        // interpreter (CPython 3.12.3) over the number
        // sweep's bit patterns, the band edges at
        // exponents -5 to 17, and the padding shapes,
        // integral values whose shortest digits are
        // short and whose presentations are long. The
        // recorded instance heads the family: its
        // ECMAScript presentation pads eleven shortest
        // digits with zeros the repr must not inherit.
        assert_eq!(repr_float(1000000000500000000.0), "1.0000000005e+18");
        let corpus = include_str!("../tests/fixtures/repr_corpus.txt");
        let mut checked = 0;
        for line in corpus.lines() {
            let (bits, expected) = line.split_once(' ').unwrap();
            let value = f64::from_bits(u64::from_str_radix(bits, 16).unwrap());
            assert_eq!(repr_float(value), expected, "bits {}", bits);
            assert_eq!(repr_value(&Value::Float(value)), expected, "bits {}", bits);
            assert_eq!(
                dumps_sorted(&Value::Float(value)),
                expected,
                "bits {}",
                bits
            );
            assert_eq!(
                dumps_indent(&Value::Float(value)),
                expected,
                "bits {}",
                bits
            );
            checked += 1;
        }
        assert_eq!(checked, 6576, "the corpus carries {} lines", checked);
    }

    #[test]
    fn printability_agrees_with_the_generated_table() {
        // FT-59: the table was generated from CPython
        // 3.12.3 (Unicode 15.0.0) over the whole
        // codepoint space. The sweep counts the
        // non-printable scalar values: 963,066, the
        // interpreter's count with the surrogate range
        // (unrepresentable as a char) removed. A drift
        // in the table moves the count.
        let mut non_printable = 0usize;
        for value in 0..=0x10FFFFu32 {
            if let Some(ch) = char::from_u32(value) {
                if !is_printable(ch) {
                    non_printable += 1;
                }
            }
        }
        assert_eq!(non_printable, 963_066);
        // The four recorded disagreements of the hand
        // approximation, named: unassigned codepoints
        // the interpreter treats as non-printable.
        for point in [0x0378u32, 0x0379, 0x1AFF, 0xE0001] {
            let ch = char::from_u32(point).unwrap();
            assert!(!is_printable(ch), "U+{:04X} must be non-printable", point);
        }
    }

    #[test]
    fn the_recorded_unassigned_points_escape_in_repr() {
        assert_eq!(repr_text(&Text::from_str("\u{378}")), "'\\u0378'");
        assert_eq!(repr_text(&Text::from_str("\u{379}")), "'\\u0379'");
        assert_eq!(repr_text(&Text::from_str("\u{1AFF}")), "'\\u1aff'");
        assert_eq!(repr_text(&Text::from_str("\u{E0001}")), "'\\U000e0001'");
    }
}
