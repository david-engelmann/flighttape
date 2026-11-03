//! Tests for the converter and the outcome rules,
//! ported from the Python suite files for file.

use flighttape::convert::{convert_file, convert_lines};
use flighttape::json::parse;
use flighttape::outcome::determine_outcome;
use flighttape::validate::validate_tape;
use flighttape::value::{object, Text, Value};

fn fixture_tape() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/claude_session_synthetic.jsonl"
    );
    convert_file(std::path::Path::new(path)).unwrap()
}

fn lines(values: &[&str]) -> Vec<Option<String>> {
    values.iter().map(|line| Some(line.to_string())).collect()
}

fn events(tape: &Value) -> Vec<Value> {
    tape.get("events").unwrap().as_array().unwrap().clone()
}

fn of_kind(tape: &Value, kind: &str) -> Vec<Value> {
    events(tape)
        .into_iter()
        .filter(|event| {
            event.get("kind") == Some(&Value::Str(Text::from_str(kind)))
        })
        .collect()
}

fn skipped(tape: &Value, key: &str) -> Option<Value> {
    tape.get("run")
        .and_then(|run| run.get("skipped_lines"))
        .and_then(|skipped| skipped.get(key))
        .cloned()
}

fn total(tape: &Value, key: &str) -> Option<Value> {
    tape.get("run")
        .and_then(|run| run.get("totals"))
        .and_then(|totals| totals.get(key))
        .cloned()
}

fn int(value: &str) -> Value {
    Value::Int(value.to_string())
}

fn text(value: &str) -> Value {
    Value::Str(Text::from_str(value))
}

#[test]
fn converted_tape_validates() {
    assert!(validate_tape(&fixture_tape()).is_empty());
}

#[test]
fn tool_call_keeps_source_id_and_pairs_with_result() {
    let tape = fixture_tape();
    let calls = of_kind(&tape, "tool_call");
    let results = of_kind(&tape, "tool_result");
    assert_eq!(calls[0].get("call_id"), Some(&text("toolu_synth_1")));
    assert_eq!(results[0].get("call_id"), Some(&text("toolu_synth_1")));
    assert_eq!(calls[0].get("tool"), Some(&text("Read")));
}

#[test]
fn sidechain_event_is_preserved_and_flagged() {
    let tape = fixture_tape();
    let side: Vec<Value> = events(&tape)
        .into_iter()
        .filter(|event| event.get("sidechain") == Some(&Value::Bool(true)))
        .collect();
    assert_eq!(side.len(), 1);
    let rendered = side[0]
        .get("text")
        .and_then(|value| match value {
            Value::Str(text) => text.as_str(),
            _ => None,
        })
        .unwrap();
    assert!(rendered.contains("sidechain note"));
}

#[test]
fn usage_is_summed_into_run_totals() {
    let tape = fixture_tape();
    assert_eq!(total(&tape, "input_tokens"), Some(int("150")));
    assert_eq!(total(&tape, "output_tokens"), Some(int("50")));
    assert_eq!(total(&tape, "tool_calls"), Some(int("1")));
    assert_eq!(total(&tape, "messages"), Some(int("3")));
}

#[test]
fn cache_tokens_are_summed_under_their_own_keys() {
    let tape = convert_lines(lines(&[
        r#"{"type": "assistant", "sessionId": "s3", "message": {"role": "assistant", "usage": {"input_tokens": 100, "output_tokens": 10, "cache_creation_input_tokens": 500, "cache_read_input_tokens": 900}, "content": [{"type": "text", "text": "hi"}]}}"#,
    ]));
    assert_eq!(total(&tape, "cache_creation_input_tokens"), Some(int("500")));
    assert_eq!(total(&tape, "cache_read_input_tokens"), Some(int("900")));
}

#[test]
fn tool_result_content_is_carried_into_the_tape() {
    let tape = fixture_tape();
    let results = of_kind(&tape, "tool_result");
    assert_eq!(results[0].get("content"), Some(&text("counter = 0")));
}

#[test]
fn tool_result_error_flag_is_preserved() {
    let tape = convert_lines(lines(&[
        r#"{"type": "assistant", "sessionId": "s4", "message": {"role": "assistant", "content": [{"type": "tool_use", "id": "t1", "name": "Bash", "input": {}}]}}"#,
        r#"{"type": "user", "sessionId": "s4", "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "t1", "is_error": true, "content": "boom"}]}}"#,
    ]));
    let results = of_kind(&tape, "tool_result");
    assert_eq!(results[0].get("is_error"), Some(&Value::Bool(true)));
    assert_eq!(results[0].get("content"), Some(&text("boom")));
}

#[test]
fn events_keep_timestamps_and_run_keeps_start() {
    let tape = fixture_tape();
    assert_eq!(
        tape.get("run").and_then(|run| run.get("started_at")),
        Some(&text("2026-01-01T09:00:00.000Z"))
    );
    assert_eq!(
        events(&tape)[0].get("timestamp"),
        Some(&text("2026-01-01T09:00:00.000Z"))
    );
}

#[test]
fn non_object_and_malformed_lines_are_counted_not_fatal() {
    let tape = convert_lines(lines(&[
        "42",
        "[1,2,3]",
        "null",
        r#"{"type": "user", "sessionId": "s5", "message": "oops"}"#,
    ]));
    assert_eq!(skipped(&tape, "non-object"), Some(int("3")));
    assert_eq!(skipped(&tape, "malformed:message"), Some(int("1")));
    assert!(events(&tape).is_empty());
}

#[test]
fn second_session_lines_are_counted_and_skipped() {
    let tape = convert_lines(lines(&[
        r#"{"type": "user", "sessionId": "first", "message": {"role": "user", "content": "one"}}"#,
        r#"{"type": "user", "sessionId": "second", "message": {"role": "user", "content": "two"}}"#,
    ]));
    assert_eq!(
        tape.get("run").and_then(|run| run.get("session_id")),
        Some(&text("first"))
    );
    assert_eq!(skipped(&tape, "foreign-session"), Some(int("1")));
    assert_eq!(events(&tape).len(), 1);
}

#[test]
fn contentless_lines_are_counted() {
    let tape = convert_lines(lines(&[
        r#"{"type": "user", "sessionId": "s6", "message": {"role": "user", "content": []}}"#,
    ]));
    assert_eq!(skipped(&tape, "contentless:user"), Some(int("1")));
    assert!(events(&tape).is_empty());
}

#[test]
fn empty_conversion_fabricates_no_event() {
    let tape = convert_lines(Vec::new());
    assert!(events(&tape).is_empty());
    assert!(!validate_tape(&tape).is_empty());
}

#[test]
fn skipped_lines_are_counted_by_type() {
    let tape = fixture_tape();
    assert_eq!(
        tape.get("run").and_then(|run| run.get("skipped_lines")),
        Some(&object(vec![("system", int("1"))]))
    );
}

#[test]
fn unparseable_line_is_counted_not_fatal() {
    let tape = convert_lines(lines(&[
        "{\"type\": \"user\", broken",
        r#"{"type": "user", "sessionId": "s9", "message": {"role": "user", "content": "hello"}}"#,
    ]));
    assert_eq!(skipped(&tape, "unparseable"), Some(int("1")));
    assert!(validate_tape(&tape).is_empty());
}

#[test]
fn tape_carries_no_source_paths() {
    let tape = fixture_tape();
    let blob = flighttape::dumps::dumps_indent(&tape);
    assert!(!blob.contains("/home/synthetic"));
    assert!(!blob.contains("synthetic-branch"));
    assert!(!blob.contains("\"cwd\"") && !blob.contains("gitBranch"));
    assert_eq!(
        tape.get("run").and_then(|run| run.get("source_format")),
        Some(&text("claude-code-jsonl"))
    );
    assert_eq!(
        tape.get("run").and_then(|run| run.get("session_id")),
        Some(&text("synthetic-session-001"))
    );
}

#[test]
fn absolute_paths_in_tool_inputs_are_reduced_and_counted() {
    let tape = convert_lines(lines(&[
        r#"{"type": "assistant", "sessionId": "s7", "message": {"role": "assistant", "content": [{"type": "tool_use", "id": "t9", "name": "Read", "input": {"file_path": "/home/synthetic/private/notes.txt"}}]}}"#,
    ]));
    let blob = flighttape::dumps::dumps_indent(&tape);
    assert!(!blob.contains("/home/synthetic"));
    assert!(blob.contains("notes.txt"));
    assert_eq!(
        tape.get("run").and_then(|run| run.get("redacted_values")),
        Some(&int("1"))
    );
}

#[test]
fn string_content_becomes_a_message_event() {
    let tape = convert_lines(lines(&[
        r#"{"type": "user", "sessionId": "s2", "message": {"role": "user", "content": "plain string prompt"}}"#,
    ]));
    let kinds: Vec<Value> = events(&tape)
        .iter()
        .map(|event| event.get("kind").unwrap().clone())
        .collect();
    assert_eq!(kinds, vec![text("message")]);
}

#[test]
fn fixture_outcome_is_unknown_with_unresolved_ending_basis() {
    let tape = fixture_tape();
    assert_eq!(
        tape.get("outcome"),
        Some(&object(vec![
            ("status", text("unknown")),
            ("basis", text("unresolved-ending")),
        ]))
    );
}

// ---- outcome determination, ported ----

fn event(pairs: Vec<(&str, Value)>) -> Value {
    object(pairs)
}

fn message(role: &str, text_value: Value, sidechain: Value) -> Value {
    event(vec![
        ("kind", text("message")),
        ("role", text(role)),
        ("text", text_value),
        ("sidechain", sidechain),
    ])
}

fn outcome_of(events: Vec<Value>) -> Value {
    determine_outcome(&Value::Array(events))
}

fn expect_outcome(events: Vec<Value>, status: &str, basis: &str) {
    assert_eq!(
        outcome_of(events),
        object(vec![("status", text(status)), ("basis", text(basis))])
    );
}

#[test]
fn outcome_completed_on_final_assistant_message() {
    expect_outcome(
        vec![
            message("user", text("text"), Value::Bool(false)),
            message("assistant", text("text"), Value::Bool(false)),
        ],
        "completed",
        "final-assistant-message",
    );
}

#[test]
fn outcome_failed_on_final_error_result() {
    expect_outcome(
        vec![
            message("user", text("text"), Value::Bool(false)),
            event(vec![
                ("kind", text("tool_call")),
                ("sidechain", Value::Bool(false)),
            ]),
            event(vec![
                ("kind", text("tool_result")),
                ("is_error", Value::Bool(true)),
                ("sidechain", Value::Bool(false)),
            ]),
        ],
        "failed",
        "final-error-result",
    );
}

#[test]
fn outcome_recovered_error_is_not_failed() {
    let outcome = outcome_of(vec![
        event(vec![
            ("kind", text("tool_result")),
            ("is_error", Value::Bool(true)),
            ("sidechain", Value::Bool(false)),
        ]),
        message("assistant", text("text"), Value::Bool(false)),
    ]);
    assert_eq!(outcome.get("status"), Some(&text("completed")));
}

#[test]
fn outcome_aborted_on_the_interrupt_marker() {
    expect_outcome(
        vec![
            message("assistant", text("text"), Value::Bool(false)),
            message("user", text("[Request interrupted by user]"), Value::Bool(false)),
        ],
        "aborted",
        "user-interrupt-marker",
    );
    let outcome = outcome_of(vec![message(
        "user",
        text("[Request interrupted by user for tool use]"),
        Value::Bool(false),
    )]);
    assert_eq!(outcome.get("status"), Some(&text("aborted")));
}

#[test]
fn outcome_unknown_endings() {
    expect_outcome(
        vec![message("user", text("text"), Value::Bool(false))],
        "unknown",
        "unresolved-ending",
    );
    for events in [
        vec![event(vec![
            ("kind", text("tool_call")),
            ("sidechain", Value::Bool(false)),
        ])],
        vec![event(vec![
            ("kind", text("tool_result")),
            ("is_error", Value::Bool(false)),
            ("sidechain", Value::Bool(false)),
        ])],
        vec![event(vec![
            ("kind", text("note")),
            ("sidechain", Value::Bool(false)),
        ])],
    ] {
        assert_eq!(
            outcome_of(events).get("basis"),
            Some(&text("unresolved-ending"))
        );
    }
}

#[test]
fn outcome_no_main_chain_events_is_unknown_no_events() {
    expect_outcome(vec![], "unknown", "no-events");
    expect_outcome(
        vec![message("assistant", text("text"), Value::Bool(true))],
        "unknown",
        "no-events",
    );
}

#[test]
fn outcome_sidechain_ending_never_decides() {
    let outcome = outcome_of(vec![
        message("assistant", text("text"), Value::Bool(false)),
        event(vec![
            ("kind", text("tool_result")),
            ("is_error", Value::Bool(true)),
            ("sidechain", Value::Bool(true)),
        ]),
    ]);
    assert_eq!(outcome.get("status"), Some(&text("completed")));
    let outcome = outcome_of(vec![
        event(vec![
            ("kind", text("tool_result")),
            ("is_error", Value::Bool(true)),
            ("sidechain", Value::Bool(false)),
        ]),
        message("assistant", text("text"), Value::Bool(true)),
    ]);
    assert_eq!(outcome.get("status"), Some(&text("failed")));
}

#[test]
fn outcome_trailing_session_end_is_set_aside() {
    let end = || {
        event(vec![
            ("kind", text("session_end")),
            ("sidechain", Value::Bool(false)),
        ])
    };
    let outcome = outcome_of(vec![
        event(vec![
            ("kind", text("tool_result")),
            ("is_error", Value::Bool(true)),
            ("sidechain", Value::Bool(false)),
        ]),
        end(),
    ]);
    assert_eq!(outcome.get("status"), Some(&text("failed")));
    expect_outcome(vec![end()], "unknown", "no-events");
    let outcome = outcome_of(vec![
        message("assistant", text("text"), Value::Bool(false)),
        end(),
        end(),
    ]);
    assert_eq!(outcome.get("status"), Some(&text("completed")));
}

#[test]
fn outcome_marker_must_begin_the_text() {
    expect_outcome(
        vec![message(
            "user",
            text("Quoting a log line, [Request interrupted by user] appears here"),
            Value::Bool(false),
        )],
        "unknown",
        "unresolved-ending",
    );
    for text_value in [
        "[Request interrupted by the system]",
        " [Request interrupted by user]",
    ] {
        let outcome = outcome_of(vec![message(
            "user",
            text(text_value),
            Value::Bool(false),
        )]);
        assert_eq!(outcome.get("status"), Some(&text("unknown")));
    }
}

#[test]
fn outcome_flags_read_as_booleans_only() {
    for is_error in [text("false"), int("1")] {
        let outcome = outcome_of(vec![event(vec![
            ("kind", text("tool_result")),
            ("is_error", is_error),
            ("sidechain", Value::Bool(false)),
        ])]);
        assert_eq!(outcome.get("basis"), Some(&text("unresolved-ending")));
    }
    for sidechain in [int("1"), text("yes")] {
        let outcome = outcome_of(vec![message(
            "assistant",
            text("text"),
            sidechain,
        )]);
        assert_eq!(outcome.get("status"), Some(&text("completed")));
    }
}

#[test]
fn outcome_non_object_entries_are_ignored() {
    expect_outcome(
        vec![Value::Null, int("42"), text("x")],
        "unknown",
        "no-events",
    );
    let outcome = outcome_of(vec![
        int("42"),
        message("assistant", text("text"), Value::Bool(false)),
    ]);
    assert_eq!(outcome.get("status"), Some(&text("completed")));
}

#[test]
fn outcome_final_user_message_without_string_text_is_unknown() {
    let outcome = outcome_of(vec![event(vec![
        ("kind", text("message")),
        ("role", text("user")),
        ("sidechain", Value::Bool(false)),
    ])]);
    assert_eq!(outcome.get("status"), Some(&text("unknown")));
    let outcome = outcome_of(vec![message(
        "user",
        Value::Null,
        Value::Bool(false),
    )]);
    assert_eq!(outcome.get("status"), Some(&text("unknown")));
}

#[test]
fn outcome_non_message_events_with_roles_do_not_decide() {
    for event in [
        event(vec![
            ("kind", text("note")),
            ("role", text("assistant")),
            ("sidechain", Value::Bool(false)),
        ]),
        event(vec![
            ("kind", text("diff")),
            ("role", text("user")),
            ("sidechain", Value::Bool(false)),
            ("text", text("[Request interrupted by user]")),
        ]),
    ] {
        let outcome = outcome_of(vec![event]);
        assert_eq!(outcome.get("status"), Some(&text("unknown")));
    }
}

#[test]
#[should_panic(expected = "events must be a list")]
fn outcome_events_must_be_a_list() {
    let _ = determine_outcome(&Value::Null);
}

#[test]
fn converter_records_outcomes() {
    let line = |kind: &str, content: &str| -> String {
        format!(
            "{{\"type\": \"{kind}\", \"sessionId\": \"synthetic-outcome\", \"message\": {{\"role\": \"{kind}\", \"content\": {content}}}}}",
            kind = kind,
            content = content
        )
    };
    let tape = convert_lines(lines(&[
        &line("user", "\"Synthetic prompt.\""),
        &line("assistant", "\"Synthetic answer.\""),
    ]));
    assert_eq!(
        tape.get("outcome"),
        Some(&object(vec![
            ("status", text("completed")),
            ("basis", text("final-assistant-message")),
        ]))
    );
    let tape = convert_lines(lines(&[
        &line("user", "\"Synthetic prompt.\""),
        &line("assistant", "\"Synthetic partial answer.\""),
        &line("user", "\"[Request interrupted by user]\""),
    ]));
    assert_eq!(
        tape.get("outcome"),
        Some(&object(vec![
            ("status", text("aborted")),
            ("basis", text("user-interrupt-marker")),
        ]))
    );
    let tape = convert_lines(lines(&[
        &line(
            "assistant",
            "[{\"type\": \"tool_use\", \"id\": \"c1\", \"name\": \"Read\", \"input\": {}}]",
        ),
        &line(
            "user",
            "[{\"type\": \"tool_result\", \"tool_use_id\": \"c1\", \"is_error\": true, \"content\": \"Synthetic failure.\"}]",
        ),
    ]));
    assert_eq!(
        tape.get("outcome"),
        Some(&object(vec![
            ("status", text("failed")),
            ("basis", text("final-error-result")),
        ]))
    );
}

#[test]
fn the_golden_fixture_digest_stands() {
    let tape = fixture_tape();
    assert_eq!(
        flighttape::identity::tape_digest(&tape).unwrap(),
        "cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3"
    );
    let _ = parse("null").unwrap();
}
