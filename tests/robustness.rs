//! Robustness tests (FT-18 to FT-21), ported from
//! the Python suite. The mutation corpus is the
//! Python generator's own output, pinned as a
//! fixture (seed 20261101, 240 cases) so the port
//! faces the identical corpus; its outcome totals
//! are pinned at 81 events and 188 counted skips,
//! verified against the Python before the fixture
//! was kept. Every line in it is synthetic.

use std::path::PathBuf;

use flighttape::convert::{convert_file, convert_lines};
use flighttape::dumps::dumps_indent;
use flighttape::json::parse;
use flighttape::validate::validate_tape;
use flighttape::value::{object, set_key, Text, Value};

fn text(value: &str) -> Value {
    Value::Str(Text::from_str(value))
}

fn int(value: &str) -> Value {
    Value::Int(value.to_string())
}

fn corpus() -> Vec<String> {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/mutation_corpus.jsonl"
    ))
    .unwrap();
    raw.lines().map(|line| line.to_string()).collect()
}

fn fixture_lines() -> Vec<String> {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/claude_session_synthetic.jsonl"
    ))
    .unwrap();
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect()
}

fn skipped_total(tape: &Value) -> i64 {
    let mut total = 0i64;
    if let Some(Value::Object(entries)) = tape.get("run").and_then(|run| run.get("skipped_lines")) {
        for (_, value) in entries {
            if let Value::Int(digits) = value {
                total += digits.parse::<i64>().unwrap();
            }
        }
    }
    total
}

#[test]
fn corpus_is_pinned_at_240_cases() {
    assert_eq!(corpus().len(), 240);
}

#[test]
fn corpus_never_raises_and_every_line_leaves_a_trace() {
    let mut total_events = 0usize;
    let mut total_skipped = 0i64;
    for line in corpus() {
        let tape = convert_lines(vec![Some(line.clone())]);
        let _defects = validate_tape(&tape);
        let skipped = skipped_total(&tape);
        if !line.trim().is_empty() {
            let events = tape.get("events").unwrap().as_array().unwrap().len();
            assert!(events > 0 || skipped > 0, "line {}", line);
        }
        total_events += tape.get("events").unwrap().as_array().unwrap().len();
        total_skipped += skipped;
    }
    assert_eq!(total_events, 81);
    assert_eq!(total_skipped, 188);
}

#[test]
fn a_structured_line_type_is_counted_never_raised() {
    for value in ["[\"user\"]", "{\"t\": 1}", "5", "0", "false", "[]", "{}"] {
        let line = format!(
            "{{\"type\": {}, \"sessionId\": \"synthetic-type\", \"message\": {{\"role\": \"user\", \"content\": \"hi\"}}}}",
            value
        );
        let tape = convert_lines(vec![Some(line)]);
        assert!(tape.get("events").unwrap().as_array().unwrap().is_empty());
        assert_eq!(
            tape.get("run").and_then(|run| run.get("skipped_lines")),
            Some(&object(vec![("malformed:type", int("1"))]))
        );
    }
}

#[test]
fn undecodable_lines_are_counted_not_fatal() {
    let dir = tmpdir("undecodable");
    let good = &fixture_lines()[0];
    let mut bytes = good.clone().into_bytes();
    bytes.push(b'\n');
    bytes.extend_from_slice(b"\xff\xfe not utf-8 \x80\n");
    let path = dir.join("mixed.jsonl");
    std::fs::write(&path, bytes).unwrap();
    let tape = convert_file(&path).unwrap();
    assert_eq!(
        tape.get("run")
            .and_then(|run| run.get("skipped_lines"))
            .and_then(|skipped| skipped.get("undecodable")),
        Some(&int("1"))
    );
    assert!(!tape.get("events").unwrap().as_array().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn huge_integer_line_is_counted_unparseable() {
    let line = format!(
        "{{\"type\":\"user\",\"message\":{{\"content\":{}}}}}}}",
        "9".repeat(6000)
    );
    let tape = convert_lines(vec![Some(line)]);
    assert!(tape.get("events").unwrap().as_array().unwrap().is_empty());
    assert_eq!(
        tape.get("run").and_then(|run| run.get("skipped_lines")),
        Some(&object(vec![("unparseable", int("1"))]))
    );
}

#[test]
fn deeply_nested_tool_input_is_counted_whole() {
    let nested = format!("{}{}1{}", "{\"a\":".repeat(3000), "", "}".repeat(3000));
    let line = format!(
        "{{\"type\": \"assistant\", \"sessionId\": \"synthetic-deep\", \"message\": {{\"role\": \"assistant\", \"content\": [{{\"type\": \"tool_use\", \"id\": \"c1\", \"name\": \"Read\", \"input\": {}}}]}}}}",
        nested
    );
    let tape = convert_lines(vec![Some(line)]);
    assert!(tape.get("events").unwrap().as_array().unwrap().is_empty());
    assert_eq!(
        tape.get("run").and_then(|run| run.get("skipped_lines")),
        Some(&object(vec![("too-deep", int("1"))]))
    );
}

fn good_tape() -> Value {
    parse(
        r#"{
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "s1"},
        "events": [
            {"seq": 0, "kind": "tool_call", "sidechain": false,
             "call_id": "c1", "tool": "Read"},
            {"seq": 1, "kind": "tool_result", "sidechain": false,
             "call_id": "c1"}
        ],
        "outcome": {"status": "completed"}
    }"#,
    )
    .unwrap()
}

/// One adversarial edit: a label, the defect word
/// it must earn, and the edit itself.
type CaseEdit = Box<dyn Fn(&mut Value)>;

#[test]
fn adversarial_tapes_are_named_defects_never_exceptions() {
    let cases: Vec<(&str, &str, CaseEdit)> = vec![
        (
            "kind as list",
            "kind",
            Box::new(|tape: &mut Value| {
                let events = match tape.get_mut("events") {
                    Some(Value::Array(events)) => events,
                    _ => panic!("events"),
                };
                set_key(
                    &mut events[0],
                    "kind",
                    Value::Array(vec![text("tool_call")]),
                );
            }),
        ),
        (
            "kind as dict",
            "kind",
            Box::new(|tape: &mut Value| {
                let events = match tape.get_mut("events") {
                    Some(Value::Array(events)) => events,
                    _ => panic!("events"),
                };
                set_key(&mut events[0], "kind", object(vec![("k", int("1"))]));
            }),
        ),
        (
            "seq as list",
            "seq",
            Box::new(|tape: &mut Value| {
                let events = match tape.get_mut("events") {
                    Some(Value::Array(events)) => events,
                    _ => panic!("events"),
                };
                set_key(&mut events[0], "seq", Value::Array(vec![int("0")]));
            }),
        ),
        (
            "status as list",
            "outcome.status",
            Box::new(|tape: &mut Value| {
                let outcome = tape.get_mut("outcome").unwrap();
                set_key(outcome, "status", Value::Array(vec![text("completed")]));
            }),
        ),
        (
            "status as dict with basis",
            "outcome.status",
            Box::new(|tape: &mut Value| {
                let outcome = tape.get_mut("outcome").unwrap();
                set_key(outcome, "status", object(vec![("s", int("1"))]));
                set_key(outcome, "basis", text("no-events"));
            }),
        ),
        (
            "basis as dict",
            "outcome.basis",
            Box::new(|tape: &mut Value| {
                let outcome = tape.get_mut("outcome").unwrap();
                set_key(outcome, "basis", object(vec![("b", int("1"))]));
            }),
        ),
        (
            "basis as list",
            "outcome.basis",
            Box::new(|tape: &mut Value| {
                let outcome = tape.get_mut("outcome").unwrap();
                set_key(outcome, "basis", Value::Array(vec![text("no-events")]));
            }),
        ),
        (
            "result call_id as list",
            "call_id",
            Box::new(|tape: &mut Value| {
                let events = match tape.get_mut("events") {
                    Some(Value::Array(events)) => events,
                    _ => panic!("events"),
                };
                set_key(&mut events[1], "call_id", Value::Array(vec![text("c1")]));
            }),
        ),
        (
            "call call_id as dict",
            "call_id",
            Box::new(|tape: &mut Value| {
                let events = match tape.get_mut("events") {
                    Some(Value::Array(events)) => events,
                    _ => panic!("events"),
                };
                set_key(&mut events[0], "call_id", object(vec![("c", int("1"))]));
            }),
        ),
        (
            "run as list",
            "run",
            Box::new(|tape: &mut Value| {
                set_key(tape, "run", Value::Array(vec![text("x")]));
            }),
        ),
        (
            "events as dict",
            "events",
            Box::new(|tape: &mut Value| {
                set_key(tape, "events", object(vec![("e", int("1"))]));
            }),
        ),
        (
            "event as string",
            "is not an object",
            Box::new(|tape: &mut Value| {
                let events = match tape.get_mut("events") {
                    Some(Value::Array(events)) => events,
                    _ => panic!("events"),
                };
                events[1] = text("not an event");
            }),
        ),
        (
            "timestamp as list",
            "timestamp",
            Box::new(|tape: &mut Value| {
                let events = match tape.get_mut("events") {
                    Some(Value::Array(events)) => events,
                    _ => panic!("events"),
                };
                set_key(
                    &mut events[0],
                    "timestamp",
                    Value::Array(vec![text("2026-01-01")]),
                );
            }),
        ),
        (
            "totals value as dict",
            "totals",
            Box::new(|tape: &mut Value| {
                let run = tape.get_mut("run").unwrap();
                set_key(
                    run,
                    "totals",
                    object(vec![("tool_calls", object(vec![("n", int("1"))]))]),
                );
            }),
        ),
        (
            "version as list",
            "tape_version",
            Box::new(|tape: &mut Value| {
                set_key(tape, "tape_version", Value::Array(vec![text("1.0")]));
            }),
        ),
        (
            "sidechain as string",
            "sidechain",
            Box::new(|tape: &mut Value| {
                let events = match tape.get_mut("events") {
                    Some(Value::Array(events)) => events,
                    _ => panic!("events"),
                };
                set_key(&mut events[0], "sidechain", text("no"));
            }),
        ),
    ];
    for (name, fragment, mutate) in cases {
        let mut tape = good_tape();
        mutate(&mut tape);
        let defects = validate_tape(&tape);
        assert!(!defects.is_empty(), "case {}", name);
        assert!(
            defects.iter().any(|defect| defect.contains(fragment)),
            "case {} defects {:?}",
            name,
            defects
        );
    }
}

#[test]
fn a_tape_that_is_not_an_object_is_a_defect() {
    for value in [
        Value::Array(vec![text("not"), text("a"), text("tape")]),
        text("not a tape"),
        int("7"),
        Value::Null,
    ] {
        let defects = validate_tape(&value);
        assert!(
            defects
                .iter()
                .any(|defect| defect.contains("is not a JSON object")),
            "defects {:?}",
            defects
        );
    }
}

// ---- the CLI, exercised through the built binary ----

fn tmpdir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("flighttape-robust-{}-{}", std::process::id(), tag));
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

#[test]
fn cli_convert_missing_source_is_a_named_error() {
    let dir = tmpdir("missing");
    let missing = dir.join("nope.jsonl").to_string_lossy().into_owned();
    let (code, _, err) = run_cli(&["convert".into(), missing.clone()]);
    assert_eq!(code, 1);
    assert!(err.contains("no such source file"), "stderr {}", err);
    let out = dir.join("out").to_string_lossy().into_owned();
    let (code, _, err) = run_cli(&["convert-dir".into(), missing, "-o".into(), out]);
    assert_eq!(code, 1);
    assert!(err.contains("no such source"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_convert_dir_names_an_entry_that_is_not_a_file() {
    let dir = tmpdir("entries");
    let src = dir.join("sessions");
    std::fs::create_dir_all(src.join("fake.jsonl")).unwrap();
    std::fs::write(src.join("real.jsonl"), format!("{}\n", fixture_lines()[0])).unwrap();
    let out = dir.join("tapes");
    let (code, _, err) = run_cli(&[
        "convert-dir".into(),
        src.to_string_lossy().into_owned(),
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("not a file"), "stderr {}", err);
    let summary = parse(&std::fs::read_to_string(out.join("summary.json")).unwrap()).unwrap();
    assert_eq!(summary.get("converted"), Some(&int("1")));
    assert_eq!(summary.get("failed"), Some(&int("1")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_convert_to_a_directory_output_is_a_named_error() {
    let dir = tmpdir("dirout");
    let source = dir.join("session.jsonl");
    std::fs::write(&source, format!("{}\n", fixture_lines()[0])).unwrap();
    let (code, _, err) = run_cli(&[
        "convert".into(),
        source.to_string_lossy().into_owned(),
        "-o".into(),
        dir.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("cannot write the tape file"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_validate_and_play_reject_an_undecodable_file() {
    let dir = tmpdir("undecodable-file");
    let bad = dir.join("bad.json");
    let mut bytes = b"{\"tape_version\": \"1.0\", ".to_vec();
    bytes.extend_from_slice(&[0xff, 0xfe, b'}']);
    std::fs::write(&bad, bytes).unwrap();
    let bad_arg = bad.to_string_lossy().into_owned();
    let (code, _, err) = run_cli(&["validate".into(), bad_arg.clone()]);
    assert_eq!(code, 1);
    assert!(err.contains("UTF-8"), "stderr {}", err);
    let (code, _, err) = run_cli(&["play".into(), bad_arg]);
    assert_eq!(code, 1);
    assert!(err.contains("UTF-8"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_validate_a_missing_tape_is_a_named_error() {
    let dir = tmpdir("missing-tape");
    let missing = dir.join("nope.json").to_string_lossy().into_owned();
    let (code, _, err) = run_cli(&["validate".into(), missing]);
    assert_eq!(code, 1);
    assert!(err.contains("cannot read the tape file"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_play_rejects_a_tape_with_an_unhashable_kind() {
    let dir = tmpdir("kind");
    let mut tape = good_tape();
    {
        let events = match tape.get_mut("events") {
            Some(Value::Array(events)) => events,
            _ => panic!("events"),
        };
        set_key(
            &mut events[0],
            "kind",
            Value::Array(vec![text("tool_call")]),
        );
    }
    let path = dir.join("tape.json");
    std::fs::write(&path, dumps_indent(&tape)).unwrap();
    let path_arg = path.to_string_lossy().into_owned();
    let (code, _, err) = run_cli(&["validate".into(), path_arg.clone()]);
    assert_eq!(code, 1);
    assert!(err.contains("kind"), "stderr {}", err);
    let (code, _, err) = run_cli(&["play".into(), path_arg]);
    assert_eq!(code, 1);
    assert!(err.contains("kind"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_good_tape_validates() {
    assert!(validate_tape(&good_tape()).is_empty());
}
