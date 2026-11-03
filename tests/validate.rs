//! Tests for the validator, ported from the Python
//! suite file for file. Tapes are built through the
//! crate's own JSON reader, as files arrive.

use flighttape::json::parse;
use flighttape::validate::validate_tape;
use flighttape::value::Value;

fn good_tape() -> Value {
    parse(
        r#"{
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "s1",
                "totals": {"tool_calls": 1}},
        "events": [
            {"seq": 0, "kind": "tool_call", "sidechain": false,
             "call_id": "c1", "tool": "Read",
             "timestamp": "2026-01-01T09:00:05.000Z"},
            {"seq": 1, "kind": "tool_result", "sidechain": false,
             "call_id": "c1", "timestamp": "2026-01-01T09:00:06.000Z"}
        ],
        "outcome": {"status": "completed"}
    }"#,
    )
    .unwrap()
}

fn set_path(tape: &mut Value, path: &[&str], value: Value) {
    if path.is_empty() {
        *tape = value;
        return;
    }
    let head = path[0];
    match tape {
        Value::Object(pairs) => {
            if !pairs.iter().any(|(name, _)| name == &head) {
                pairs.push((flighttape::value::Text::from_str(head), Value::Null));
            }
            let entry = &mut pairs.iter_mut().find(|(name, _)| name == &head).unwrap().1;
            set_path(entry, &path[1..], value);
        }
        Value::Array(items) => {
            let index: usize = head.parse().unwrap();
            set_path(&mut items[index], &path[1..], value);
        }
        _ => panic!("path crosses a scalar"),
    }
}

fn has(defects: &[String], needle: &str) -> bool {
    defects.iter().any(|defect| defect.contains(needle))
}

#[test]
fn good_tape_is_valid() {
    assert_eq!(validate_tape(&good_tape()), Vec::<String>::new());
}

#[test]
fn wrong_version_is_rejected() {
    let mut tape = good_tape();
    set_path(&mut tape, &["tape_version"], parse("\"9.9\"").unwrap());
    assert!(has(&validate_tape(&tape), "tape_version"));
}

#[test]
fn seq_gap_is_rejected() {
    let mut tape = good_tape();
    set_path(&mut tape, &["events", "1", "seq"], parse("7").unwrap());
    assert!(has(&validate_tape(&tape), "seq"));
}

#[test]
fn orphan_tool_result_is_rejected() {
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["events", "1", "call_id"],
        parse("\"nope\"").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "answers no earlier"));
}

#[test]
fn backward_timestamp_is_rejected() {
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["events", "1", "timestamp"],
        parse("\"2026-01-01T08:00:00.000Z\"").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "backward"));
}

#[test]
fn duplicate_call_id_is_rejected() {
    let mut tape = good_tape();
    if let Some(Value::Array(events)) = tape.get_mut("events") {
        events.push(
            parse(r#"{"seq": 2, "kind": "tool_call", "sidechain": false, "call_id": "c1", "tool": "Read"}"#)
                .unwrap(),
        );
    }
    assert!(has(&validate_tape(&tape), "used twice"));
}

#[test]
fn bad_outcome_is_rejected() {
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["outcome", "status"],
        parse("\"shrug\"").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "outcome.status"));
}

#[test]
fn negative_total_is_rejected() {
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["run", "totals", "tool_calls"],
        parse("-1").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "totals"));
}

#[test]
fn all_defects_are_named_in_one_pass() {
    let mut tape = good_tape();
    set_path(&mut tape, &["tape_version"], parse("\"9.9\"").unwrap());
    set_path(
        &mut tape,
        &["outcome", "status"],
        parse("\"shrug\"").unwrap(),
    );
    set_path(
        &mut tape,
        &["events", "1", "call_id"],
        parse("\"nope\"").unwrap(),
    );
    let defects = validate_tape(&tape);
    assert!(has(&defects, "tape_version"));
    assert!(has(&defects, "outcome.status"));
    assert!(has(&defects, "answers no earlier"));
    assert!(defects.len() >= 3);
}

#[test]
fn skipped_lines_values_are_checked() {
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["run", "skipped_lines"],
        parse("{\"system\": -3}").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "skipped_lines"));
    set_path(
        &mut tape,
        &["run", "skipped_lines"],
        parse("{\"system\": \"many\"}").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "skipped_lines"));
}

#[test]
fn started_at_must_be_a_string() {
    let mut tape = good_tape();
    set_path(&mut tape, &["run", "started_at"], parse("12345").unwrap());
    assert!(has(&validate_tape(&tape), "started_at"));
}

#[test]
fn boolean_seq_is_rejected() {
    let mut tape = good_tape();
    set_path(&mut tape, &["events", "1", "seq"], parse("true").unwrap());
    assert!(has(&validate_tape(&tape), "seq"));
}

#[test]
fn integer_call_id_on_a_note_is_rejected() {
    let mut tape = good_tape();
    if let Some(Value::Array(events)) = tape.get_mut("events") {
        events.push(
            parse(r#"{"seq": 2, "kind": "note", "sidechain": false, "call_id": 7}"#).unwrap(),
        );
    }
    assert!(has(&validate_tape(&tape), "call_id must be a string"));
}

#[test]
fn boolean_total_is_rejected() {
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["run", "totals", "tool_calls"],
        parse("true").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "totals"));
}

#[test]
fn non_boolean_sidechain_is_rejected() {
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["events", "0", "sidechain"],
        parse("\"yes\"").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "sidechain"));
}

#[test]
fn offset_timestamps_compare_chronologically() {
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["events", "0", "timestamp"],
        parse("\"2026-01-01T10:00:00+02:00\"").unwrap(),
    );
    set_path(
        &mut tape,
        &["events", "1", "timestamp"],
        parse("\"2026-01-01T09:00:00.000Z\"").unwrap(),
    );
    assert_eq!(validate_tape(&tape), Vec::<String>::new());
    set_path(
        &mut tape,
        &["events", "1", "timestamp"],
        parse("\"2026-01-01T07:00:00.000Z\"").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "backward"));
}

#[test]
fn unparseable_timestamp_is_rejected() {
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["events", "0", "timestamp"],
        parse("\"next tuesday\"").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "ISO 8601"));
}

#[test]
fn basis_must_be_a_named_rule_that_pairs_with_the_status() {
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["outcome", "basis"],
        parse("\"final-assistant-message\"").unwrap(),
    );
    assert_eq!(validate_tape(&tape), Vec::<String>::new());
    for bad in ["\"\"", "7", "null", "[\"x\"]", "\"banana\"", "\" \""] {
        let mut tape = good_tape();
        set_path(&mut tape, &["outcome", "basis"], parse(bad).unwrap());
        assert!(has(&validate_tape(&tape), "outcome.basis"), "basis {}", bad);
    }
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["outcome", "basis"],
        parse("\"final-error-result\"").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "does not go with"));
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["outcome"],
        parse("{\"status\": \"unknown\", \"basis\": \"no-events\"}").unwrap(),
    );
    assert_eq!(validate_tape(&tape), Vec::<String>::new());
}

#[test]
fn is_error_must_be_a_boolean_when_present() {
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["events", "1", "is_error"],
        parse("true").unwrap(),
    );
    assert_eq!(validate_tape(&tape), Vec::<String>::new());
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["events", "1", "is_error"],
        parse("\"false\"").unwrap(),
    );
    assert!(has(&validate_tape(&tape), "is_error"));
}

#[test]
fn a_non_finite_number_is_a_located_defect() {
    let mut tape = good_tape();
    set_path(&mut tape, &["events", "0", "score"], Value::Float(f64::NAN));
    let defects = validate_tape(&tape);
    assert!(defects
        .iter()
        .any(|defect| defect.contains("non-finite") && defect.contains("events[0]")));
}

#[test]
fn a_non_finite_number_is_named_on_an_early_return_path() {
    let mut tape = good_tape();
    set_path(&mut tape, &["events"], parse("\"not a list\"").unwrap());
    set_path(&mut tape, &["run", "cost"], Value::Float(f64::INFINITY));
    assert!(has(&validate_tape(&tape), "non-finite"));
}

#[test]
fn a_finite_tape_earns_no_non_finite_defect() {
    assert!(!has(&validate_tape(&good_tape()), "non-finite"));
}

#[test]
fn a_shared_substructure_is_named_at_every_location() {
    // The same NaN-bearing object filed under two keys
    // earns its located defect at both paths.
    let tape = parse(r#"{"a": {"reading": NaN}, "b": {"reading": NaN}}"#).unwrap();
    let defects = validate_tape(&tape);
    let named: Vec<&String> = defects
        .iter()
        .filter(|defect| defect.contains("non-finite"))
        .collect();
    assert!(
        named.iter().any(|defect| defect.contains("a.reading")),
        "{:?}",
        named
    );
    assert!(
        named.iter().any(|defect| defect.contains("b.reading")),
        "{:?}",
        named
    );
}

#[test]
fn walk_defects_arrive_in_document_order() {
    let tape = parse(r#"{"z": NaN, "a": NaN}"#).unwrap();
    let defects = validate_tape(&tape);
    let named: Vec<&str> = defects
        .iter()
        .filter(|defect| defect.contains("non-finite"))
        .map(|defect| defect.as_str())
        .collect();
    assert_eq!(
        named,
        ["z is a non-finite number", "a is a non-finite number"]
    );
}

#[test]
fn an_infinity_in_a_run_totals_field_is_named() {
    // The Python case places Infinity at run.totals.cost
    // of the fixture tape and asserts the walk names the
    // totals location and the canonical form refuses.
    let mut tape = good_tape();
    set_path(
        &mut tape,
        &["run", "totals", "cost"],
        Value::Float(f64::INFINITY),
    );
    let defects = validate_tape(&tape);
    assert!(
        defects
            .iter()
            .any(|defect| { defect.contains("non-finite") && defect.contains("totals") }),
        "{:?}",
        defects
    );
    assert!(flighttape::identity::canonical_bytes(&tape).is_err());
}
