//! Determine a tape's outcome from its events (FT-14
//! to FT-16). The determination reads the main chain
//! only. Trailing session_end events are set aside
//! first. The rules then read the final main chain
//! event, and the outcome records the basis, the name
//! of the rule that fired. The sidechain and is_error
//! flags are booleans, and only the value true reads
//! as set.

use crate::value::{object, Text, Value};

pub const INTERRUPT_MARKER: &str = "[Request interrupted by user";

fn outcome(status: &str, basis: &str) -> Value {
    object(vec![
        ("status", Value::Str(Text::from_str(status))),
        ("basis", Value::Str(Text::from_str(basis))),
    ])
}

/// Return the outcome object for a tape's events
/// list. The events must be a list, as the Python's
/// TypeError states; callers inside the crate pass
/// event lists by construction.
pub fn determine_outcome(events: &Value) -> Value {
    let items = match events.as_array() {
        Some(items) => items,
        None => panic!("events must be a list"),
    };
    let mut main: Vec<&Value> = items
        .iter()
        .filter(|event| {
            event.as_object().is_some()
                && event.get("sidechain") != Some(&Value::Bool(true))
        })
        .collect();
    while let Some(last) = main.last() {
        if last.get("kind") == Some(&Value::Str(Text::from_str("session_end"))) {
            main.pop();
        } else {
            break;
        }
    }
    let final_event = match main.last() {
        Some(event) => *event,
        None => return outcome("unknown", "no-events"),
    };
    let kind = final_event.get("kind");
    if kind == Some(&Value::Str(Text::from_str("message")))
        && final_event.get("role") == Some(&Value::Str(Text::from_str("user")))
    {
        if let Some(Value::Str(text)) = final_event.get("text") {
            if let Some(rendered) = text.as_str() {
                if rendered.starts_with(INTERRUPT_MARKER) {
                    return outcome("aborted", "user-interrupt-marker");
                }
            }
        }
    }
    if kind == Some(&Value::Str(Text::from_str("tool_result")))
        && final_event.get("is_error") == Some(&Value::Bool(true))
    {
        return outcome("failed", "final-error-result");
    }
    if kind == Some(&Value::Str(Text::from_str("message")))
        && final_event.get("role") == Some(&Value::Str(Text::from_str("assistant")))
    {
        return outcome("completed", "final-assistant-message");
    }
    outcome("unknown", "unresolved-ending")
}
