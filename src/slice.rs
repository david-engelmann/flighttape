//! Tape slicing (FT-39 to FT-43). A slice is an
//! excerpt of a valid tape that is itself a valid
//! tape: the slice renumbers its events from 0,
//! recomputes its outcome and its countable totals
//! over the selected events, and records where it was
//! cut from at run.slice_of, the source tape's digest
//! under canonical form 1.

use crate::identity::{tape_digest, DigestError};
use crate::outcome::determine_outcome;
use crate::validate::{validate_tape, EVENT_KINDS};
use crate::value::{int_cmp, object, set_key, Text, Value};

/// A selection that cannot be cut, named for the CLI.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceError(pub String);

impl std::fmt::Display for SliceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SliceError {}

const HUGE: i128 = 1_000_000_000_000_000_000;

/// Read one CLI bound (FT-39). Bounds arrive as text
/// so that an absurdly long digit string clamps to a
/// huge sentinel instead of dying in the interpreter's
/// integer parsing limit, and a non-integer is a named
/// refusal, never a traceback.
pub fn parse_bound(text: Option<&str>, flag: &str) -> Result<Option<i128>, SliceError> {
    let text = match text {
        None => return Ok(None),
        Some(text) => text,
    };
    // Python's int() accepts surrounding whitespace, a
    // sign, and digit separators (single underscores
    // between digits); it refuses the rest.
    match parse_python_int(text) {
        Some(value) => Ok(Some(value)),
        None => {
            let digits = text.trim_start_matches(['+', '-']);
            if !digits.is_empty()
                && digits.chars().all(|ch| ch.is_ascii_digit())
                && digits.len() > 18
            {
                return Ok(Some(if text.starts_with('-') { -HUGE } else { HUGE }));
            }
            Err(SliceError(format!(
                "the {} bound is not an integer, {}",
                flag, text
            )))
        }
    }
}

fn parse_python_int(text: &str) -> Option<i128> {
    let trimmed = text.trim_matches(|ch: char| {
        matches!(ch,
            '\u{09}'..='\u{0D}' | '\u{20}' | '\u{85}' | '\u{A0}' | '\u{1680}'
            | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}'
            | '\u{205F}' | '\u{3000}' | '\u{1C}'..='\u{1F}')
    });
    if trimmed.is_empty() {
        return None;
    }
    let (sign, rest) = match trimmed.strip_prefix('-') {
        Some(rest) => (-1i128, rest),
        None => (1i128, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    if rest.is_empty() {
        return None;
    }
    // Underscores are legal only between digits.
    let cleaned: String = rest.chars().filter(|ch| *ch != '_').collect();
    if cleaned.is_empty()
        || !cleaned.chars().all(|ch| ch.is_ascii_digit())
        || rest.starts_with('_')
        || rest.ends_with('_')
        || rest.contains("__")
    {
        return None;
    }
    if cleaned.len() > 18 {
        return None;
    }
    cleaned.parse::<i128>().ok().map(|value| sign * value)
}

/// Refuse a malformed selection before any file is
/// read. Bounds here are the parsed values.
pub fn check_selection(
    from_seq: Option<i128>,
    to_seq: Option<i128>,
    kinds: Option<&[String]>,
) -> Result<(), SliceError> {
    for (flag, bound) in [("--from", from_seq), ("--to", to_seq)] {
        if let Some(bound) = bound {
            if bound < 0 {
                return Err(SliceError(format!(
                    "the {} bound is below zero, {}",
                    flag, bound
                )));
            }
        }
    }
    if let (Some(from), Some(to)) = (from_seq, to_seq) {
        if from > to {
            return Err(SliceError(format!(
                "the --from bound {} is above the --to bound {}",
                from, to
            )));
        }
    }
    for kind in kinds.unwrap_or(&[]) {
        if !EVENT_KINDS.contains(&kind.as_str()) {
            return Err(SliceError(format!("unknown event kind, {}", kind)));
        }
    }
    Ok(())
}

/// Cut the selected events out of a valid tape.
pub fn slice_tape(
    tape: &Value,
    from_seq: Option<i128>,
    to_seq: Option<i128>,
    kinds: Option<Vec<String>>,
) -> Result<Value, SliceErrorOrDigest> {
    check_selection(from_seq, to_seq, kinds.as_deref())
        .map_err(SliceErrorOrDigest::Slice)?;
    build_slice(tape, from_seq, to_seq, kinds).map_err(|error| match error {
        SliceErrorOrDigest::Digest(digest_error) => {
            SliceErrorOrDigest::Digest(digest_error)
        }
        other => other,
    })
}

/// The slice command's two named failure families.
#[derive(Debug)]
pub enum SliceErrorOrDigest {
    Slice(SliceError),
    Digest(DigestError),
}

fn build_slice(
    tape: &Value,
    from_seq: Option<i128>,
    to_seq: Option<i128>,
    kinds: Option<Vec<String>>,
) -> Result<Value, SliceErrorOrDigest> {
    let digest = tape_digest(tape).map_err(SliceErrorOrDigest::Digest)?;
    let wanted: Option<Vec<String>> = kinds.clone();
    let mut selected: Vec<Value> = Vec::new();
    let empty = Value::Array(Vec::new());
    let events = tape.get("events").unwrap_or(&empty);
    if let Some(items) = events.as_array() {
        for event in items {
            let seq_digits = match event.get("seq") {
                Some(Value::Int(digits)) => digits.clone(),
                _ => continue,
            };
            if let Some(from) = from_seq {
                if int_cmp(&seq_digits, &from.to_string())
                    == std::cmp::Ordering::Less
                {
                    continue;
                }
            }
            if let Some(to) = to_seq {
                if int_cmp(&seq_digits, &to.to_string())
                    == std::cmp::Ordering::Greater
                {
                    continue;
                }
            }
            if let Some(wanted) = &wanted {
                let kind_ok = match event.get("kind") {
                    Some(Value::Str(text)) => text
                        .as_str()
                        .map(|name| wanted.iter().any(|kind| kind == &name))
                        .unwrap_or(false),
                    _ => false,
                };
                if !kind_ok {
                    continue;
                }
            }
            let mut clone = event.clone();
            set_key(
                &mut clone,
                "seq",
                Value::Int(selected.len().to_string()),
            );
            selected.push(clone);
        }
    }
    if selected.is_empty() {
        return Err(SliceErrorOrDigest::Slice(SliceError(
            "the selection matches no events".to_string(),
        )));
    }
    let mut result_pairs: Vec<(Text, Value)> = Vec::new();
    if let Some(pairs) = tape.as_object() {
        for (key, value) in pairs {
            let name = key.as_str().unwrap_or_default();
            if name != "events" && name != "run" && name != "outcome" {
                result_pairs.push((key.clone(), value.clone()));
            }
        }
    }
    let mut run = tape
        .get("run")
        .cloned()
        .unwrap_or(Value::Object(Vec::new()));
    let messages = selected
        .iter()
        .filter(|event| {
            event.get("kind") == Some(&Value::Str(Text::from_str("message")))
        })
        .count();
    let tool_calls = selected
        .iter()
        .filter(|event| {
            event.get("kind") == Some(&Value::Str(Text::from_str("tool_call")))
        })
        .count();
    set_key(
        &mut run,
        "totals",
        object(vec![
            ("messages", Value::Int(messages.to_string())),
            ("tool_calls", Value::Int(tool_calls.to_string())),
        ]),
    );
    set_key(
        &mut run,
        "slice_of",
        object(vec![
            ("digest", Value::Str(Text::from_str(&digest))),
            (
                "from",
                from_seq.map(|value| Value::Int(value.to_string())).unwrap_or(Value::Null),
            ),
            (
                "to",
                to_seq.map(|value| Value::Int(value.to_string())).unwrap_or(Value::Null),
            ),
            (
                "kinds",
                match &kinds {
                    Some(kinds) => Value::Array(
                        kinds
                            .iter()
                            .map(|kind| Value::Str(Text::from_str(kind)))
                            .collect(),
                    ),
                    None => Value::Null,
                },
            ),
            ("events", Value::Int(selected.len().to_string())),
        ]),
    );
    result_pairs.push((Text::from_str("run"), run));
    let selected_value = Value::Array(selected);
    let outcome = determine_outcome(&selected_value);
    result_pairs.push((Text::from_str("events"), selected_value));
    result_pairs.push((Text::from_str("outcome"), outcome));
    let result = Value::Object(result_pairs);
    let defects = validate_tape(&result);
    if !defects.is_empty() {
        return Err(SliceErrorOrDigest::Slice(SliceError(format!(
            "the slice is not a valid tape, {}",
            defects.join("; ")
        ))));
    }
    Ok(result)
}
