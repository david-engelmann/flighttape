//! Playback tests, ported from the Python suite.

use flighttape::convert::convert_file;
use flighttape::play::render_tape;
use flighttape::value::{object, set_key, Text, Value};

fn fixture_tape() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/claude_session_synthetic.jsonl"
    );
    convert_file(std::path::Path::new(path)).unwrap()
}

fn text(value: &str) -> Value {
    Value::Str(Text::from_str(value))
}

fn int(value: &str) -> Value {
    Value::Int(value.to_string())
}

fn events_mut(tape: &mut Value) -> &mut Vec<Value> {
    match tape.get_mut("events") {
        Some(Value::Array(events)) => events,
        _ => panic!("the fixture tape holds an events array"),
    }
}

fn remove_key(value: &mut Value, key: &str) {
    if let Value::Object(entries) = value {
        let key = Text::from_str(key);
        entries.retain(|(entry_key, _)| *entry_key != key);
    }
}

#[test]
fn rendering_matches_the_golden_file() {
    let golden = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/playback_golden.txt"
    ))
    .unwrap();
    assert_eq!(render_tape(&fixture_tape()), golden);
}

#[test]
fn rendering_is_deterministic() {
    let tape = fixture_tape();
    let first = render_tape(&tape);
    assert_eq!(render_tape(&tape), first);
}

#[test]
fn header_carries_run_facts() {
    let out = render_tape(&fixture_tape());
    for needle in [
        "synthetic-session-001",
        "claude-code",
        "outcome unknown",
        "tool calls 1",
        "input tokens 150",
    ] {
        assert!(out.contains(needle), "missing {}", needle);
    }
}

#[test]
fn every_carried_field_appears() {
    let out = render_tape(&fixture_tape());
    for needle in [
        "Synthetic prompt, fix the widget counter.",
        "Read",
        "toolu_synth_1",
        "counter = 0",
        "sidechain",
        "Synthetic sidechain note",
    ] {
        assert!(out.contains(needle), "missing {}", needle);
    }
}

#[test]
fn offsets_are_seconds_from_the_first_timestamp() {
    let out = render_tape(&fixture_tape());
    assert!(out.contains("[+0.00s] #0"));
    assert!(out.contains("[+5.00s] #1"));
    assert!(out.contains("[+60.0s]"));
}

#[test]
fn sub_second_offsets_stay_visible() {
    let mut tape = fixture_tape();
    set_key(
        &mut events_mut(&mut tape)[1],
        "timestamp",
        text("2026-01-01T09:00:00.040Z"),
    );
    let out = render_tape(&tape);
    assert!(out.contains("[+0.04s] #1"));
}

#[test]
fn untimestamped_events_render_no_time() {
    let mut tape = fixture_tape();
    for event in events_mut(&mut tape).iter_mut() {
        remove_key(event, "timestamp");
    }
    let out = render_tape(&tape);
    assert!(out.contains("[no time] #0"));
}

#[test]
fn tool_call_input_is_rendered() {
    let out = render_tape(&fixture_tape());
    assert!(out.contains("\"path\": \"widget.py\""));
}

#[test]
fn diff_and_spend_content_is_rendered() {
    let mut tape = fixture_tape();
    events_mut(&mut tape).push(object(vec![
        ("seq", int("5")),
        ("kind", text("diff")),
        ("sidechain", Value::Bool(false)),
        ("timestamp", text("2026-01-01T09:02:00.000Z")),
        ("content", text("counter = 0 becomes counter = 1")),
    ]));
    events_mut(&mut tape).push(object(vec![
        ("seq", int("6")),
        ("kind", text("spend")),
        ("sidechain", Value::Bool(false)),
        ("timestamp", text("2026-01-01T09:02:01.000Z")),
        ("content", text("0.0042 USD")),
    ]));
    let out = render_tape(&tape);
    assert!(out.contains("counter = 0 becomes counter = 1"));
    assert!(out.contains("0.0042 USD"));
}

#[test]
fn extra_event_fields_are_rendered_not_dropped() {
    let mut tape = fixture_tape();
    let first = events_mut(&mut tape)[0].clone();
    let mut first = first;
    set_key(&mut first, "model", text("synthetic-model"));
    events_mut(&mut tape)[0] = first;
    let out = render_tape(&tape);
    assert!(out.contains("\"model\": \"synthetic-model\""));
}

#[test]
fn zero_totals_render_and_cache_labels_are_exact() {
    let mut tape = fixture_tape();
    assert!(render_tape(&tape).contains("cache creation input tokens 0"));
    {
        let totals = tape
            .get_mut("run")
            .and_then(|run| run.get_mut("totals"))
            .unwrap()
            .clone();
        let mut totals = totals;
        set_key(&mut totals, "cache_creation_input_tokens", int("500"));
        set_key(&mut totals, "cache_read_input_tokens", int("900"));
        let run = tape.get_mut("run").unwrap();
        set_key(run, "totals", totals);
    }
    let out = render_tape(&tape);
    assert!(out.contains("cache creation input tokens 500"));
    assert!(out.contains("cache read input tokens 900"));
}

#[test]
fn counted_figures_come_from_the_events_not_the_totals() {
    let mut tape = fixture_tape();
    let totals = tape
        .get_mut("run")
        .and_then(|run| run.get_mut("totals"))
        .unwrap()
        .clone();
    let mut totals = totals;
    set_key(&mut totals, "tool_calls", int("99"));
    let run = tape.get_mut("run").unwrap();
    set_key(run, "totals", totals);
    let out = render_tape(&tape);
    assert!(out.contains("tool calls 1"));
    assert!(!out.contains("tool calls 99"));
}

#[test]
fn totals_absent_fallback_counts_events() {
    let mut tape = fixture_tape();
    {
        let run = tape.get_mut("run").unwrap();
        let mut run = run.clone();
        remove_key(&mut run, "totals");
        *tape.get_mut("run").unwrap() = run;
    }
    let out = render_tape(&tape);
    assert!(out.contains("tool calls 1"));
    assert!(out.contains("messages 3"));
}

#[test]
fn header_states_run_metadata_and_loss_counts() {
    let out = render_tape(&fixture_tape());
    assert!(out.contains("started 2026-01-01T09:00:00.000Z"));
    assert!(out.contains("source claude-code-jsonl"));
    assert!(out.contains("skipped lines 1 (system 1)"));
}

#[test]
fn error_results_are_marked() {
    let mut tape = fixture_tape();
    for index in 0..events_mut(&mut tape).len() {
        let is_result = events_mut(&mut tape)[index].get("kind")
            == Some(&text("tool_result"));
        if is_result {
            let mut event = events_mut(&mut tape)[index].clone();
            set_key(&mut event, "is_error", Value::Bool(true));
            events_mut(&mut tape)[index] = event;
        }
    }
    assert!(render_tape(&tape).contains("ERROR"));
}

#[test]
fn header_states_the_outcome_basis() {
    let mut tape = fixture_tape();
    let header = render_tape(&tape).lines().nth(1).unwrap().to_string();
    assert!(header.starts_with("outcome unknown (unresolved-ending), "));
    set_key(
        &mut tape,
        "outcome",
        object(vec![("status", text("completed"))]),
    );
    let header = render_tape(&tape).lines().nth(1).unwrap().to_string();
    assert!(header.starts_with("outcome completed, "));
}

#[test]
fn header_survives_an_unvalidated_outcome_shape() {
    let mut tape = fixture_tape();
    set_key(
        &mut tape,
        "outcome",
        object(vec![
            ("status", int("5")),
            ("basis", text("final-error-result")),
        ]),
    );
    let header = render_tape(&tape).lines().nth(1).unwrap().to_string();
    assert!(header.starts_with("outcome unknown, "));
    set_key(
        &mut tape,
        "outcome",
        object(vec![("status", text("completed")), ("basis", int("7"))]),
    );
    let header = render_tape(&tape).lines().nth(1).unwrap().to_string();
    assert!(header.starts_with("outcome completed, "));
    set_key(&mut tape, "outcome", text("completed"));
    let header = render_tape(&tape).lines().nth(1).unwrap().to_string();
    assert!(header.starts_with("outcome unknown, "));
}
