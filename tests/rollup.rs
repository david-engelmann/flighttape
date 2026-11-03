//! FT-35 to FT-38, the directory rollup, ported
//! from the Python suite.

use std::path::PathBuf;

use flighttape::dumps::dumps_indent;
use flighttape::identity::tape_digest;
use flighttape::json::parse;
use flighttape::rollup::rollup_report;
use flighttape::validate::validate_tape;
use flighttape::value::{object, set_key, Text, Value};

fn text(value: &str) -> Value {
    Value::Str(Text::from_str(value))
}

fn int(value: &str) -> Value {
    Value::Int(value.to_string())
}

fn tape_one() -> Value {
    parse(
        r#"{
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "one",
                "totals": {"input_tokens": 10, "messages": 2},
                "redacted_values": 2, "skipped_lines": {"system": 1}},
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

fn tape_two() -> Value {
    parse(
        r#"{
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "two",
                "totals": {"input_tokens": 5, "tool_calls": 3},
                "redacted_values": "many",
                "skipped_lines": {"system": 2, "summary": 1}},
        "events": [
            {"seq": 0, "kind": "message", "role": "user",
             "sidechain": false, "text": "hey",
             "timestamp": "2026-01-01T10:00:00.000Z"}
        ],
        "outcome": {"status": "failed",
                    "basis": "final-error-result"}
    }"#,
    )
    .unwrap()
}

fn records(named: &[(&str, Value)]) -> Vec<Value> {
    named
        .iter()
        .map(|(name, tape)| {
            object(vec![
                ("file", text(name)),
                ("tape", tape.clone()),
                ("digest", text(&tape_digest(tape).unwrap())),
            ])
        })
        .collect()
}

#[test]
fn the_hand_built_pair_is_valid() {
    assert!(validate_tape(&tape_one()).is_empty());
    assert!(validate_tape(&tape_two()).is_empty());
}

#[test]
fn the_report_is_pinned_field_by_field() {
    let report = rollup_report(
        records(&[("b.json", tape_two()), ("a.json", tape_one())]),
        "dir",
    );
    assert_eq!(report.get("directory"), Some(&text("dir")));
    let tapes = report.get("tapes").unwrap().as_array().unwrap();
    let files: Vec<Value> = tapes
        .iter()
        .map(|entry| entry.get("file").unwrap().clone())
        .collect();
    assert_eq!(files, vec![text("a.json"), text("b.json")]);
    let first = &tapes[0];
    assert_eq!(
        first.get("digest"),
        Some(&text(&tape_digest(&tape_one()).unwrap()))
    );
    assert_eq!(
        first.get("outcome"),
        Some(&object(vec![
            ("status", text("completed")),
            ("basis", text("final-assistant-message")),
        ]))
    );
    assert_eq!(first.get("duration_seconds"), Some(&Value::Float(10.0)));
    assert_eq!(first.get("events"), Some(&int("2")));
    assert_eq!(
        first.get("totals"),
        Some(&object(vec![
            ("input_tokens", int("10")),
            ("messages", int("2")),
        ]))
    );
    assert_eq!(tapes[1].get("duration_seconds"), Some(&Value::Null));
    assert_eq!(report.get("excluded"), Some(&Value::Array(vec![])));
    assert_eq!(
        report.get("counts"),
        Some(&object(vec![
            ("tapes", int("2")),
            ("excluded", int("0")),
            ("events", int("3")),
        ]))
    );
    assert_eq!(
        report.get("outcomes"),
        Some(&object(
            vec![("completed", int("1")), ("failed", int("1")),]
        ))
    );
    assert_eq!(
        report.get("totals"),
        Some(&object(vec![
            ("input_tokens", int("15")),
            ("messages", int("2")),
            ("tool_calls", int("3")),
        ]))
    );
    assert_eq!(
        report.get("redacted_values"),
        Some(&object(vec![
            ("total", int("2")),
            ("non_numeric", int("1")),
        ]))
    );
    assert_eq!(
        report.get("skipped_lines"),
        Some(&object(vec![("summary", int("1")), ("system", int("3")),]))
    );
    assert_eq!(
        report.get("duration_seconds"),
        Some(&object(vec![
            ("total", Value::Float(10.0)),
            ("known", int("1")),
            ("unknown", int("1")),
        ]))
    );
}

#[test]
fn identical_files_are_two_tapes_not_one() {
    let report = rollup_report(
        records(&[("a.json", tape_one()), ("copy.json", tape_one())]),
        "d",
    );
    assert_eq!(report.get("counts").unwrap().get("tapes"), Some(&int("2")));
    assert_eq!(
        report.get("totals").unwrap().get("input_tokens"),
        Some(&int("20"))
    );
    let digest = tape_digest(&tape_one()).unwrap();
    let digests: Vec<Value> = report
        .get("tapes")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry.get("digest").unwrap().clone())
        .collect();
    assert_eq!(digests, vec![text(&digest), text(&digest)]);
}

#[test]
fn record_order_does_not_change_the_report() {
    let forward = rollup_report(
        records(&[("a.json", tape_one()), ("b.json", tape_two())]),
        "d",
    );
    let backward = rollup_report(
        records(&[("b.json", tape_two()), ("a.json", tape_one())]),
        "d",
    );
    assert_eq!(dumps_indent(&forward), dumps_indent(&backward));
}

#[test]
fn a_null_totals_tape_rolls_up_with_empty_totals() {
    let mut tape = tape_one();
    {
        let run = tape.get_mut("run").unwrap();
        set_key(run, "totals", Value::Null);
        set_key(run, "skipped_lines", Value::Null);
    }
    assert!(validate_tape(&tape).is_empty());
    let report = rollup_report(records(&[("a.json", tape)]), "d");
    assert_eq!(
        report.get("tapes").unwrap().as_array().unwrap()[0].get("totals"),
        Some(&Value::Object(vec![]))
    );
    assert_eq!(report.get("totals"), Some(&Value::Object(vec![])));
    assert_eq!(report.get("skipped_lines"), Some(&Value::Object(vec![])));
}

#[test]
fn an_outcome_without_a_basis_reports_a_null_basis() {
    let mut tape = tape_one();
    set_key(
        &mut tape,
        "outcome",
        object(vec![("status", text("completed"))]),
    );
    assert!(validate_tape(&tape).is_empty());
    let report = rollup_report(records(&[("a.json", tape)]), "d");
    assert_eq!(
        report.get("tapes").unwrap().as_array().unwrap()[0].get("outcome"),
        Some(&object(vec![
            ("status", text("completed")),
            ("basis", Value::Null),
        ]))
    );
}

#[test]
fn an_unrepresentable_redacted_total_is_null() {
    let huge = format!("1{}", "0".repeat(400));
    let mut first = tape_one();
    let mut second = tape_two();
    let set_redacted = |tape: &mut Value, value: Value| {
        let run = tape.get_mut("run").unwrap();
        set_key(run, "redacted_values", value);
    };
    set_redacted(&mut first, Value::Int(huge.clone()));
    set_redacted(&mut second, Value::Float(0.5));
    let report = rollup_report(
        records(&[("a.json", first.clone()), ("b.json", second.clone())]),
        "d",
    );
    assert_eq!(
        report.get("redacted_values"),
        Some(&object(vec![
            ("total", Value::Null),
            ("non_numeric", int("0")),
        ]))
    );
    set_redacted(&mut second, int("1"));
    let report = rollup_report(records(&[("a.json", first), ("b.json", second)]), "d");
    assert_eq!(
        report.get("redacted_values").unwrap().get("total"),
        Some(&Value::Int(format!("1{}1", "0".repeat(399))))
    );
}

#[test]
fn a_sum_past_the_rendering_digit_limit_reports_null() {
    let big = format!("5{}", "0".repeat(4299));
    let mut first = tape_one();
    let mut second = tape_two();
    for tape in [&mut first, &mut second] {
        let run = tape.get_mut("run").unwrap();
        set_key(
            run,
            "totals",
            object(vec![("big", Value::Int(big.clone()))]),
        );
        set_key(
            run,
            "skipped_lines",
            object(vec![("big", Value::Int(big.clone()))]),
        );
    }
    let report = rollup_report(
        records(&[("a.json", first.clone()), ("b.json", second.clone())]),
        "d",
    );
    assert_eq!(
        report.get("totals"),
        Some(&object(vec![("big", Value::Null)]))
    );
    assert_eq!(
        report.get("skipped_lines"),
        Some(&object(vec![("big", Value::Null)]))
    );
    for tape in [&mut first, &mut second] {
        let run = tape.get_mut("run").unwrap();
        set_key(run, "redacted_values", Value::Int(big.clone()));
    }
    let report = rollup_report(records(&[("a.json", first), ("b.json", second)]), "d");
    assert_eq!(
        report.get("redacted_values").unwrap().get("total"),
        Some(&Value::Null)
    );
}

#[test]
#[should_panic(expected = "x.json")]
fn a_record_both_tape_and_exclusion_is_refused() {
    let tape = tape_one();
    let both = object(vec![
        ("file", text("x.json")),
        ("tape", tape.clone()),
        ("reason", text("invalid")),
        ("digest", text(&tape_digest(&tape).unwrap())),
    ]);
    let _ = rollup_report(vec![both], "d");
}

#[test]
#[should_panic(expected = "x.json")]
fn a_rolled_record_without_a_tape_is_refused() {
    let record = object(vec![
        ("file", text("x.json")),
        ("tape", Value::Null),
        ("digest", text(&"0".repeat(64))),
    ]);
    let _ = rollup_report(vec![record], "d");
}

#[test]
#[should_panic(expected = "x.json")]
fn a_rolled_record_without_a_digest_is_refused() {
    let record = object(vec![("file", text("x.json")), ("tape", tape_one())]);
    let _ = rollup_report(vec![record], "d");
}

#[test]
#[should_panic(expected = "x.json")]
fn a_record_neither_tape_nor_exclusion_is_refused() {
    let record = object(vec![("file", text("x.json"))]);
    let _ = rollup_report(vec![record], "d");
}

#[test]
fn an_exclusions_defects_are_copied_not_aliased() {
    // The report owns its values in the port, so a
    // mutation of the report cannot reach the record;
    // the pinned fact is the report carries the defects.
    let record = object(vec![
        ("file", text("x.json")),
        ("reason", text("invalid")),
        ("defects", Value::Array(vec![text("one defect")])),
    ]);
    let report = rollup_report(vec![record], "d");
    assert_eq!(
        report.get("excluded").unwrap().as_array().unwrap()[0].get("defects"),
        Some(&Value::Array(vec![text("one defect")]))
    );
}

// ---- the CLI, exercised through the built binary ----

fn tmpdir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("flighttape-rollup-{}-{}", std::process::id(), tag));
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

fn excluded_entry<'a>(report: &'a Value, file: &str) -> &'a Value {
    report
        .get("excluded")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry.get("file") == Some(&text(file)))
        .unwrap()
}

#[test]
fn cli_rollup_reports_a_mixed_directory() {
    let dir = tmpdir("mixed");
    std::fs::write(dir.join("good.json"), dumps_indent(&tape_one())).unwrap();
    let mut broken = tape_two();
    {
        let events = match broken.get_mut("events") {
            Some(Value::Array(events)) => events,
            _ => panic!("events array"),
        };
        set_key(&mut events[0], "seq", int("7"));
    }
    std::fs::write(dir.join("bad.json"), dumps_indent(&broken)).unwrap();
    std::fs::write(dir.join("raw.json"), [0xff, 0xfe, b' ', b'n']).unwrap();
    let surrogate_text = dumps_indent(&tape_two()).replace("hey", "\\ud800");
    std::fs::write(dir.join("surrogate.json"), &surrogate_text).unwrap();
    std::fs::create_dir(dir.join("folder.json")).unwrap();
    std::fs::write(dir.join("notes.txt"), "not a tape").unwrap();
    let dir_arg = dir.to_string_lossy().into_owned();
    let (code, out, err) = run_cli(&["rollup".into(), dir_arg]);
    assert_eq!(code, 1);
    let report = parse(out.trim_end()).unwrap();
    let files: Vec<Value> = report
        .get("tapes")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry.get("file").unwrap().clone())
        .collect();
    assert_eq!(files, vec![text("good.json")]);
    assert_eq!(
        report.get("counts"),
        Some(&object(vec![
            ("tapes", int("1")),
            ("excluded", int("4")),
            ("events", int("2")),
        ]))
    );
    let excluded_files: Vec<Value> = report
        .get("excluded")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry.get("file").unwrap().clone())
        .collect();
    assert_eq!(
        excluded_files,
        vec![
            text("bad.json"),
            text("folder.json"),
            text("raw.json"),
            text("surrogate.json"),
        ]
    );
    assert_eq!(
        excluded_entry(&report, "bad.json").get("reason"),
        Some(&text("invalid"))
    );
    assert_eq!(
        excluded_entry(&report, "folder.json").get("reason"),
        Some(&text("not a file"))
    );
    assert_eq!(
        excluded_entry(&report, "raw.json").get("reason"),
        Some(&text("unreadable"))
    );
    assert_eq!(
        excluded_entry(&report, "surrogate.json").get("reason"),
        Some(&text("undigestible"))
    );
    let defects: Vec<Value> = validate_tape(&broken)
        .iter()
        .map(|defect| text(defect))
        .collect();
    assert_eq!(
        excluded_entry(&report, "bad.json").get("defects"),
        Some(&Value::Array(defects))
    );
    assert_eq!(
        excluded_entry(&report, "raw.json").get("error"),
        Some(&text("the file is not valid UTF-8"))
    );
    let undigestible_error = excluded_entry(&report, "surrogate.json")
        .get("error")
        .unwrap();
    match undigestible_error {
        Value::Str(message) => {
            assert!(message.as_str().unwrap().contains("cannot be encoded"))
        }
        _ => panic!("error is a string"),
    }
    for name in ["raw.json", "bad.json", "folder.json", "surrogate.json"] {
        assert!(err.contains(name), "stderr {}", err);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_rollup_of_an_empty_directory() {
    let dir = tmpdir("empty");
    let (code, out, _) = run_cli(&["rollup".into(), dir.to_string_lossy().into_owned()]);
    assert_eq!(code, 0);
    let report = parse(out.trim_end()).unwrap();
    assert_eq!(report.get("tapes"), Some(&Value::Array(vec![])));
    assert_eq!(report.get("excluded"), Some(&Value::Array(vec![])));
    assert_eq!(
        report.get("counts"),
        Some(&object(vec![
            ("tapes", int("0")),
            ("excluded", int("0")),
            ("events", int("0")),
        ]))
    );
    assert_eq!(
        report.get("redacted_values"),
        Some(&object(vec![
            ("total", int("0")),
            ("non_numeric", int("0")),
        ]))
    );
    assert_eq!(
        report.get("duration_seconds"),
        Some(&object(vec![
            ("total", Value::Float(0.0)),
            ("known", int("0")),
            ("unknown", int("0")),
        ]))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_rollup_refuses_a_missing_directory() {
    let dir = tmpdir("missing-base");
    let missing = dir.join("nope").to_string_lossy().into_owned();
    let (code, out, err) = run_cli(&["rollup".into(), missing]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.contains("no such source directory"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_rollup_output_is_byte_identical_across_runs() {
    let dir = tmpdir("twice");
    std::fs::write(dir.join("b.json"), dumps_indent(&tape_two())).unwrap();
    std::fs::write(dir.join("a.json"), dumps_indent(&tape_one())).unwrap();
    let arg = dir.to_string_lossy().into_owned();
    let (code, printed, _) = run_cli(&["rollup".into(), arg.clone()]);
    assert_eq!(code, 0);
    let (code, again, _) = run_cli(&["rollup".into(), arg]);
    assert_eq!(code, 0);
    assert_eq!(again, printed);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_rollup_names_a_null_tape_as_invalid() {
    let dir = tmpdir("null");
    std::fs::write(dir.join("null.json"), "null\n").unwrap();
    std::fs::write(dir.join("good.json"), dumps_indent(&tape_one())).unwrap();
    let (code, out, _) = run_cli(&["rollup".into(), dir.to_string_lossy().into_owned()]);
    assert_eq!(code, 1);
    let report = parse(out.trim_end()).unwrap();
    assert_eq!(
        report.get("counts"),
        Some(&object(vec![
            ("tapes", int("1")),
            ("excluded", int("1")),
            ("events", int("2")),
        ]))
    );
    let excluded = &report.get("excluded").unwrap().as_array().unwrap()[0];
    assert_eq!(excluded.get("file"), Some(&text("null.json")));
    assert_eq!(excluded.get("reason"), Some(&text("invalid")));
    let defects: Vec<Value> = flighttape::validate::validate_tape(&Value::Null)
        .iter()
        .map(|defect| text(defect))
        .collect();
    assert_eq!(excluded.get("defects"), Some(&Value::Array(defects)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_rollup_refuses_a_path_that_is_a_file() {
    let dir = tmpdir("afile");
    let target = dir.join("lonely.json");
    std::fs::write(&target, dumps_indent(&tape_one())).unwrap();
    let (code, out, err) = run_cli(&["rollup".into(), target.to_string_lossy().into_owned()]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.contains("no such source directory"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_rollup_of_a_directory_without_json_files() {
    let dir = tmpdir("nojson");
    std::fs::write(dir.join("notes.txt"), "not a tape").unwrap();
    std::fs::write(dir.join("data.JSON"), dumps_indent(&tape_one())).unwrap();
    let (code, out, _) = run_cli(&["rollup".into(), dir.to_string_lossy().into_owned()]);
    assert_eq!(code, 0);
    let report = parse(out.trim_end()).unwrap();
    assert_eq!(
        report.get("counts"),
        Some(&object(vec![
            ("tapes", int("0")),
            ("excluded", int("0")),
            ("events", int("0")),
        ]))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_rollup_renders_a_report_despite_a_huge_sum() {
    let dir = tmpdir("huge");
    let big = format!("5{}", "0".repeat(4299));
    for name in ["a.json", "b.json"] {
        let mut tape = tape_one();
        let run = tape.get_mut("run").unwrap();
        set_key(
            run,
            "totals",
            object(vec![("big", Value::Int(big.clone()))]),
        );
        std::fs::write(dir.join(name), dumps_indent(&tape)).unwrap();
    }
    let (code, out, _) = run_cli(&["rollup".into(), dir.to_string_lossy().into_owned()]);
    assert_eq!(code, 0);
    let report = parse(out.trim_end()).unwrap();
    assert_eq!(report.get("counts").unwrap().get("tapes"), Some(&int("2")));
    assert_eq!(
        report.get("totals"),
        Some(&object(vec![("big", Value::Null)]))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_empty_rollup_reports_a_positive_zero_duration_total() {
    // Red team m1: with no known durations the total
    // is zero with a positive sign, as Python's sum
    // from zero renders it.
    let report = rollup_report(records(&[]), "dir");
    let total = report
        .get("duration_seconds")
        .and_then(|durations| durations.get("total"))
        .expect("the report carries a duration total");
    match total {
        Value::Float(value) => assert_eq!(
            value.to_bits(),
            0.0f64.to_bits(),
            "the total is positive zero"
        ),
        other => panic!("the total is not a float: {:?}", other),
    }
}
