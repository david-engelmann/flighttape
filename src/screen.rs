//! Tape screening (FT-44 to FT-48). Conversion redacts
//! every absolute path out of the text it emits;
//! screening is the same rule read in the other
//! direction. A tape is walked over exactly the
//! strings FT-22 names as text, each run through the
//! converter's own path counter, and every string that
//! holds a path is reported as a hit: the event's seq,
//! the field, whether the paths sat in a key or in a
//! value, and how many paths the string holds. The
//! report never carries a screened string or any part
//! of a path it found.

use crate::convert::paths_in;
use crate::identity::tape_digest;
use crate::value::{object, Text, Value};

const TEXT_FIELDS: [(&str, &str); 2] =
    [("message", "text"), ("tool_result", "content")];

struct Screening {
    hits: Vec<Value>,
    strings: u64,
    paths: u64,
}

fn screen_string(
    text: &Text,
    seq: &Value,
    field: &str,
    where_: &str,
    state: &mut Screening,
) {
    state.strings += 1;
    let found = paths_in(text);
    if found > 0 {
        state.paths += found;
        state.hits.push(object(vec![
            ("seq", seq.clone()),
            ("field", Value::Str(Text::from_str(field))),
            ("in", Value::Str(Text::from_str(where_))),
            ("paths", Value::Int(found.to_string())),
        ]));
    }
}

/// Screen every string in a carrier value (FT-45,
/// FT-45a). The walk is iterative (FT-45b): an
/// explicit stack, so no nesting depth a valid tape
/// can hold exhausts a call stack. Object keys are
/// screened as keys; the values under them as values.
fn screen_value(
    value: Option<&Value>,
    seq: &Value,
    field: &str,
    state: &mut Screening,
) {
    let value = match value {
        None => return,
        Some(value) => value,
    };
    enum Frame {
        Object(Vec<(Text, Value)>, usize),
        Array(Vec<Value>, usize),
    }
    fn frame_of(node: &Value) -> Option<Frame> {
        match node {
            Value::Object(pairs) => Some(Frame::Object(pairs.clone(), 0)),
            Value::Array(items) => Some(Frame::Array(items.clone(), 0)),
            _ => None,
        }
    }
    match value {
        Value::Str(text) => {
            let text = text.clone();
            screen_string(&text, seq, field, "value", state);
        }
        Value::Array(_) | Value::Object(_) => {
            let mut stack: Vec<Frame> = vec![frame_of(value).unwrap()];
            while !stack.is_empty() {
                let top = stack.len() - 1;
                let child: Option<Value> = match &mut stack[top] {
                    Frame::Object(pairs, index) => {
                        if *index < pairs.len() {
                            let (key, item) = &pairs[*index];
                            let key = key.clone();
                            let item = item.clone();
                            *index += 1;
                            screen_string(&key, seq, field, "key", state);
                            Some(item)
                        } else {
                            None
                        }
                    }
                    Frame::Array(items, index) => {
                        if *index < items.len() {
                            let item = items[*index].clone();
                            *index += 1;
                            Some(item)
                        } else {
                            None
                        }
                    }
                };
                match child {
                    None => {
                        stack.pop();
                    }
                    Some(child) => match &child {
                        Value::Str(text) => {
                            let text = text.clone();
                            screen_string(&text, seq, field, "value", state);
                        }
                        Value::Array(_) | Value::Object(_) => {
                            stack.push(frame_of(&child).unwrap());
                        }
                        _ => {}
                    },
                }
            }
        }
        _ => {}
    }
}

/// The screening report for one valid tape (FT-46).
pub fn screen_tape(tape: &Value) -> Result<Value, crate::identity::DigestError> {
    let mut state = Screening {
        hits: Vec::new(),
        strings: 0,
        paths: 0,
    };
    let empty = Value::Array(Vec::new());
    let events = tape.get("events").unwrap_or(&empty);
    if let Some(items) = events.as_array() {
        for event in items {
            let seq = event.get("seq").cloned().unwrap_or(Value::Null);
            let kind = event.get("kind");
            for (kind_name, field) in TEXT_FIELDS {
                if kind == Some(&Value::Str(Text::from_str(kind_name))) {
                    screen_value(event.get(field), &seq, field, &mut state);
                }
            }
            if kind == Some(&Value::Str(Text::from_str("tool_call"))) {
                screen_value(event.get("input"), &seq, "input", &mut state);
            }
        }
    }
    Ok(object(vec![
        (
            "tape",
            Value::Str(Text::from_str(&tape_digest(tape)?)),
        ),
        ("strings_screened", Value::Int(state.strings.to_string())),
        ("paths_found", Value::Int(state.paths.to_string())),
        ("hits", Value::Array(state.hits)),
        ("clean", Value::Bool(state.paths == 0)),
    ]))
}
