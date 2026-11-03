//! Validate a flight tape against the flighttape
//! Contract. The published schema states the shape of
//! a tape; this validator enforces that shape and the
//! structural rules JSON Schema cannot state, namely
//! consecutive seq values, tool call pairing, and
//! chronological timestamp order, exactly as FT-1
//! divides the work. It also names every non-finite
//! number in the tape (FT-29). It returns every defect
//! it finds in one pass, never just the first (FT-2).

use std::collections::HashSet;

use crate::pyrepr::repr_value;
use crate::timeparse::parse_timestamp;
use crate::value::{int_cmp, Text, Value};
use crate::TAPE_VERSION;

pub const EVENT_KINDS: [&str; 8] = [
    "session_start",
    "message",
    "tool_call",
    "tool_result",
    "diff",
    "spend",
    "note",
    "session_end",
];

pub const OUTCOMES: [&str; 4] = ["completed", "failed", "aborted", "unknown"];

pub const BASES: [&str; 5] = [
    "final-assistant-message",
    "final-error-result",
    "user-interrupt-marker",
    "no-events",
    "unresolved-ending",
];

fn bases_for_status(status: &str) -> Option<&'static [&'static str]> {
    match status {
        "completed" => Some(&["final-assistant-message"]),
        "failed" => Some(&["final-error-result"]),
        "aborted" => Some(&["user-interrupt-marker"]),
        "unknown" => Some(&["no-events", "unresolved-ending"]),
        _ => None,
    }
}

const OUTCOMES_REPR: &str = "['aborted', 'completed', 'failed', 'unknown']";
const BASES_REPR: &str = "['final-assistant-message', 'final-error-result', \
    'no-events', 'unresolved-ending', 'user-interrupt-marker']";

/// True for an int that is not a bool; Value has no
/// bool-int overlap, so this is the Int variant.
fn is_int(value: &Value) -> bool {
    value.is_int()
}

fn is_negative_int(value: &Value) -> bool {
    match value {
        Value::Int(digits) => int_cmp(digits, "0") == std::cmp::Ordering::Less,
        _ => false,
    }
}

/// A tape's duration in seconds (FT-32), or None. The
/// span between the first and last parseable event
/// timestamps, None when fewer than two events carry
/// one. Shared by the comparison and rollup reports.
pub fn tape_duration(tape: &Value) -> Option<f64> {
    let events = tape.get("events")?.as_array()?;
    let mut stamps: Vec<i128> = Vec::new();
    for event in events {
        if let Some(Value::Str(text)) = event.get("timestamp") {
            if let Some(rendered) = text.as_str() {
                if let Some(parsed) = parse_timestamp(&rendered) {
                    stamps.push(parsed);
                }
            }
        }
    }
    if stamps.len() < 2 {
        return None;
    }
    let micros = stamps[stamps.len() - 1] - stamps[0];
    Some(micros as f64 / 1_000_000.0)
}

fn key_text(text: &Text) -> String {
    match text.as_str() {
        Some(rendered) => rendered,
        None => String::from_utf16_lossy(text.units()),
    }
}

/// Name every non-finite number in the tape, wherever
/// it sits (FT-29). The walk is iterative and visits
/// children in document order. Paths are rooted at
/// the field, with the bare root named tape.
fn nonfinite_defects(tape: &Value) -> Vec<String> {
    let mut defects = Vec::new();
    let mut stack: Vec<(&Value, String)> = vec![(tape, String::new())];
    while let Some((value, path)) = stack.pop() {
        match value {
            Value::Float(number) => {
                if !number.is_finite() {
                    defects.push(format!(
                        "{} is a non-finite number",
                        if path.is_empty() {
                            "tape".to_string()
                        } else {
                            path.clone()
                        }
                    ));
                }
            }
            Value::Object(pairs) => {
                let mut children: Vec<(&Value, String)> = Vec::new();
                for (key, item) in pairs {
                    let child_path = if path.is_empty() {
                        key_text(key)
                    } else {
                        format!("{}.{}", path, key_text(key))
                    };
                    children.push((item, child_path));
                }
                for child in children.into_iter().rev() {
                    stack.push(child);
                }
            }
            Value::Array(items) => {
                for index in (0..items.len()).rev() {
                    stack.push((&items[index], format!("{}[{}]", path, index)));
                }
            }
            _ => {}
        }
    }
    defects
}

/// Return a list of defect strings. An empty list
/// means valid. The non-finite walk runs on every
/// path, including the structural early returns.
pub fn validate_tape(tape: &Value) -> Vec<String> {
    let mut defects = validate_structure(tape);
    defects.extend(nonfinite_defects(tape));
    defects
}

fn none_repr() -> String {
    "None".to_string()
}

fn repr_of(value: Option<&Value>) -> String {
    match value {
        Some(value) => repr_value(value),
        None => none_repr(),
    }
}

fn validate_structure(tape: &Value) -> Vec<String> {
    let mut defects: Vec<String> = Vec::new();
    let tape_pairs = match tape.as_object() {
        Some(pairs) => pairs,
        None => return vec!["tape is not a JSON object".to_string()],
    };
    let _ = tape_pairs;
    if tape.get("tape_version") != Some(&Value::Str(Text::from_str(TAPE_VERSION))) {
        defects.push(format!(
            "tape_version must be '{}', got {}",
            TAPE_VERSION,
            repr_of(tape.get("tape_version"))
        ));
    }
    let empty_object = Value::Object(Vec::new());
    let run = match tape.get("run") {
        Some(run @ Value::Object(_)) => run.clone(),
        _ => {
            defects.push("run is missing or is not an object".to_string());
            empty_object.clone()
        }
    };
    for field in ["harness", "session_id"] {
        let valid = matches!(
            run.get(field),
            Some(Value::Str(text)) if !text.units().is_empty()
        );
        if !valid {
            defects.push(format!("run.{} must be a non-empty string", field));
        }
    }
    for field in ["source_format", "started_at"] {
        if let Some(value) = run.get(field) {
            if !value.is_null() && !matches!(value, Value::Str(_)) {
                defects.push(format!("run.{} must be a string when present", field));
            }
        }
    }
    for field in ["skipped_lines", "totals"] {
        if let Some(value) = run.get(field) {
            if value.is_null() {
                continue;
            }
            match value.as_object() {
                None => defects.push(format!("run.{} must be an object when present", field)),
                Some(pairs) => {
                    for (key, entry) in pairs {
                        if !is_int(entry) || is_negative_int(entry) {
                            defects.push(format!(
                                "run.{}.{} must be a non-negative integer",
                                field,
                                key_text(key)
                            ));
                        }
                    }
                }
            }
        }
    }
    let outcome = tape.get("outcome");
    match outcome {
        Some(outcome @ Value::Object(_)) => {
            let status = outcome.get("status");
            let status_text = match status {
                Some(Value::Str(text)) => text.as_str(),
                _ => None,
            };
            let status_invalid = match &status_text {
                None => true,
                Some(name) => !OUTCOMES.contains(&name.as_str()),
            };
            if status_invalid {
                defects.push(format!(
                    "outcome.status must be one of {}, got {}",
                    OUTCOMES_REPR,
                    repr_of(status)
                ));
            }
            if let Some(basis) = outcome.get("basis") {
                let basis_text = match basis {
                    Value::Str(text) => text.as_str(),
                    _ => None,
                };
                match basis_text {
                    Some(name) if BASES.contains(&name.as_str()) => {
                        if let Some(status_name) = status_text {
                            if let Some(allowed) = bases_for_status(&status_name) {
                                if !allowed.contains(&name.as_str()) {
                                    defects.push(format!(
                                        "outcome.basis {} does not go with \
                                         outcome.status {}",
                                        repr_value(basis),
                                        repr_of(status)
                                    ));
                                }
                            }
                        }
                    }
                    _ => {
                        defects.push(format!(
                            "outcome.basis must be one of {}, got {}",
                            BASES_REPR,
                            repr_value(basis)
                        ));
                    }
                }
            }
        }
        _ => defects.push("outcome is missing or is not an object".to_string()),
    }
    let events = match tape.get("events") {
        Some(Value::Array(items)) => items.clone(),
        _ => {
            defects.push("events is missing or is not a list".to_string());
            return defects;
        }
    };
    if events.is_empty() {
        defects.push("events must hold at least one event".to_string());
    }
    let mut seen_calls: HashSet<Text> = HashSet::new();
    let mut last_ts: Option<i128> = None;
    for (want_seq, event) in events.iter().enumerate() {
        let where_ = format!("events[{}]", want_seq);
        let event_pairs = match event.as_object() {
            Some(pairs) => pairs,
            None => {
                defects.push(format!("{} is not an object", where_));
                continue;
            }
        };
        let _ = event_pairs;
        let seq = event.get("seq");
        let seq_ok = match seq {
            Some(Value::Int(digits)) => {
                int_cmp(digits, &want_seq.to_string()) == std::cmp::Ordering::Equal
            }
            _ => false,
        };
        if !seq_ok {
            defects.push(format!(
                "{}.seq must be {}, got {}",
                where_,
                want_seq,
                repr_of(seq)
            ));
        }
        let kind = event.get("kind");
        let kind_text = match kind {
            Some(Value::Str(text)) => Some(text.clone()),
            _ => None,
        };
        let kind_name = kind_text.as_ref().and_then(|text| text.as_str());
        let kind_invalid = match &kind_name {
            None => true,
            Some(name) => !EVENT_KINDS.contains(&name.as_str()),
        };
        if kind_invalid {
            defects.push(format!(
                "{}.kind {} is not a tape event kind",
                where_,
                repr_of(kind)
            ));
        }
        if !matches!(event.get("sidechain"), Some(Value::Bool(_))) {
            defects.push(format!("{}.sidechain must be a boolean", where_));
        }
        let call_id = event.get("call_id");
        if let Some(call_id) = call_id {
            if !call_id.is_null() && !matches!(call_id, Value::Str(_)) {
                defects.push(format!("{}.call_id must be a string", where_));
            }
        }
        if let Some(ts) = event.get("timestamp") {
            if !ts.is_null() {
                match ts {
                    Value::Str(text) => match text.as_str() {
                        Some(rendered) => match parse_timestamp(&rendered) {
                            None => defects.push(format!(
                                "{}.timestamp {} is not ISO 8601",
                                where_,
                                repr_value(ts)
                            )),
                            Some(parsed) => {
                                if let Some(last) = last_ts {
                                    if parsed < last {
                                        defects.push(format!("{}.timestamp runs backward", where_));
                                    } else {
                                        last_ts = Some(parsed);
                                    }
                                } else {
                                    last_ts = Some(parsed);
                                }
                            }
                        },
                        None => defects.push(format!(
                            "{}.timestamp {} is not ISO 8601",
                            where_,
                            repr_value(ts)
                        )),
                    },
                    _ => defects.push(format!("{}.timestamp must be a string", where_)),
                }
            }
        }
        let call_text = match call_id {
            Some(Value::Str(text)) => Some(text.clone()),
            _ => None,
        };
        if kind_name.as_deref() == Some("tool_call") {
            match &call_text {
                Some(text) if !text.units().is_empty() => {
                    if seen_calls.contains(text) {
                        defects.push(format!(
                            "{}.call_id {} is used twice",
                            where_,
                            repr_of(call_id)
                        ));
                    } else {
                        seen_calls.insert(text.clone());
                    }
                }
                _ => defects.push(format!("{}.call_id must be a non-empty string", where_)),
            }
            let tool_ok = matches!(
                event.get("tool"),
                Some(Value::Str(text)) if !text.units().is_empty()
            );
            if !tool_ok {
                defects.push(format!("{}.tool must be a non-empty string", where_));
            }
        }
        if kind_name.as_deref() == Some("tool_result") {
            let answers = match &call_text {
                Some(text) => seen_calls.contains(text),
                None => false,
            };
            if !answers {
                defects.push(format!(
                    "{}.call_id {} answers no earlier tool_call in this tape",
                    where_,
                    repr_of(call_id)
                ));
            }
            if let Some(content) = event.get("content") {
                if !content.is_null() && !matches!(content, Value::Str(_)) {
                    defects.push(format!("{}.content must be a string", where_));
                }
            }
            if let Some(is_error) = event.get("is_error") {
                if !matches!(is_error, Value::Bool(_)) {
                    defects.push(format!("{}.is_error must be a boolean", where_));
                }
            }
        }
    }
    defects
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}
