//! The schema and the validator share one vocabulary,
//! ported from the Python suite. The schema is read
//! with the crate's own JSON reader.

use flighttape::json::parse;
use flighttape::validate::{
    validate_tape, BASES, EVENT_KINDS, OUTCOMES,
};
use flighttape::value::Value;
use flighttape::TAPE_VERSION;

fn load_schema() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/schema/tape.schema.json");
    let text = std::fs::read_to_string(path).unwrap();
    parse(&text).unwrap()
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
                pairs.push((
                    flighttape::value::Text::from_str(head),
                    Value::Null,
                ));
            }
            let entry = &mut pairs
                .iter_mut()
                .find(|(name, _)| name == &head)
                .unwrap()
                .1;
            set_path(entry, &path[1..], value);
        }
        Value::Array(items) => {
            let index: usize = head.parse().unwrap();
            set_path(&mut items[index], &path[1..], value);
        }
        _ => panic!("path crosses a scalar"),
    }
}

fn strings(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .map(|value| match value {
            Value::Str(text) => text.as_str().unwrap(),
            _ => panic!("schema enum holds text"),
        })
        .collect()
}

fn at<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    let mut current = value;
    for part in path {
        current = current.get(part).expect("schema path exists");
    }
    current
}

#[test]
fn schema_and_validator_share_the_vocabulary() {
    let schema = load_schema();
    assert_eq!(
        at(&schema, &["properties", "tape_version", "const"]),
        &Value::Str(flighttape::value::Text::from_str(TAPE_VERSION))
    );
    let kinds = strings(
        at(&schema, &["properties", "events", "items", "properties", "kind", "enum"])
            .as_array()
            .unwrap(),
    );
    let mut expected: Vec<String> = EVENT_KINDS.iter().map(|kind| kind.to_string()).collect();
    let mut actual = kinds.clone();
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
    let outcomes = strings(
        at(&schema, &["properties", "outcome", "properties", "status", "enum"])
            .as_array()
            .unwrap(),
    );
    let mut expected: Vec<String> = OUTCOMES.iter().map(|name| name.to_string()).collect();
    let mut actual = outcomes;
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
    let bases = strings(
        at(&schema, &["properties", "outcome", "properties", "basis", "enum"])
            .as_array()
            .unwrap(),
    );
    let mut expected: Vec<String> = BASES.iter().map(|name| name.to_string()).collect();
    let mut actual = bases;
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn schema_requires_non_empty_call_and_tool() {
    let schema = load_schema();
    let props = at(&schema, &["properties", "events", "items", "properties"]);
    assert_eq!(
        props.get("call_id").unwrap().get("minLength"),
        Some(&Value::Int("1".to_string()))
    );
    assert_eq!(
        props.get("tool").unwrap().get("minLength"),
        Some(&Value::Int("1".to_string()))
    );
}

#[test]
fn validator_rejects_the_schema_probes() {
    // A valid base tape in the schema's shape, then the
    // three probes the Python case mutates into it: a
    // negative skipped_lines count, a numeric started_at,
    // and a numeric call_id. Each must earn a defect.
    let base = parse(
        r#"{
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "s1"},
        "events": [{"seq": 0, "kind": "note", "sidechain": false}],
        "outcome": {"status": "completed"}
    }"#,
    )
    .unwrap();
    assert_eq!(validate_tape(&base), Vec::<String>::new());
    let mut probe = base.clone();
    set_path(
        &mut probe,
        &["run", "skipped_lines"],
        parse(r#"{"system": -3}"#).unwrap(),
    );
    assert!(!validate_tape(&probe).is_empty());
    let mut probe = base.clone();
    set_path(
        &mut probe,
        &["run", "started_at"],
        Value::Int("12345".to_string()),
    );
    assert!(!validate_tape(&probe).is_empty());
    let mut probe = base.clone();
    set_path(
        &mut probe,
        &["events", "0", "call_id"],
        Value::Int("7".to_string()),
    );
    assert!(!validate_tape(&probe).is_empty());
}

#[test]
fn schema_states_the_basis_field() {
    let schema = load_schema();
    let basis = at(
        &schema,
        &["properties", "outcome", "properties", "basis"],
    );
    assert_eq!(
        basis.get("type"),
        Some(&Value::Str(flighttape::value::Text::from_str("string")))
    );
    let bases = strings(basis.get("enum").unwrap().as_array().unwrap());
    let mut expected: Vec<String> =
        BASES.iter().map(|name| name.to_string()).collect();
    let mut actual = bases;
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn schema_types_is_error_as_a_boolean() {
    let schema = load_schema();
    let props = at(
        &schema,
        &["properties", "events", "items", "properties"],
    );
    let expected = parse(r#"{"type": "boolean"}"#).unwrap();
    assert_eq!(props.get("is_error"), Some(&expected));
}
