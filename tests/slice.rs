//! FT-39 to FT-43, tape slicing, ported from the
//! Python suite.

use std::path::PathBuf;

use flighttape::dumps::dumps_indent;
use flighttape::identity::tape_digest;
use flighttape::json::parse;
use flighttape::slice::{slice_tape, SliceErrorOrDigest};
use flighttape::validate::validate_tape;
use flighttape::value::{object, set_key, Text, Value};

const T0: &str = "2026-01-01T09:00:00.000Z";
const T1: &str = "2026-01-01T09:00:10.000Z";
const T2: &str = "2026-01-01T09:00:20.000Z";
const T3: &str = "2026-01-01T09:00:30.000Z";

fn text(value: &str) -> Value {
    Value::Str(Text::from_str(value))
}

fn int(value: &str) -> Value {
    Value::Int(value.to_string())
}

fn source_tape() -> Value {
    parse(
        r#"{
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "slice-me",
                "source_format": "claude-code-jsonl",
                "started_at": "2026-01-01T09:00:00.000Z",
                "skipped_lines": {"system": 1},
                "redacted_values": 2,
                "totals": {"input_tokens": 150, "output_tokens": 50,
                           "tool_calls": 1, "messages": 2}},
        "events": [
            {"seq": 0, "kind": "message", "sidechain": false,
             "timestamp": "2026-01-01T09:00:00.000Z",
             "role": "user", "text": "do it"},
            {"seq": 1, "kind": "message", "sidechain": false,
             "timestamp": "2026-01-01T09:00:10.000Z",
             "role": "assistant", "text": "done"},
            {"seq": 2, "kind": "tool_call", "sidechain": false,
             "timestamp": "2026-01-01T09:00:20.000Z",
             "tool": "run", "call_id": "call-1"},
            {"seq": 3, "kind": "note", "sidechain": false,
             "timestamp": "2026-01-01T09:00:30.000Z",
             "text": "later"}
        ],
        "outcome": {"status": "completed",
                    "basis": "final-assistant-message"}
    }"#,
    )
    .unwrap()
}

fn kinds_of(tape: &Value) -> Vec<Value> {
    tape.get("events")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event.get("kind").unwrap().clone())
        .collect()
}

fn seqs_of(tape: &Value) -> Vec<Value> {
    tape.get("events")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event.get("seq").unwrap().clone())
        .collect()
}

fn slice(
    tape: &Value,
    from: Option<i128>,
    to: Option<i128>,
    kinds: Option<Vec<String>>,
) -> Value {
    slice_tape(tape, from, to, kinds).expect("the slice succeeds")
}

fn slice_err(
    tape: &Value,
    from: Option<i128>,
    to: Option<i128>,
    kinds: Option<Vec<String>>,
) -> String {
    match slice_tape(tape, from, to, kinds) {
        Err(SliceErrorOrDigest::Slice(error)) => error.0,
        Err(SliceErrorOrDigest::Digest(error)) => {
            panic!("a digest error, not a slice error: {}", error.0)
        }
        Ok(_) => panic!("the slice unexpectedly succeeded"),
    }
}

#[test]
fn a_range_slice_renumbers_and_keeps_the_events() {
    let cut = slice(&source_tape(), Some(2), Some(3), None);
    assert_eq!(kinds_of(&cut), vec![text("tool_call"), text("note")]);
    assert_eq!(seqs_of(&cut), vec![int("0"), int("1")]);
    assert_eq!(
        cut.get("events").unwrap().as_array().unwrap()[1].get("text"),
        Some(&text("later"))
    );
    assert!(validate_tape(&cut).is_empty());
}

#[test]
fn bounds_omit_to_the_tapes_ends_and_clamp_to_what_exists() {
    let tape = source_tape();
    let whole = slice(&tape, None, None, None);
    assert_eq!(
        seqs_of(&whole),
        vec![int("0"), int("1"), int("2"), int("3")]
    );
    let past = slice(&tape, None, Some(99), None);
    assert_eq!(
        past.get("events").unwrap().as_array().unwrap().len(),
        4
    );
    let _ = slice_err(&tape, Some(99), None, None);
}

#[test]
fn a_kind_filter_selects_across_the_range() {
    let cut = slice(
        &source_tape(),
        None,
        None,
        Some(vec!["message".to_string()]),
    );
    assert_eq!(seqs_of(&cut), vec![int("0"), int("1")]);
    assert!(kinds_of(&cut).iter().all(|kind| *kind == text("message")));
    let both = slice(
        &source_tape(),
        Some(1),
        None,
        Some(vec!["message".to_string(), "note".to_string()]),
    );
    assert_eq!(kinds_of(&both), vec![text("message"), text("note")]);
}

#[test]
fn provenance_records_the_source_digest_and_the_request() {
    let tape = source_tape();
    let cut = slice(
        &tape,
        Some(1),
        None,
        Some(vec!["note".to_string()]),
    );
    let info = cut
        .get("run")
        .and_then(|run| run.get("slice_of"))
        .unwrap()
        .clone();
    assert_eq!(
        info,
        object(vec![
            ("digest", text(&tape_digest(&tape).unwrap())),
            ("from", int("1")),
            ("to", Value::Null),
            ("kinds", Value::Array(vec![text("note")])),
            ("events", int("1")),
        ])
    );
    let again = slice(&cut, None, None, None);
    assert_eq!(
        again
            .get("run")
            .and_then(|run| run.get("slice_of"))
            .and_then(|info| info.get("digest")),
        Some(&text(&tape_digest(&cut).unwrap()))
    );
    assert_ne!(tape_digest(&cut).unwrap(), tape_digest(&tape).unwrap());
}

#[test]
fn the_outcome_is_recomputed_never_carried() {
    let cut = slice(&source_tape(), None, Some(0), None);
    assert_eq!(
        cut.get("outcome"),
        Some(&object(vec![
            ("status", text("unknown")),
            ("basis", text("unresolved-ending")),
        ]))
    );
    let whole = slice(&source_tape(), None, None, None);
    assert_eq!(
        whole.get("outcome"),
        Some(&object(vec![
            ("status", text("unknown")),
            ("basis", text("unresolved-ending")),
        ]))
    );
}

#[test]
fn totals_are_recomputed_and_token_totals_are_omitted() {
    let cut = slice(&source_tape(), None, Some(2), None);
    assert_eq!(
        cut.get("run").and_then(|run| run.get("totals")),
        Some(&object(vec![
            ("messages", int("2")),
            ("tool_calls", int("1")),
        ]))
    );
    let notes = slice(
        &source_tape(),
        None,
        None,
        Some(vec!["note".to_string()]),
    );
    assert_eq!(
        notes.get("run").and_then(|run| run.get("totals")),
        Some(&object(vec![
            ("messages", int("0")),
            ("tool_calls", int("0")),
        ]))
    );
}

#[test]
fn run_identity_is_carried_as_recorded() {
    let cut = slice(&source_tape(), Some(3), None, None);
    let run = cut.get("run").unwrap();
    assert_eq!(run.get("harness"), Some(&text("claude-code")));
    assert_eq!(run.get("session_id"), Some(&text("slice-me")));
    assert_eq!(
        run.get("source_format"),
        Some(&text("claude-code-jsonl"))
    );
    assert_eq!(run.get("started_at"), Some(&text(T0)));
    assert_eq!(
        run.get("skipped_lines"),
        Some(&object(vec![("system", int("1"))]))
    );
    assert_eq!(run.get("redacted_values"), Some(&int("2")));
}

#[test]
fn a_slice_of_only_session_end_events_has_no_outcome_events() {
    let mut tape = source_tape();
    {
        let events = match tape.get_mut("events") {
            Some(Value::Array(events)) => events,
            _ => panic!("events array"),
        };
        events.push(object(vec![
            ("seq", int("4")),
            ("kind", text("session_end")),
            ("sidechain", Value::Bool(false)),
            ("timestamp", text(T3)),
        ]));
    }
    let cut = slice(
        &tape,
        None,
        None,
        Some(vec!["session_end".to_string()]),
    );
    assert_eq!(
        cut.get("outcome"),
        Some(&object(vec![
            ("status", text("unknown")),
            ("basis", text("no-events")),
        ]))
    );
    assert!(validate_tape(&cut).is_empty());
}

#[test]
fn malformed_selections_are_refused_by_name() {
    let tape = source_tape();
    let _ = slice_err(&tape, Some(-1), None, None);
    let _ = slice_err(&tape, Some(3), Some(1), None);
    let message = slice_err(
        &tape,
        None,
        None,
        Some(vec!["bogus".to_string()]),
    );
    assert!(message.contains("bogus"), "message {}", message);
}

fn paired_tape() -> Value {
    parse(
        r#"{
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "slice-me",
                "source_format": "claude-code-jsonl",
                "started_at": "2026-01-01T09:00:00.000Z",
                "skipped_lines": {"system": 1},
                "redacted_values": 2,
                "totals": {"input_tokens": 150, "output_tokens": 50,
                           "tool_calls": 1, "messages": 2}},
        "events": [
            {"seq": 0, "kind": "tool_call", "sidechain": false,
             "timestamp": "2026-01-01T09:00:00.000Z",
             "tool": "run", "call_id": "call-1"},
            {"seq": 1, "kind": "tool_result", "sidechain": false,
             "timestamp": "2026-01-01T09:00:10.000Z",
             "call_id": "call-1", "is_error": false, "text": "ok"},
            {"seq": 2, "kind": "message", "sidechain": false,
             "timestamp": "2026-01-01T09:00:20.000Z",
             "role": "assistant", "text": "done"}
        ],
        "outcome": {"status": "completed",
                    "basis": "final-assistant-message"}
    }"#,
    )
    .unwrap()
}

#[test]
fn the_library_refuses_a_slice_that_would_not_validate() {
    let _ = slice_err(
        &paired_tape(),
        None,
        None,
        Some(vec!["tool_result".to_string()]),
    );
}

// ---- the CLI, exercised through the built binary ----

fn tmpdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "flighttape-slice-{}-{}",
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

fn write_tape(dir: &PathBuf, tape: &Value) -> String {
    let path = dir.join("tape.json");
    std::fs::write(&path, dumps_indent(tape)).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn cli_slice_to_a_file_and_to_stdout() {
    let dir = tmpdir("basic");
    let source = write_tape(&dir, &source_tape());
    let out = dir.join("cut.json");
    let (code, _, _) = run_cli(&[
        "slice".into(),
        source.clone(),
        "--from".into(),
        "2".into(),
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 0);
    let cut = parse(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(kinds_of(&cut), vec![text("tool_call"), text("note")]);
    let (code, printed, _) = run_cli(&[
        "slice".into(),
        source,
        "--kind".into(),
        "note".into(),
    ]);
    assert_eq!(code, 0);
    let printed = parse(printed.trim_end()).unwrap();
    assert_eq!(kinds_of(&printed), vec![text("note")]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_refusals_write_nothing() {
    let dir = tmpdir("refusals");
    let source = write_tape(&dir, &source_tape());
    let out = dir.join("cut.json");
    let out_arg = out.to_string_lossy().into_owned();
    let (code, _, err) = run_cli(&[
        "slice".into(),
        source.clone(),
        "--from".into(),
        "3".into(),
        "--to".into(),
        "1".into(),
        "-o".into(),
        out_arg.clone(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("above the --to bound"), "stderr {}", err);
    assert!(!out.exists());
    let (code, _, err) = run_cli(&[
        "slice".into(),
        source.clone(),
        "--kind".into(),
        "bogus".into(),
        "-o".into(),
        out_arg.clone(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("unknown event kind"), "stderr {}", err);
    assert!(!out.exists());
    let (code, _, err) = run_cli(&[
        "slice".into(),
        source.clone(),
        "--from".into(),
        "99".into(),
        "-o".into(),
        out_arg.clone(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("matches no events"), "stderr {}", err);
    assert!(!out.exists());
    let missing = dir.join("missing.json").to_string_lossy().into_owned();
    let (code, _, _) = run_cli(&["slice".into(), missing]);
    assert_eq!(code, 1);
    assert!(!out.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_an_invalid_source_is_refused_with_its_defects() {
    let dir = tmpdir("invalid");
    let mut bad = source_tape();
    {
        let events = match bad.get_mut("events") {
            Some(Value::Array(events)) => events,
            _ => panic!("events array"),
        };
        set_key(&mut events[1], "seq", int("7"));
    }
    let source = write_tape(&dir, &bad);
    let out = dir.join("cut.json");
    let (code, _, err) = run_cli(&[
        "slice".into(),
        source,
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("is not a valid tape"), "stderr {}", err);
    assert!(err.contains("defect,"), "stderr {}", err);
    assert!(!out.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_a_slice_that_breaks_pairing_is_refused_before_writing() {
    let dir = tmpdir("pairing");
    let source = write_tape(&dir, &paired_tape());
    let out = dir.join("cut.json");
    let (code, _, err) = run_cli(&[
        "slice".into(),
        source,
        "--kind".into(),
        "tool_result".into(),
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("the slice is not a valid tape"), "stderr {}", err);
    assert!(err.contains("answers no earlier"), "stderr {}", err);
    assert!(!out.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_a_write_failure_is_a_named_error() {
    let dir = tmpdir("writefail");
    let source = write_tape(&dir, &source_tape());
    let (code, _, err) = run_cli(&[
        "slice".into(),
        source,
        "-o".into(),
        dir.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("cannot write the tape file"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_to_below_zero_and_two_kind_flags() {
    let dir = tmpdir("flags");
    let source = write_tape(&dir, &source_tape());
    let (code, _, err) = run_cli(&[
        "slice".into(),
        source.clone(),
        "--to=-1".into(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("below zero"), "stderr {}", err);
    let (code, printed, _) = run_cli(&[
        "slice".into(),
        source,
        "--kind".into(),
        "tool_call".into(),
        "--kind".into(),
        "note".into(),
    ]);
    assert_eq!(code, 0);
    let printed = parse(printed.trim_end()).unwrap();
    assert_eq!(kinds_of(&printed), vec![text("tool_call"), text("note")]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_an_undigestible_source_is_refused_by_name() {
    let dir = tmpdir("undigestible");
    let path = dir.join("tape.json");
    let tape_text =
        dumps_indent(&source_tape()).replace("do it", "\\ud800");
    std::fs::write(&path, &tape_text).unwrap();
    let loaded = parse(&tape_text).unwrap();
    assert!(validate_tape(&loaded).is_empty());
    let (code, out, err) =
        run_cli(&["slice".into(), path.to_string_lossy().into_owned()]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.contains("cannot be digested"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_a_deeply_nested_tape_is_a_named_refusal() {
    // The port refuses past the canonical limit of
    // 1000 nested containers (FT-54); the Python
    // refuses earlier, at its interpreter limit.
    let dir = tmpdir("deep");
    let mut tape = source_tape();
    let mut nested = Value::Array(vec![]);
    for _ in 0..1100 {
        nested = Value::Array(vec![nested]);
    }
    {
        let events = match tape.get_mut("events") {
            Some(Value::Array(events)) => events,
            _ => panic!("events array"),
        };
        let mut event = events[3].clone();
        set_key(&mut event, "payload", nested);
        events[3] = event;
    }
    let source = write_tape(&dir, &tape);
    let out = dir.join("cut.json");
    let (code, _, err) = run_cli(&[
        "slice".into(),
        source,
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("nested too deeply"), "stderr {}", err);
    assert!(!err.contains("Traceback"));
    assert!(!out.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_absurd_bounds_clamp_and_non_integers_are_named() {
    let dir = tmpdir("bounds");
    let source = write_tape(&dir, &source_tape());
    let huge = "9".repeat(5000);
    let (code, _, err) = run_cli(&[
        "slice".into(),
        source.clone(),
        "--from".into(),
        huge.clone(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("matches no events"), "stderr {}", err);
    let (code, printed, _) = run_cli(&[
        "slice".into(),
        source.clone(),
        "--to".into(),
        huge,
    ]);
    assert_eq!(code, 0);
    let printed = parse(printed.trim_end()).unwrap();
    assert_eq!(
        printed.get("events").unwrap().as_array().unwrap().len(),
        4
    );
    let (code, _, err) = run_cli(&[
        "slice".into(),
        source,
        "--from".into(),
        "abc".into(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("not an integer"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_a_source_holding_an_oversized_number_is_named() {
    let dir = tmpdir("oversized");
    let path = dir.join("tape.json");
    let tape_text = dumps_indent(&source_tape()).replace(
        "\"input_tokens\": 150",
        &format!("\"input_tokens\": {}", "9".repeat(5000)),
    );
    std::fs::write(&path, &tape_text).unwrap();
    let (code, _, err) =
        run_cli(&["slice".into(), path.to_string_lossy().into_owned()]);
    assert_eq!(code, 1);
    assert!(err.contains("number too large"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_t0_constant_is_the_fixture_start() {
    assert_eq!(T0, "2026-01-01T09:00:00.000Z");
    let _ = T1;
    let _ = T2;
}
