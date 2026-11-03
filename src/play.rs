//! Play a flight tape back as a readable timeline
//! (FT-10 to FT-12). The header also states the
//! outcome basis the tape carries (FT-17), read
//! defensively. The rendering is a pure function of
//! the tape: the same tape always renders byte
//! identical output, and the test suite holds the
//! rendering to a stored golden file built from the
//! synthetic fixture.

use crate::convert::TOKEN_KEYS;
use crate::dumps::dumps_sorted;
use crate::timeparse::parse_timestamp;
use crate::value::{int_add, py_str, scalar_cmp, truthy, Text, Value};

const COUNTED: [(&str, &str); 2] = [("tool_call", "tool calls"), ("message", "messages")];
const KNOWN_FIELDS: [&str; 11] = [
    "seq",
    "kind",
    "sidechain",
    "timestamp",
    "call_id",
    "tool",
    "role",
    "text",
    "content",
    "input",
    "is_error",
];

fn offset(stamp: Option<i128>, origin: Option<i128>) -> String {
    match (stamp, origin) {
        (Some(stamp), Some(origin)) => {
            let delta = (stamp - origin) as f64 / 1_000_000.0;
            if delta.abs() < 10.0 {
                format!("{:+.2}s", delta)
            } else {
                format!("{:+.1}s", delta)
            }
        }
        _ => "no time".to_string(),
    }
}

fn field_text(value: Option<&Value>) -> Option<String> {
    // The f-string rendering of a header field: the
    // value itself when a string, its Python str
    // otherwise; a missing field reads unknown at the
    // call sites that pass the default.
    value.map(|value| match value {
        Value::Str(text) => text
            .as_str()
            .unwrap_or_else(|| String::from_utf16_lossy(text.units())),
        _ => {
            let text = py_str(value);
            text.as_str()
                .unwrap_or_else(|| String::from_utf16_lossy(text.units()))
        }
    })
}

fn header_lines(tape: &Value) -> Vec<String> {
    let empty = Value::Object(Vec::new());
    let run = tape.get("run").unwrap_or(&empty);
    let run = if run.as_object().is_some() {
        run
    } else {
        &empty
    };
    let totals = match run.get("totals") {
        Some(totals) if truthy(totals) => totals.clone(),
        _ => empty.clone(),
    };
    let events = tape
        .get("events")
        .cloned()
        .unwrap_or(Value::Array(Vec::new()));
    let event_items = events.as_array().cloned().unwrap_or_default();
    let mut parts: Vec<String> = vec![format!("events {}", event_items.len())];
    for (kind, label) in COUNTED {
        let counted = event_items
            .iter()
            .filter(|event| event.get("kind") == Some(&Value::Str(Text::from_str(kind))))
            .count();
        parts.push(format!("{} {}", label, counted));
    }
    if let Some(totals_map) = totals.as_object() {
        for key in TOKEN_KEYS {
            if let Some(value) = totals_map
                .iter()
                .find(|(name, _)| name == &key)
                .map(|(_, value)| value)
            {
                let rendered = field_text(Some(value)).unwrap_or_default();
                parts.push(format!("{} {}", key.replace('_', " "), rendered));
            }
        }
    }
    let outcome = tape.get("outcome");
    let outcome_map = match outcome {
        Some(outcome @ Value::Object(_)) => Some(outcome.clone()),
        _ => None,
    };
    let outcome_text = match outcome_map
        .as_ref()
        .and_then(|outcome| outcome.get("status"))
    {
        Some(Value::Str(status)) => {
            let mut text = status
                .as_str()
                .unwrap_or_else(|| String::from_utf16_lossy(status.units()));
            if let Some(Value::Str(basis)) = outcome_map
                .as_ref()
                .and_then(|outcome| outcome.get("basis"))
            {
                if !basis.units().is_empty() {
                    text.push_str(&format!(
                        " ({})",
                        basis
                            .as_str()
                            .unwrap_or_else(|| { String::from_utf16_lossy(basis.units()) })
                    ));
                }
            }
            text
        }
        _ => "unknown".to_string(),
    };
    let mut lines = vec![
        format!(
            "tape {} from {}",
            field_text(run.get("session_id")).unwrap_or_else(|| "unknown".to_string()),
            field_text(run.get("harness")).unwrap_or_else(|| "unknown".to_string())
        ),
        format!("outcome {}, {}", outcome_text, parts.join(", ")),
    ];
    let mut meta: Vec<String> = Vec::new();
    if let Some(started) = run.get("started_at") {
        if truthy(started) {
            meta.push(format!(
                "started {}",
                field_text(Some(started)).unwrap_or_default()
            ));
        }
    }
    if let Some(source) = run.get("source_format") {
        if truthy(source) {
            meta.push(format!(
                "source {}",
                field_text(Some(source)).unwrap_or_default()
            ));
        }
    }
    let skipped = match run.get("skipped_lines") {
        Some(skipped) if truthy(skipped) => Some(skipped.clone()),
        _ => None,
    };
    if let Some(skipped) = skipped {
        if let Some(pairs) = skipped.as_object() {
            if !pairs.is_empty() {
                let mut ordered: Vec<&(Text, Value)> = pairs.iter().collect();
                ordered.sort_by(|a, b| scalar_cmp(&a.0, &b.0));
                let detail: Vec<String> = ordered
                    .iter()
                    .map(|(key, value)| {
                        format!(
                            "{} {}",
                            key.as_str()
                                .unwrap_or_else(|| { String::from_utf16_lossy(key.units()) }),
                            field_text(Some(value)).unwrap_or_default()
                        )
                    })
                    .collect();
                let mut total = "0".to_string();
                for (_, value) in pairs {
                    if let Value::Int(digits) = value {
                        total = int_add(&total, digits);
                    }
                }
                meta.push(format!("skipped lines {} ({})", total, detail.join(", ")));
            }
        }
    }
    if let Some(redacted) = run.get("redacted_values") {
        if truthy(redacted) {
            meta.push(format!(
                "redacted values {}",
                field_text(Some(redacted)).unwrap_or_default()
            ));
        }
    }
    if !meta.is_empty() {
        lines.push(meta.join(", "));
    }
    lines.push(String::new());
    lines
}

/// Render one validated tape as timeline text.
pub fn render_tape(tape: &Value) -> String {
    let mut lines = header_lines(tape);
    let empty = Value::Array(Vec::new());
    let events = tape.get("events").unwrap_or(&empty);
    let event_items = events.as_array().cloned().unwrap_or_default();
    let stamps: Vec<Option<i128>> = event_items
        .iter()
        .map(|event| match event.get("timestamp") {
            Some(Value::Str(text)) => text
                .as_str()
                .and_then(|rendered| parse_timestamp(&rendered)),
            _ => None,
        })
        .collect();
    let origin = stamps.iter().flatten().next().copied();
    for (event, stamp) in event_items.iter().zip(stamps.iter()) {
        let kind_value = event.get("kind");
        let kind = match kind_value {
            Some(Value::Str(text)) => text
                .as_str()
                .unwrap_or_else(|| String::from_utf16_lossy(text.units())),
            Some(_) => field_text(kind_value).unwrap_or_default(),
            None => "note".to_string(),
        };
        let side = if truthy(event.get("sidechain").unwrap_or(&Value::Null)) {
            " sidechain"
        } else {
            ""
        };
        let seq = field_text(event.get("seq")).unwrap_or_else(|| "?".to_string());
        let mut head = format!("[{}] #{} {}{}", offset(*stamp, origin), seq, kind, side);
        let mut body: Option<Value> = None;
        if kind == "message" {
            head.push_str(&format!(
                " {}",
                field_text(event.get("role")).unwrap_or_else(|| "unknown".to_string())
            ));
            body = event.get("text").cloned();
        } else if kind == "tool_call" {
            head.push_str(&format!(
                " {} call {}",
                field_text(event.get("tool")).unwrap_or_else(|| "unknown".to_string()),
                field_text(event.get("call_id")).unwrap_or_else(|| "unknown".to_string())
            ));
            if let Some(input) = event.get("input") {
                if !input.is_null() {
                    body = Some(Value::Str(Text::from_str(&dumps_sorted(input))));
                }
            }
        } else if kind == "tool_result" {
            head.push_str(&format!(
                " call {}",
                field_text(event.get("call_id")).unwrap_or_else(|| "unknown".to_string())
            ));
            if truthy(event.get("is_error").unwrap_or(&Value::Null)) {
                head.push_str(" ERROR");
            }
            body = event.get("content").cloned();
        } else {
            body = event
                .get("content")
                .cloned()
                .or_else(|| event.get("text").cloned());
        }
        lines.push(head);
        if let Some(body) = &body {
            if truthy(body) {
                lines.push(match body {
                    Value::Str(text) => text
                        .as_str()
                        .unwrap_or_else(|| String::from_utf16_lossy(text.units())),
                    _ => field_text(Some(body)).unwrap_or_default(),
                });
            }
        }
        if let Some(pairs) = event.as_object() {
            let extra: Vec<(Text, Value)> = pairs
                .iter()
                .filter(|(key, _)| !KNOWN_FIELDS.iter().any(|known| key == known))
                .cloned()
                .collect();
            if !extra.is_empty() {
                lines.push(dumps_sorted(&Value::Object(extra)));
            }
        }
    }
    let mut rendered = lines.join("\n");
    rendered.push('\n');
    rendered
}
