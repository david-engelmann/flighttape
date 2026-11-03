//! Tests for tape comparison, FT-31 to FT-34,
//! ported from the Python suite.

use std::path::PathBuf;

use flighttape::compare::compare_tapes;
use flighttape::convert::convert_lines;
use flighttape::dumps::{dumps_indent, dumps_sorted};
use flighttape::identity::tape_digest;
use flighttape::json::parse;
use flighttape::validate::validate_tape;
use flighttape::value::{object, set_key, Text, Value};

fn text(value: &str) -> Value {
    Value::Str(Text::from_str(value))
}

fn int(value: &str) -> Value {
    Value::Int(value.to_string())
}

fn tape_a() -> Value {
    parse(
        r#"{
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "a",
                "totals": {"input_tokens": 10, "messages": 2},
                "redacted_values": 0, "skipped_lines": {"system": 1}},
        "events": [
            {"seq": 0, "kind": "message", "role": "user",
             "sidechain": false, "text": "hi",
             "timestamp": "2026-01-01T09:00:00.000Z"},
            {"seq": 1, "kind": "message", "role": "assistant",
             "sidechain": false, "text": "hello",
             "timestamp": "2026-01-01T09:00:10.000Z"}
        ],
        "outcome": {"status": "completed",
                    "basis": "final-assistant-message"}
    }"#,
    )
    .unwrap()
}

fn tape_b() -> Value {
    parse(
        r#"{
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "b",
                "totals": {"input_tokens": 14, "messages": 2,
                           "tool_calls": 1},
                "redacted_values": 1, "skipped_lines": {}},
        "events": [
            {"seq": 0, "kind": "message", "role": "user",
             "sidechain": false, "text": "hi",
             "timestamp": "2026-01-01T09:00:00.000Z"},
            {"seq": 1, "kind": "message", "role": "assistant",
             "sidechain": false, "text": "hello!",
             "timestamp": "2026-01-01T09:00:10.000Z"},
            {"seq": 2, "kind": "message", "role": "assistant",
             "sidechain": false, "text": "more",
             "timestamp": "2026-01-01T09:00:25.000Z"}
        ],
        "outcome": {"status": "completed",
                    "basis": "final-assistant-message"}
    }"#,
    )
    .unwrap()
}

fn expected_report() -> Value {
    let first = tape_a();
    let second = tape_b();
    let outcome_side = || {
        object(vec![
            ("status", text("completed")),
            ("basis", text("final-assistant-message")),
        ])
    };
    object(vec![
        (
            "digests",
            object(vec![
                ("a", text(&tape_digest(&first).unwrap())),
                ("b", text(&tape_digest(&second).unwrap())),
            ]),
        ),
        (
            "outcome",
            object(vec![
                ("a", outcome_side()),
                ("b", outcome_side()),
                ("changed", Value::Bool(false)),
            ]),
        ),
        (
            "duration_seconds",
            object(vec![
                ("a", Value::Float(10.0)),
                ("b", Value::Float(25.0)),
                ("delta", Value::Float(15.0)),
            ]),
        ),
        (
            "measures",
            object(vec![
                (
                    "input_tokens",
                    object(vec![
                        ("a", int("10")),
                        ("b", int("14")),
                        ("delta", int("4")),
                    ]),
                ),
                (
                    "messages",
                    object(vec![
                        ("a", int("2")),
                        ("b", int("2")),
                        ("delta", int("0")),
                    ]),
                ),
                ("tool_calls", object(vec![("b", int("1"))])),
            ]),
        ),
        (
            "redacted_values",
            object(vec![
                ("a", int("0")),
                ("b", int("1")),
                ("delta", int("1")),
            ]),
        ),
        (
            "skipped_lines",
            object(vec![("system", object(vec![("a", int("1"))]))]),
        ),
        (
            "events",
            object(vec![
                (
                    "count",
                    object(vec![("a", int("2")), ("b", int("3"))]),
                ),
                (
                    "by_kind",
                    object(vec![(
                        "message",
                        object(vec![("a", int("2")), ("b", int("3"))]),
                    )]),
                ),
                ("changed", Value::Array(vec![int("1")])),
                ("added", Value::Array(vec![int("2")])),
                ("removed", Value::Array(vec![])),
            ]),
        ),
    ])
}

#[test]
fn the_hand_built_pair_is_valid() {
    assert!(validate_tape(&tape_a()).is_empty());
    assert!(validate_tape(&tape_b()).is_empty());
}

#[test]
fn the_report_is_pinned_field_by_field() {
    let report = compare_tapes(&tape_a(), &tape_b()).unwrap();
    assert_eq!(report, expected_report());
}

#[test]
fn an_outcome_change_is_reported() {
    let mut second = tape_a();
    set_key(
        &mut second,
        "outcome",
        object(vec![
            ("status", text("unknown")),
            ("basis", text("unresolved-ending")),
        ]),
    );
    assert!(validate_tape(&second).is_empty());
    let report = compare_tapes(&tape_a(), &second).unwrap();
    let outcome = report.get("outcome").unwrap();
    assert_eq!(outcome.get("changed"), Some(&Value::Bool(true)));
    assert_eq!(
        outcome
            .get("b")
            .and_then(|side| side.get("status")),
        Some(&text("unknown"))
    );
}

fn fixture_tape() -> Value {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/claude_session_synthetic.jsonl"
    ))
    .unwrap();
    let lines: Vec<Option<String>> =
        raw.lines().map(|line| Some(line.to_string())).collect();
    convert_lines(lines)
}

#[test]
fn a_tape_compared_with_itself_reports_no_differences() {
    let tape = fixture_tape();
    let report = compare_tapes(&tape, &tape).unwrap();
    let digests = report.get("digests").unwrap();
    assert_eq!(digests.get("a"), digests.get("b"));
    assert_eq!(
        report.get("outcome").unwrap().get("changed"),
        Some(&Value::Bool(false))
    );
    let events = report.get("events").unwrap();
    assert_eq!(events.get("changed"), Some(&Value::Array(vec![])));
    assert_eq!(events.get("added"), Some(&Value::Array(vec![])));
    assert_eq!(events.get("removed"), Some(&Value::Array(vec![])));
    if let Value::Object(measures) = report.get("measures").unwrap() {
        for (_, entry) in measures {
            assert_eq!(entry.get("delta"), Some(&int("0")));
        }
    }
    assert_eq!(
        report
            .get("duration_seconds")
            .unwrap()
            .get("delta"),
        Some(&Value::Float(0.0))
    );
}

#[test]
fn one_changed_line_changes_exactly_its_event() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/claude_session_synthetic.jsonl"
    ))
    .unwrap();
    let lines: Vec<String> =
        raw.lines().map(|line| line.to_string()).collect();
    let mut changed_first = parse(&lines[0]).unwrap();
    {
        let content = changed_first
            .get_mut("message")
            .and_then(|message| message.get_mut("content"))
            .unwrap();
        if let Value::Array(blocks) = content {
            set_key(
                &mut blocks[0],
                "text",
                text("A different prompt"),
            );
        }
    }
    let mut variant = lines.clone();
    variant[0] = dumps_sorted(&changed_first);
    let to_lines = |items: Vec<String>| -> Vec<Option<String>> {
        items.into_iter().map(Some).collect()
    };
    let report = compare_tapes(
        &convert_lines(to_lines(lines)),
        &convert_lines(to_lines(variant)),
    )
    .unwrap();
    let events = report.get("events").unwrap();
    assert_eq!(events.get("changed"), Some(&Value::Array(vec![int("0")])));
    assert_eq!(events.get("added"), Some(&Value::Array(vec![])));
    assert_eq!(events.get("removed"), Some(&Value::Array(vec![])));
}

#[test]
fn duration_is_null_below_two_timestamps() {
    let mut single = tape_a();
    {
        let events = match single.get_mut("events") {
            Some(Value::Array(events)) => events,
            _ => panic!("events array"),
        };
        events.truncate(1);
    }
    set_key(
        &mut single,
        "outcome",
        object(vec![
            ("status", text("unknown")),
            ("basis", text("unresolved-ending")),
        ]),
    );
    assert!(validate_tape(&single).is_empty());
    let report = compare_tapes(&single, &tape_a()).unwrap();
    let duration = report.get("duration_seconds").unwrap();
    assert_eq!(duration.get("a"), Some(&Value::Null));
    assert!(duration.get("delta").is_none());
}

#[test]
fn null_totals_and_skipped_lines_compare_cleanly() {
    let mut first = tape_a();
    {
        let run = first.get_mut("run").unwrap();
        set_key(run, "totals", Value::Null);
        set_key(run, "skipped_lines", Value::Null);
    }
    assert!(validate_tape(&first).is_empty());
    let report = compare_tapes(&first, &tape_b()).unwrap();
    assert_eq!(
        report.get("measures"),
        Some(&object(vec![
            ("input_tokens", object(vec![("b", int("14"))])),
            ("messages", object(vec![("b", int("2"))])),
            ("tool_calls", object(vec![("b", int("1"))])),
        ]))
    );
    assert_eq!(
        report.get("skipped_lines"),
        Some(&Value::Object(vec![]))
    );
}

#[test]
fn a_non_numeric_redacted_value_is_carried_not_dropped() {
    let mut first = tape_a();
    let mut second = tape_b();
    {
        let run = first.get_mut("run").unwrap();
        set_key(run, "redacted_values", text("many"));
        let run = second.get_mut("run").unwrap();
        set_key(run, "redacted_values", text("few"));
    }
    assert!(validate_tape(&first).is_empty());
    let report = compare_tapes(&first, &second).unwrap();
    assert_eq!(
        report.get("redacted_values"),
        Some(&object(vec![("a", text("many")), ("b", text("few"))]))
    );
}

#[test]
fn an_outcome_without_a_basis_reports_a_null_basis() {
    let mut second = tape_a();
    set_key(
        &mut second,
        "outcome",
        object(vec![("status", text("completed"))]),
    );
    assert!(validate_tape(&second).is_empty());
    let report = compare_tapes(&tape_a(), &second).unwrap();
    let outcome = report.get("outcome").unwrap();
    assert_eq!(
        outcome.get("b"),
        Some(&object(vec![
            ("status", text("completed")),
            ("basis", Value::Null),
        ]))
    );
    assert_eq!(outcome.get("changed"), Some(&Value::Bool(true)));
}

#[test]
fn self_comparison_zeroes_redacted_and_skipped_deltas() {
    let tape = fixture_tape();
    let report = compare_tapes(&tape, &tape).unwrap();
    assert_eq!(
        report
            .get("redacted_values")
            .unwrap()
            .get("delta"),
        Some(&int("0"))
    );
    if let Value::Object(entries) = report.get("skipped_lines").unwrap() {
        for (_, entry) in entries {
            assert_eq!(entry.get("delta"), Some(&int("0")));
        }
    }
}

#[test]
fn an_unrepresentable_delta_is_omitted_not_emitted() {
    let huge = format!("1{}", "0".repeat(400));
    let mut first = tape_a();
    let mut second = tape_b();
    let set_redacted = |tape: &mut Value, value: Value| {
        let run = tape.get_mut("run").unwrap();
        set_key(run, "redacted_values", value);
    };
    set_redacted(&mut first, Value::Int(huge.clone()));
    set_redacted(&mut second, Value::Float(0.5));
    let report = compare_tapes(&first, &second).unwrap();
    assert_eq!(
        report.get("redacted_values"),
        Some(&object(vec![
            ("a", Value::Int(huge.clone())),
            ("b", Value::Float(0.5)),
        ]))
    );
    set_redacted(&mut first, Value::Float(-1e308));
    set_redacted(&mut second, Value::Float(1e308));
    let report = compare_tapes(&first, &second).unwrap();
    assert_eq!(
        report.get("redacted_values"),
        Some(&object(vec![
            ("a", Value::Float(-1e308)),
            ("b", Value::Float(1e308)),
        ]))
    );
    set_redacted(&mut first, Value::Int(huge.clone()));
    set_redacted(&mut second, int("1"));
    let report = compare_tapes(&first, &second).unwrap();
    let expected_delta = format!("-{}", "9".repeat(400));
    assert_eq!(
        report
            .get("redacted_values")
            .unwrap()
            .get("delta"),
        Some(&Value::Int(expected_delta))
    );
}

// ---- the CLI, exercised through the built binary ----

fn tmpdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "flighttape-compare-{}-{}",
        std::process::id(),
        tag
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_cli(args: &[String]) -> (i32, String, String) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_flighttape"))
        .args(args)
        .output()
        .unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn write_tape(dir: &PathBuf, name: &str, tape: &Value) -> String {
    let path = dir.join(name);
    std::fs::write(&path, dumps_indent(tape)).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn cli_compare_prints_the_report() {
    let dir = tmpdir("prints");
    let first = write_tape(&dir, "a.json", &tape_a());
    let second = write_tape(&dir, "b.json", &tape_b());
    let args = vec!["compare".to_string(), first, second];
    let (code, out, _) = run_cli(&args);
    assert_eq!(code, 0);
    assert_eq!(parse(out.trim_end()).unwrap(), expected_report());
    let (code, out_again, _) = run_cli(&args);
    assert_eq!(code, 0);
    assert_eq!(out_again, out);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_compare_refuses_an_invalid_tape() {
    let dir = tmpdir("invalid");
    let mut broken = tape_b();
    {
        let events = match broken.get_mut("events") {
            Some(Value::Array(events)) => events,
            _ => panic!("events array"),
        };
        set_key(&mut events[1], "seq", int("9"));
    }
    let first = write_tape(&dir, "a.json", &tape_a());
    let second = write_tape(&dir, "b.json", &broken);
    let (code, out, err) =
        run_cli(&["compare".into(), first, second.clone()]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.contains(&second), "stderr {}", err);
    assert!(err.contains("defect"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_compare_names_a_missing_file() {
    let dir = tmpdir("missing");
    let present = write_tape(&dir, "a.json", &tape_a());
    let missing = dir.join("nope.json").to_string_lossy().into_owned();
    let (code, _, err) = run_cli(&["compare".into(), present, missing]);
    assert_eq!(code, 1);
    assert!(err.contains("cannot read"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_output_is_identical_across_serializations() {
    let dir = tmpdir("serial");
    let first_path = dir.join("a.json");
    let second_path = dir.join("b.json");
    std::fs::write(&first_path, dumps_indent(&tape_a())).unwrap();
    std::fs::write(&second_path, dumps_sorted(&tape_b())).unwrap();
    let args = vec![
        "compare".to_string(),
        first_path.to_string_lossy().into_owned(),
        second_path.to_string_lossy().into_owned(),
    ];
    let (code, printed, _) = run_cli(&args);
    assert_eq!(code, 0);
    std::fs::write(&first_path, dumps_sorted(&tape_a())).unwrap();
    let (code, again, _) = run_cli(&args);
    assert_eq!(code, 0);
    assert_eq!(again, printed);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_compare_names_a_missing_file_and_an_invalid_tape() {
    let dir = tmpdir("bothbad");
    let mut tape = tape_b();
    {
        let events = match tape.get_mut("events") {
            Some(Value::Array(events)) => events,
            _ => panic!("events array"),
        };
        set_key(&mut events[1], "seq", int("9"));
    }
    let broken = write_tape(&dir, "broken.json", &tape);
    let missing = dir.join("nope.json").to_string_lossy().into_owned();
    let (code, out, err) =
        run_cli(&["compare".into(), broken.clone(), missing]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.contains("cannot read"), "stderr {}", err);
    assert!(err.contains(&broken), "stderr {}", err);
    assert!(err.contains("defect"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_compare_refuses_an_undigestible_tape() {
    let dir = tmpdir("undigestible");
    let first = write_tape(&dir, "a.json", &tape_a());
    let second_path = dir.join("b.json");
    let mut second_text = dumps_indent(&tape_b());
    second_text = second_text.replace("hello!", "\\ud800");
    std::fs::write(&second_path, &second_text).unwrap();
    let loaded = parse(&second_text).unwrap();
    assert!(validate_tape(&loaded).is_empty());
    let second = second_path.to_string_lossy().into_owned();
    let (code, out, err) = run_cli(&["compare".into(), first, second]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.contains("cannot be digested"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_compare_names_a_null_tape_instead_of_crashing() {
    let dir = tmpdir("null");
    let null_path = dir.join("null.json");
    std::fs::write(&null_path, "null\n").unwrap();
    let null_file = null_path.to_string_lossy().into_owned();
    let valid = write_tape(&dir, "a.json", &tape_a());
    let (code, out, err) =
        run_cli(&["compare".into(), null_file.clone(), valid.clone()]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.contains(&null_file), "stderr {}", err);
    assert!(err.contains("not a JSON object"), "stderr {}", err);
    let (code, _, _) = run_cli(&["compare".into(), valid, null_file]);
    assert_eq!(code, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_compare_names_which_file_is_not_json() {
    let dir = tmpdir("notjson");
    let bad_path = dir.join("bad.json");
    std::fs::write(&bad_path, [0xff, 0xfe, b' ', b'n', b'o']).unwrap();
    let bad = bad_path.to_string_lossy().into_owned();
    let valid = write_tape(&dir, "a.json", &tape_a());
    let (code, _, err) = run_cli(&["compare".into(), valid, bad.clone()]);
    assert_eq!(code, 1);
    assert!(err.contains("not valid UTF-8"), "stderr {}", err);
    assert!(err.contains(&bad), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}
