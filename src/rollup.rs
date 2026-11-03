//! Roll a directory of tapes up into one report
//! (FT-35 to FT-38). Only valid, digestible tapes are
//! rolled up; every other file is recorded as an
//! exclusion with its reason, and aggregates cover the
//! rolled tapes only.

use crate::validate::tape_duration;
use crate::value::{num_add, object, scalar_cmp, Text, Value};

/// The sum, or None when JSON cannot render it: a
/// float sum past finite, or an integer sum past the
/// 4300-digit rendering limit (FT-36 as amended).
fn renderable(total: &Value) -> Value {
    match total {
        Value::Float(number) if !number.is_finite() => Value::Null,
        Value::Int(digits) if digits.trim_start_matches('-').len() > 4300 => Value::Null,
        _ => total.clone(),
    }
}

/// Records are the CLI's construction: a rolled tape
/// carries "tape" and "digest"; an exclusion carries
/// "reason". The classification mirrors the Python's
/// ValueError discipline with a panic carrying the
/// same message; the CLI builds only well-formed
/// records, so this is unreachable from the command
/// line, as it is there.
fn classify(record: &Value) -> &'static str {
    let name = match record.get("file") {
        Some(Value::Str(text)) => text
            .as_str()
            .unwrap_or_else(|| String::from_utf16_lossy(text.units())),
        _ => "<unnamed>".to_string(),
    };
    let has_tape = record.get("tape").is_some();
    let has_reason = record.get("reason").is_some();
    if has_tape && has_reason {
        panic!("record for {} is both a tape and an exclusion", name);
    }
    if has_tape {
        if record
            .get("tape")
            .and_then(|tape| tape.as_object())
            .is_none()
        {
            panic!("rolled record for {} holds no tape", name);
        }
        if record.get("digest").is_none() {
            panic!("rolled record for {} holds no digest", name);
        }
        return "tape";
    }
    if has_reason {
        return "exclusion";
    }
    panic!("record for {} is neither a tape nor an exclusion", name);
}

fn tape_entry(record: &Value) -> Value {
    let tape = record.get("tape").expect("classified as a tape");
    let outcome = tape.get("outcome").cloned().unwrap_or(Value::Null);
    let totals = match tape.get("run").and_then(|run| run.get("totals")) {
        Some(totals) if crate::value::truthy(totals) => totals.clone(),
        _ => Value::Object(Vec::new()),
    };
    object(vec![
        ("file", record.get("file").cloned().unwrap_or(Value::Null)),
        (
            "digest",
            record.get("digest").cloned().unwrap_or(Value::Null),
        ),
        (
            "outcome",
            object(vec![
                (
                    "status",
                    outcome.get("status").cloned().unwrap_or(Value::Null),
                ),
                (
                    "basis",
                    outcome.get("basis").cloned().unwrap_or(Value::Null),
                ),
            ]),
        ),
        (
            "duration_seconds",
            tape_duration(tape).map(Value::Float).unwrap_or(Value::Null),
        ),
        (
            "events",
            Value::Int(
                tape.get("events")
                    .and_then(|events| events.as_array())
                    .map(|items| items.len())
                    .unwrap_or(0)
                    .to_string(),
            ),
        ),
        ("totals", totals),
    ])
}

fn exclusion_entry(record: &Value) -> Value {
    let mut pairs: Vec<(Text, Value)> = vec![
        (
            Text::from_str("file"),
            record.get("file").cloned().unwrap_or(Value::Null),
        ),
        (
            Text::from_str("reason"),
            record.get("reason").cloned().unwrap_or(Value::Null),
        ),
    ];
    if let Some(defects) = record.get("defects") {
        pairs.push((Text::from_str("defects"), defects.clone()));
    }
    if let Some(error) = record.get("error") {
        pairs.push((Text::from_str("error"), error.clone()));
    }
    Value::Object(pairs)
}

/// Sum the numeric values per key across mappings.
fn sum_maps(maps: &[Value]) -> Value {
    let mut sums: Vec<(Text, Value)> = Vec::new();
    let mut unrenderable: Vec<Text> = Vec::new();
    for map in maps {
        if let Some(pairs) = map.as_object() {
            for (key, value) in pairs {
                if !value.is_number() || unrenderable.contains(key) {
                    continue;
                }
                let current = sums
                    .iter()
                    .find(|(name, _)| name == key)
                    .map(|(_, total)| total.clone())
                    .unwrap_or(Value::Int("0".to_string()));
                match num_add(&current, value) {
                    Some(total) => match sums.iter_mut().find(|(name, _)| name == key) {
                        Some((_, existing)) => *existing = total,
                        None => sums.push((key.clone(), total)),
                    },
                    None => {
                        unrenderable.push(key.clone());
                        match sums.iter_mut().find(|(name, _)| name == key) {
                            Some((_, existing)) => *existing = Value::Null,
                            None => sums.push((key.clone(), Value::Null)),
                        }
                    }
                }
            }
        }
    }
    sums.sort_by(|a, b| scalar_cmp(&a.0, &b.0));
    Value::Object(
        sums.into_iter()
            .map(|(key, total)| (key, renderable(&total)))
            .collect(),
    )
}

/// The redacted_values aggregate over the rolled
/// runs.
fn redacted_summary(runs: &[Value]) -> Value {
    let mut total = Value::Int("0".to_string());
    let mut overflowed = false;
    let mut non_numeric: u64 = 0;
    for run in runs {
        if let Some(value) = run.get("redacted_values") {
            if !value.is_number() {
                non_numeric += 1;
            } else if !overflowed {
                match num_add(&total, value) {
                    Some(next) => total = next,
                    None => overflowed = true,
                }
            }
        }
    }
    object(vec![
        (
            "total",
            if overflowed {
                Value::Null
            } else {
                renderable(&total)
            },
        ),
        ("non_numeric", Value::Int(non_numeric.to_string())),
    ])
}

/// Build the rollup report from classified records.
/// Records may arrive in any order; filename order in
/// the report is this function's guarantee.
pub fn rollup_report(records: Vec<Value>, directory: &str) -> Value {
    let mut rolled: Vec<Value> = Vec::new();
    let mut excluded: Vec<Value> = Vec::new();
    for record in records {
        if classify(&record) == "tape" {
            rolled.push(record);
        } else {
            excluded.push(record);
        }
    }
    let file_key = |record: &Value| -> String {
        match record.get("file") {
            Some(Value::Str(text)) => text
                .as_str()
                .unwrap_or_else(|| String::from_utf16_lossy(text.units())),
            _ => String::new(),
        }
    };
    rolled.sort_by_key(|a| file_key(a));
    excluded.sort_by_key(|a| file_key(a));
    let entries: Vec<Value> = rolled.iter().map(tape_entry).collect();
    let runs: Vec<Value> = rolled
        .iter()
        .map(|record| {
            record
                .get("tape")
                .and_then(|tape| tape.get("run"))
                .cloned()
                .unwrap_or(Value::Object(Vec::new()))
        })
        .collect();
    let mut outcomes: Vec<(Text, u64)> = Vec::new();
    for entry in &entries {
        if let Some(Value::Str(status)) = entry
            .get("outcome")
            .and_then(|outcome| outcome.get("status"))
        {
            match outcomes.iter_mut().find(|(name, _)| name == status) {
                Some((_, count)) => *count += 1,
                None => outcomes.push((status.clone(), 1)),
            }
        }
    }
    outcomes.sort_by(|a, b| scalar_cmp(&a.0, &b.0));
    let durations: Vec<Option<f64>> = entries
        .iter()
        .map(|entry| match entry.get("duration_seconds") {
            Some(Value::Float(number)) => Some(*number),
            _ => None,
        })
        .collect();
    let known: Vec<f64> = durations.iter().flatten().copied().collect();
    let total_events: i64 = entries
        .iter()
        .map(|entry| match entry.get("events") {
            Some(Value::Int(digits)) => digits.parse::<i64>().unwrap_or(0),
            _ => 0,
        })
        .sum();
    let totals_maps: Vec<Value> = entries
        .iter()
        .map(|entry| {
            entry
                .get("totals")
                .cloned()
                .unwrap_or(Value::Object(Vec::new()))
        })
        .collect();
    let skipped_maps: Vec<Value> = runs
        .iter()
        .map(|run| match run.get("skipped_lines") {
            Some(skipped) if crate::value::truthy(skipped) => skipped.clone(),
            _ => Value::Object(Vec::new()),
        })
        .collect();
    object(vec![
        ("directory", Value::Str(Text::from_str(directory))),
        ("tapes", Value::Array(entries.clone())),
        (
            "excluded",
            Value::Array(excluded.iter().map(exclusion_entry).collect()),
        ),
        (
            "counts",
            object(vec![
                ("tapes", Value::Int(entries.len().to_string())),
                ("excluded", Value::Int(excluded.len().to_string())),
                ("events", Value::Int(total_events.to_string())),
            ]),
        ),
        (
            "outcomes",
            Value::Object(
                outcomes
                    .into_iter()
                    .map(|(key, count)| (key, Value::Int(count.to_string())))
                    .collect(),
            ),
        ),
        ("totals", sum_maps(&totals_maps)),
        ("redacted_values", redacted_summary(&runs)),
        ("skipped_lines", sum_maps(&skipped_maps)),
        (
            "duration_seconds",
            object(vec![
                // A fold from positive zero: the
                // standard sum seeds an empty fold
                // with negative zero, where Python's
                // sum starts at zero itself.
                (
                    "total",
                    Value::Float(known.iter().fold(0.0, |total, each| total + each)),
                ),
                ("known", Value::Int(known.len().to_string())),
                (
                    "unknown",
                    Value::Int((durations.len() - known.len()).to_string()),
                ),
            ]),
        ),
    ])
}
