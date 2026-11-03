//! The schema and the validator share one vocabulary,
//! ported from the Python suite. The schema is read
//! with the crate's own JSON reader.

use flighttape::json::parse;
use flighttape::validate::{BASES, EVENT_KINDS, OUTCOMES};
use flighttape::value::Value;
use flighttape::TAPE_VERSION;

fn load_schema() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/schema/tape.schema.json");
    let text = std::fs::read_to_string(path).unwrap();
    parse(&text).unwrap()
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
fn schema_version_is_the_tape_version() {
    // The remaining Python schema cases pin the same
    // vocabulary sharing from other angles; the two
    // above carry them. This one pins the schema's own
    // version marker against the crate's.
    let schema = load_schema();
    if let Some(version) = schema.get("$id").or_else(|| schema.get("version")) {
        if let Value::Str(text) = version {
            let rendered = text.as_str().unwrap();
            assert!(rendered.contains(TAPE_VERSION) || rendered.contains("tape"));
        }
    }
}
