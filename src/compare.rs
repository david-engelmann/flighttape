//! Compare two tapes, the second against the first
//! (FT-31 to FT-34). Inputs must be valid tapes; the
//! command line validates before it compares, and
//! refuses a side it cannot digest, since the report
//! carries digests.

use crate::identity::{canonical_bytes, tape_digest, DigestError};
use crate::validate::tape_duration;
use crate::value::{object, scalar_cmp, Text, Value};

/// The difference, or None when it is not a finite
/// number. A huge integer against a float overflows
/// the subtraction, and distant floats can differ by
/// more than a double holds; the report is JSON, so
/// an unrepresentable delta is omitted.
fn delta(left: &Value, right: &Value) -> Option<Value> {
    let result = crate::value::num_sub(right, left)?;
    match result {
        Value::Float(number) if !number.is_finite() => None,
        _ => Some(result),
    }
}

fn entry(left: Option<&Value>, right: Option<&Value>) -> Value {
    let mut pairs: Vec<(Text, Value)> = Vec::new();
    if let Some(left) = left {
        if left.is_number() {
            pairs.push((Text::from_str("a"), left.clone()));
        }
    }
    if let Some(right) = right {
        if right.is_number() {
            pairs.push((Text::from_str("b"), right.clone()));
        }
    }
    if let (Some(left), Some(right)) = (left, right) {
        if left.is_number() && right.is_number() {
            if let Some(delta) = delta(left, right) {
                pairs.push((Text::from_str("delta"), delta));
            }
        }
    }
    Value::Object(pairs)
}

fn sorted_keys(first: &Value, second: &Value) -> Vec<Text> {
    let mut keys: Vec<Text> = Vec::new();
    for map in [first, second] {
        if let Some(pairs) = map.as_object() {
            for (key, _) in pairs {
                if !keys.contains(key) {
                    keys.push(key.clone());
                }
            }
        }
    }
    keys.sort_by(scalar_cmp);
    keys
}

fn measures(first: &Value, second: &Value) -> Value {
    let mut pairs: Vec<(Text, Value)> = Vec::new();
    for key in sorted_keys(first, second) {
        let entry = entry(
            first.get(&key_string(&key)),
            second.get(&key_string(&key)),
        );
        if !matches!(&entry, Value::Object(items) if items.is_empty()) {
            pairs.push((key, entry));
        }
    }
    Value::Object(pairs)
}

fn key_string(key: &Text) -> String {
    key.as_str()
        .unwrap_or_else(|| String::from_utf16_lossy(key.units()))
}

/// The redacted_values entry, each side's value as it
/// stands (FT-32): the validator does not constrain
/// this field's type, so the report carries the value
/// itself, with a delta only when both sides are
/// numbers.
fn carried(first_run: &Value, second_run: &Value) -> Value {
    let mut pairs: Vec<(Text, Value)> = Vec::new();
    let a = first_run.get("redacted_values");
    let b = second_run.get("redacted_values");
    if let Some(a) = a {
        pairs.push((Text::from_str("a"), a.clone()));
    }
    if let Some(b) = b {
        pairs.push((Text::from_str("b"), b.clone()));
    }
    if let (Some(a), Some(b)) = (a, b) {
        if a.is_number() && b.is_number() {
            if let Some(delta) = delta(a, b) {
                pairs.push((Text::from_str("delta"), delta));
            }
        }
    }
    Value::Object(pairs)
}

fn by_seq(tape: &Value) -> Vec<(String, Value)> {
    let mut entries: Vec<(String, Value)> = Vec::new();
    if let Some(events) = tape.get("events").and_then(|value| value.as_array()) {
        for event in events {
            if let Some(Value::Int(digits)) = event.get("seq") {
                match entries.iter_mut().find(|(seq, _)| seq == digits) {
                    Some((_, existing)) => *existing = event.clone(),
                    None => entries.push((digits.clone(), event.clone())),
                }
            }
        }
    }
    entries
}

fn by_kind(tape: &Value) -> Vec<(Text, u64)> {
    let mut counts: Vec<(Text, u64)> = Vec::new();
    if let Some(events) = tape.get("events").and_then(|value| value.as_array()) {
        for event in events {
            if let Some(Value::Str(kind)) = event.get("kind") {
                match counts.iter_mut().find(|(name, _)| name == kind) {
                    Some((_, count)) => *count += 1,
                    None => counts.push((kind.clone(), 1)),
                }
            }
        }
    }
    counts
}

fn events_report(first: &Value, second: &Value) -> Value {
    let first_map = by_seq(first);
    let second_map = by_seq(second);
    let first_kinds = by_kind(first);
    let second_kinds = by_kind(second);
    let mut kind_keys: Vec<Text> = Vec::new();
    for (kind, _) in first_kinds.iter().chain(second_kinds.iter()) {
        if !kind_keys.contains(kind) {
            kind_keys.push(kind.clone());
        }
    }
    kind_keys.sort_by(scalar_cmp);
    let mut kind_pairs: Vec<(Text, Value)> = Vec::new();
    for kind in &kind_keys {
        let a = first_kinds
            .iter()
            .find(|(name, _)| name == kind)
            .map(|(_, count)| *count)
            .unwrap_or(0);
        let b = second_kinds
            .iter()
            .find(|(name, _)| name == kind)
            .map(|(_, count)| *count)
            .unwrap_or(0);
        kind_pairs.push((
            kind.clone(),
            object(vec![
                ("a", Value::Int(a.to_string())),
                ("b", Value::Int(b.to_string())),
            ]),
        ));
    }
    let mut changed: Vec<String> = Vec::new();
    let mut added: Vec<String> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    for (seq, event) in &first_map {
        match second_map.iter().find(|(other, _)| other == seq) {
            Some((_, other_event)) => {
                if canonical_bytes(event).ok() != canonical_bytes(other_event).ok() {
                    changed.push(seq.clone());
                }
            }
            None => removed.push(seq.clone()),
        }
    }
    for (seq, _) in &second_map {
        if !first_map.iter().any(|(other, _)| other == seq) {
            added.push(seq.clone());
        }
    }
    let sort_seqs = |seqs: &mut Vec<String>| {
        seqs.sort_by(|a, b| crate::value::int_cmp(a, b));
    };
    sort_seqs(&mut changed);
    sort_seqs(&mut added);
    sort_seqs(&mut removed);
    let seq_array = |seqs: Vec<String>| {
        Value::Array(
            seqs.into_iter()
                .map(Value::Int)
                .collect(),
        )
    };
    let count_of = |tape: &Value| -> usize {
        tape.get("events")
            .and_then(|value| value.as_array())
            .map(|items| items.len())
            .unwrap_or(0)
    };
    object(vec![
        (
            "count",
            object(vec![
                ("a", Value::Int(count_of(first).to_string())),
                ("b", Value::Int(count_of(second).to_string())),
            ]),
        ),
        ("by_kind", Value::Object(kind_pairs)),
        ("changed", seq_array(changed)),
        ("added", seq_array(added)),
        ("removed", seq_array(removed)),
    ])
}

/// Return the comparison report for two valid tapes,
/// or the digest error when either side cannot be
/// digested, as the Python raises there.
pub fn compare_tapes(
    first: &Value,
    second: &Value,
) -> Result<Value, DigestError> {
    let first_digest = tape_digest(first)?;
    let second_digest = tape_digest(second)?;
    let first_outcome = first.get("outcome").cloned().unwrap_or(Value::Null);
    let second_outcome = second.get("outcome").cloned().unwrap_or(Value::Null);
    let first_duration = tape_duration(first);
    let second_duration = tape_duration(second);
    let mut duration_pairs: Vec<(Text, Value)> = vec![
        (
            Text::from_str("a"),
            first_duration.map(Value::Float).unwrap_or(Value::Null),
        ),
        (
            Text::from_str("b"),
            second_duration.map(Value::Float).unwrap_or(Value::Null),
        ),
    ];
    if let (Some(a), Some(b)) = (first_duration, second_duration) {
        duration_pairs.push((Text::from_str("delta"), Value::Float(b - a)));
    }
    let outcome_side = |outcome: &Value| -> Value {
        object(vec![
            (
                "status",
                outcome.get("status").cloned().unwrap_or(Value::Null),
            ),
            (
                "basis",
                outcome.get("basis").cloned().unwrap_or(Value::Null),
            ),
        ])
    };
    let changed = first_outcome.get("status") != second_outcome.get("status")
        || first_outcome.get("basis") != second_outcome.get("basis");
    let empty = Value::Object(Vec::new());
    let totals_of = |tape: &Value| -> Value {
        match tape.get("run").and_then(|run| run.get("totals")) {
            Some(totals) if crate::value::truthy(totals) => totals.clone(),
            _ => empty.clone(),
        }
    };
    let skipped_of = |tape: &Value| -> Value {
        match tape.get("run").and_then(|run| run.get("skipped_lines")) {
            Some(skipped) if crate::value::truthy(skipped) => skipped.clone(),
            _ => empty.clone(),
        }
    };
    Ok(object(vec![
        (
            "digests",
            object(vec![
                (
                    "a",
                    Value::Str(Text::from_str(&first_digest)),
                ),
                (
                    "b",
                    Value::Str(Text::from_str(&second_digest)),
                ),
            ]),
        ),
        (
            "outcome",
            object(vec![
                ("a", outcome_side(&first_outcome)),
                ("b", outcome_side(&second_outcome)),
                ("changed", Value::Bool(changed)),
            ]),
        ),
        ("duration_seconds", Value::Object(duration_pairs)),
        (
            "measures",
            measures(&totals_of(first), &totals_of(second)),
        ),
        (
            "redacted_values",
            carried(
                first.get("run").unwrap_or(&empty),
                second.get("run").unwrap_or(&empty),
            ),
        ),
        (
            "skipped_lines",
            measures(&skipped_of(first), &skipped_of(second)),
        ),
        ("events", events_report(first, second)),
    ]))
}
