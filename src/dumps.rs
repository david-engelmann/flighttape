//! Python's json.dumps, the two shapes the command
//! line emits: indent=2 documents, and the compact
//! sort_keys rendering the playback uses for tool
//! inputs and extra fields. Both escape non-ASCII the
//! way ensure_ascii does by default, render floats by
//! their Python repr, and sort object keys by code
//! point, as sorted keys are sorted in Python.

use crate::pyrepr::repr_float;
use crate::value::{scalar_cmp, Text, Value};

fn escape_string(text: &Text, out: &mut String) {
    out.push('"');
    let units = text.units();
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        let (scalar, width) = if (0xD800..0xDC00).contains(&unit)
            && index + 1 < units.len()
            && (0xDC00..0xE000).contains(&units[index + 1])
        {
            (
                0x10000u32 + (((unit as u32 - 0xD800) << 10) | (units[index + 1] as u32 - 0xDC00)),
                2,
            )
        } else {
            (unit as u32, 1)
        };
        index += width;
        match scalar {
            0x22 => out.push_str("\\\""),
            0x5C => out.push_str("\\\\"),
            0x08 => out.push_str("\\b"),
            0x09 => out.push_str("\\t"),
            0x0A => out.push_str("\\n"),
            0x0C => out.push_str("\\f"),
            0x0D => out.push_str("\\r"),
            0x20..=0x7E => {
                out.push(char::from_u32(scalar).expect("ascii is a char"));
            }
            0x00..=0xFFFF => {
                out.push_str(&format!("\\u{:04x}", scalar));
            }
            _ => {
                let adjusted = scalar - 0x10000;
                out.push_str(&format!(
                    "\\u{:04x}\\u{:04x}",
                    0xD800 + (adjusted >> 10),
                    0xDC00 + (adjusted & 0x3FF)
                ));
            }
        }
    }
    out.push('"');
}

fn write_scalar(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(digits) => out.push_str(digits),
        Value::Float(number) => {
            if number.is_nan() {
                out.push_str("NaN");
            } else if number.is_infinite() {
                out.push_str(if *number > 0.0 {
                    "Infinity"
                } else {
                    "-Infinity"
                });
            } else {
                out.push_str(&repr_float(*number));
            }
        }
        Value::Str(text) => escape_string(text, out),
        Value::Array(_) | Value::Object(_) => {
            unreachable!("containers are written by the callers")
        }
    }
}

fn sorted_pairs(pairs: &[(Text, Value)]) -> Vec<&(Text, Value)> {
    let mut ordered: Vec<&(Text, Value)> = pairs.iter().collect();
    ordered.sort_by(|a, b| scalar_cmp(&a.0, &b.0));
    ordered
}

/// json.dumps(value, indent=2).
pub fn dumps_indent(value: &Value) -> String {
    let mut out = String::new();
    write_indented(value, 0, &mut out);
    out
}

fn write_indented(value: &Value, level: usize, out: &mut String) {
    match value {
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&"  ".repeat(level + 1));
                write_indented(item, level + 1, out);
            }
            out.push('\n');
            out.push_str(&"  ".repeat(level));
            out.push(']');
        }
        Value::Object(pairs) => {
            if pairs.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (index, (key, item)) in pairs.iter().enumerate() {
                if index > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&"  ".repeat(level + 1));
                escape_string(key, out);
                out.push_str(": ");
                write_indented(item, level + 1, out);
            }
            out.push('\n');
            out.push_str(&"  ".repeat(level));
            out.push('}');
        }
        _ => write_scalar(value, out),
    }
}

/// json.dumps(value, sort_keys=True) with the default
/// separators.
pub fn dumps_sorted(value: &Value) -> String {
    let mut out = String::new();
    write_sorted(value, &mut out);
    out
}

fn write_sorted(value: &Value, out: &mut String) {
    match value {
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                write_sorted(item, out);
            }
            out.push(']');
        }
        Value::Object(pairs) => {
            out.push('{');
            for (index, (key, item)) in sorted_pairs(pairs).iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                escape_string(key, out);
                out.push_str(": ");
                write_sorted(item, out);
            }
            out.push('}');
        }
        _ => write_scalar(value, out),
    }
}
