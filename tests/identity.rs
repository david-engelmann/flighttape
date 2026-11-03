//! Tests for tape identity, FT-26 to FT-30, ported
//! from the Python suite. The number table fails if
//! the rendering drifts from the ECMAScript algorithm,
//! the golden digest fails if the canonical form
//! changes in any byte, and the refusal tests fail if
//! a non-finite number or an over-deep structure
//! renders instead of being named.

use flighttape::identity::{canonical_bytes, render_number, tape_digest};
use flighttape::value::{Text, Value};

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(
        pairs
            .into_iter()
            .map(|(key, value)| (Text::from_str(key), value))
            .collect(),
    )
}

#[test]
fn numbers_render_as_ecmascript_renders_them() {
    let table: [(f64, &str); 25] = [
        (0.0, "0"),
        (-0.0, "0"),
        (2.0, "2"),
        (100.0, "100"),
        (1e16, "10000000000000000"),
        (1e20, "100000000000000000000"),
        (9007199254740991.0, "9007199254740991"),
        (4.35e15, "4350000000000000"),
        (0.95, "0.95"),
        (-2.5, "-2.5"),
        (0.1, "0.1"),
        (123456789.123, "123456789.123"),
        (1234567.891, "1234567.891"),
        (1.0 / 3.0, "0.3333333333333333"),
        (0.000123, "0.000123"),
        (1e-5, "0.00001"),
        (2.5e-5, "0.000025"),
        (1e-6, "0.000001"),
        (1e-7, "1e-7"),
        (1.5e-7, "1.5e-7"),
        (1e21, "1e+21"),
        (1.5e21, "1.5e+21"),
        (7e22, "7e+22"),
        (5e-324, "5e-324"),
        (1.7976931348623157e308, "1.7976931348623157e+308"),
    ];
    // The table above holds 25 entries as written in
    // the spec suite; assert each and the round trip.
    for (value, rendered) in table.iter() {
        assert_eq!(render_number(*value), *rendered, "value {}", value);
        assert_eq!(
            canonical_bytes(&Value::Float(*value)).unwrap(),
            rendered.as_bytes()
        );
        assert_eq!(rendered.parse::<f64>().unwrap(), *value);
    }
}

#[test]
fn the_swept_corpus_agrees_with_the_python_renderer() {
    let corpus = include_str!("fixtures/number_sweep.txt");
    let mut checked = 0;
    for line in corpus.lines() {
        let (bits, rendered) = line.split_once(' ').unwrap();
        let value = f64::from_bits(u64::from_str_radix(bits, 16).unwrap());
        assert_eq!(render_number(value), rendered, "bits {}", bits);
        checked += 1;
    }
    assert!(checked > 4000);
}

#[test]
fn canonical_bytes_are_exact_for_a_small_tape() {
    let tape = obj(vec![
        ("b", Value::Array(vec![
            Value::Int("1".to_string()),
            Value::Float(2.0),
            Value::Str(Text::from_str("café")),
        ])),
        ("a", obj(vec![
            ("z", Value::Null),
            ("m", Value::Bool(true)),
        ])),
    ]);
    assert_eq!(
        String::from_utf8(canonical_bytes(&tape).unwrap()).unwrap(),
        "{\"a\":{\"m\":true,\"z\":null},\"b\":[1,2,\"café\"]}"
    );
}

#[test]
fn keys_sort_by_utf16_code_units() {
    // U+FFFD (BMP) sorts before U+10000 (astral) in code
    // point order but after it in UTF-16 code units,
    // whose first unit for the astral key is 0xD800.
    let tape = obj(vec![
        ("\u{FFFD}", Value::Int("1".to_string())),
        ("\u{10000}", Value::Int("2".to_string())),
    ]);
    let rendered = String::from_utf8(canonical_bytes(&tape).unwrap()).unwrap();
    assert!(
        rendered.starts_with("{\"\u{10000}\":2,"),
        "astral key first under UTF-16 order: {}",
        rendered
    );
}

#[test]
fn unpaired_surrogates_are_refused_by_name() {
    let by_value = obj(vec![("text", Value::Str(Text::from_units(vec![0xD800])))]);
    let error = canonical_bytes(&by_value).unwrap_err();
    assert!(error.0.contains("cannot be encoded"), "{}", error.0);
    let by_key = Value::Object(vec![(Text::from_units(vec![0xD800]), Value::Int("1".to_string()))]);
    let error = canonical_bytes(&by_key).unwrap_err();
    assert!(error.0.contains("cannot be encoded"), "{}", error.0);
}

#[test]
fn an_integer_beyond_the_rendering_limit_is_refused() {
    let huge = Value::Int("1".repeat(5000));
    let error = canonical_bytes(&huge).unwrap_err();
    assert!(error.0.contains("cannot be rendered"), "{}", error.0);
    let at_limit = Value::Int("1".repeat(4300));
    assert!(canonical_bytes(&at_limit).is_ok());
}

#[test]
fn a_deeply_nested_tape_is_named_not_crashed() {
    let mut deep = obj(vec![("leaf", Value::Float(f64::NAN))]);
    for _ in 0..5000 {
        deep = obj(vec![("x", deep)]);
    }
    let error = canonical_bytes(&deep).unwrap_err();
    assert!(error.0.contains("nested"), "{}", error.0);
    let shallow = obj(vec![("x", obj(vec![("y", Value::Null)]))]);
    assert!(canonical_bytes(&shallow).is_ok());
}

#[test]
fn a_non_finite_number_is_refused() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = canonical_bytes(&Value::Float(value)).unwrap_err();
        assert!(error.0.contains("non-finite"), "{}", error.0);
    }
}

#[test]
fn digest_identifies_content_not_validity() {
    let tape = obj(vec![("not", Value::Str(Text::from_str("a tape")))]);
    assert_eq!(
        tape_digest(&tape).unwrap(),
        "ab587bf78c80e183317a77423a19a8f3b2633d98b1b3ed95b63d315b4d6f3c07"
    );
}
