//! Tests for the JSON reader (FT-51): it accepts what
//! Python's json accepts for a tape file and refuses
//! the rest in the loader's own terms.

use flighttape::json::{parse, ParseError};
use flighttape::value::Value;

#[test]
fn containers_with_content_parse() {
    assert!(parse("{\"a\": 1}").is_ok());
    assert!(parse("[1, 2, 3]").is_ok());
    assert!(parse("{\"a\": {\"b\": [true, null, \"x\"]}}").is_ok());
    assert!(parse("[[[[1]]]]").is_ok());
}

#[test]
fn the_python_literals_parse_as_non_finite_numbers() {
    assert_eq!(parse("NaN").unwrap(), Value::Float(f64::NAN));
    assert_eq!(parse("Infinity").unwrap(), Value::Float(f64::INFINITY));
    assert_eq!(parse("-Infinity").unwrap(), Value::Float(f64::NEG_INFINITY));
    assert!(parse("{\"score\": NaN}").is_ok());
}

#[test]
fn duplicate_keys_keep_the_first_position_and_last_value() {
    let value = parse("{\"a\": 1, \"b\": 2, \"a\": 3}").unwrap();
    let pairs = value.as_object().unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].1, Value::Int("3".to_string()));
    assert_eq!(pairs[1].1, Value::Int("2".to_string()));
}

#[test]
fn integers_are_exact_to_the_limit_and_refused_past_it() {
    let big = "9".repeat(4300);
    match parse(&big).unwrap() {
        Value::Int(digits) => assert_eq!(digits.len(), 4300),
        other => panic!("expected an integer, got {:?}", other),
    }
    let too_big = "9".repeat(4301);
    assert_eq!(parse(&too_big), Err(ParseError::TooLarge));
    assert_eq!(parse("-0").unwrap(), Value::Int("0".to_string()));
}

#[test]
fn deep_nesting_is_counted_whole() {
    let depth = 3000;
    let text = format!("{}1{}", "{\"a\":".repeat(depth), "}".repeat(depth));
    assert!(parse(&text).is_ok());
    let arrays = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
    assert!(parse(&arrays).is_ok());
}

#[test]
fn malformed_inputs_refuse() {
    for bad in [
        "",
        "{",
        "[1,]",
        "{\"a\":}",
        "{\"a\" 1}",
        "01",
        "+1",
        ".5",
        "1e",
        "tru",
        "{\"a\": 1} {\"b\": 2}",
        "\"unterminated",
        "\"bad \\q escape\"",
        "[1 2]",
        "{'a': 1}",
    ] {
        assert_eq!(parse(bad), Err(ParseError::Invalid), "input {:?}", bad);
    }
}

#[test]
fn strings_carry_the_escapes_and_pairs() {
    let value = parse(r#""a\nbé\ud83d\ude00""#).unwrap();
    match value {
        Value::Str(text) => {
            assert_eq!(text.as_str().unwrap(), "a\nbé\u{1F600}");
        }
        other => panic!("expected text, got {:?}", other),
    }
    // An unpaired surrogate is kept as its unit, as
    // Python keeps it; canonicalization refuses it.
    let value = parse(r#""\ud800""#).unwrap();
    match value {
        Value::Str(text) => {
            assert_eq!(text.units(), &[0xD800]);
            assert!(text.as_str().is_none());
        }
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn floats_parse_with_python_semantics() {
    assert_eq!(parse("1e2").unwrap(), Value::Float(100.0));
    assert_eq!(parse("2.0").unwrap(), Value::Float(2.0));
    assert_eq!(parse("2").unwrap(), Value::Int("2".to_string()));
    assert_eq!(parse("1e400").unwrap(), Value::Float(f64::INFINITY));
    assert_eq!(parse("-0.0").unwrap(), Value::Float(-0.0));
}
