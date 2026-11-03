//! Scale pins (red team C1): inputs of realistic
//! size parse and digest to the Python's own values.
//! The expected digests were taken from the Python
//! on main over byte-identical content. Before the
//! repair these inputs took minutes in the reader
//! (a validation of the remaining input per string
//! character, a key search per key); the suite
//! finishing at all is the regression signal, and
//! the digests pin that the faster reader reads the
//! same values.

use flighttape::identity::tape_digest;
use flighttape::json::parse;

#[test]
fn a_wide_object_parses_and_digests() {
    let mut text = String::from("{");
    for index in 0..100_000 {
        if index > 0 {
            text.push(',');
        }
        text.push_str(&format!("\"k{}\":{}", index, index));
    }
    text.push('}');
    let value = parse(&text).expect("the wide object parses");
    let pairs = value.as_object().expect("an object");
    assert_eq!(pairs.len(), 100_000);
    assert_eq!(
        tape_digest(&value).expect("the wide object digests"),
        "9a72499a287bfbbfc50e0ca9f363ba47aea6b6c7d98ad9d4810472259beb265a"
    );
}

#[test]
fn many_events_parse_and_digest() {
    let mut text = String::from("{\"events\":[");
    for index in 0..4_000 {
        if index > 0 {
            text.push(',');
        }
        text.push_str(&format!(
            "{{\"kind\":\"message\",\"n\":{},\"text\":\"{}\"}}",
            index,
            "x".repeat(40)
        ));
    }
    text.push_str("]}");
    let value = parse(&text).expect("the events parse");
    assert_eq!(
        tape_digest(&value).expect("the events digest"),
        "70a30aca9190b339d1abd6c3c19757e82894855fbfe00baa2ade5618dd925243"
    );
}

#[test]
fn a_long_string_parses_and_digests() {
    let text = format!("{{\"v\":\"{}\"}}", "a".repeat(400_000));
    let value = parse(&text).expect("the long string parses");
    assert_eq!(
        tape_digest(&value).expect("the long string digests"),
        "1e950f5a71482db6a3f70e45a22c8a257f1c5736b54447939b080bc0897d7301"
    );
}
