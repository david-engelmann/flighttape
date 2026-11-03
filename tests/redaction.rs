//! Redaction completeness tests (FT-22 to FT-25),
//! ported from the Python suite. Every session line
//! here is synthetic and the paths invented.

use flighttape::convert::{convert_file, convert_lines};
use flighttape::value::{object, Text, Value};

const SECRET: &str = concat!("/home/synthetic", "-user/work/secret-plan.md");

fn lines(values: &[&str]) -> Vec<Option<String>> {
    values.iter().map(|line| Some(line.to_string())).collect()
}

fn line(kind: &str, content_json: &str) -> String {
    let mut out = String::from("{\"type\": \"");
    out.push_str(kind);
    out.push_str(
        "\", \"sessionId\": \"synthetic-redact\", \"timestamp\": \
         \"2026-01-01T00:00:00Z\", \"message\": {\"role\": \"",
    );
    out.push_str(kind);
    out.push_str("\", \"content\": ");
    out.push_str(content_json);
    out.push_str("}}");
    out
}

fn esc(text: &str) -> String {
    // JSON-escape a plain string for embedding.
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn user_text(text: &str) -> String {
    line("user", &format!("\"{}\"", esc(text)))
}

fn assistant_text(text: &str) -> String {
    line(
        "assistant",
        &format!("[{{\"type\": \"text\", \"text\": \"{}\"}}]", esc(text)),
    )
}

fn tool_use(input_json: &str, name: &str) -> String {
    line(
        "assistant",
        &format!(
            "[{{\"type\": \"tool_use\", \"id\": \"c1\", \"name\": \"{}\", \"input\": {}}}]",
            name, input_json
        ),
    )
}

fn tool_result(content_json: &str) -> String {
    line(
        "user",
        &format!(
            "[{{\"type\": \"tool_result\", \"tool_use_id\": \"c1\", \"content\": {}}}]",
            content_json
        ),
    )
}

fn events(tape: &Value) -> Vec<Value> {
    tape.get("events").unwrap().as_array().unwrap().clone()
}

fn redacted(tape: &Value) -> Value {
    tape.get("run")
        .and_then(|run| run.get("redacted_values"))
        .unwrap()
        .clone()
}

fn text(value: &str) -> Value {
    Value::Str(Text::from_str(value))
}

fn int(value: &str) -> Value {
    Value::Int(value.to_string())
}

fn first_text(tape: &Value) -> Value {
    events(tape)[0].get("text").unwrap().clone()
}

#[test]
fn paths_in_every_text_field_are_redacted_and_counted() {
    let tape = convert_lines(lines(&[
        &user_text(&format!("please read {} now", SECRET)),
        &tool_use(&format!("{{\"file_path\": \"{}\"}}", SECRET), "Read"),
        &tool_result(&format!("\"contents of {} follow\"", SECRET)),
        &tool_use(
            &format!("{{\"command\": \"cat {} | wc -l\"}}", SECRET),
            "Bash",
        ),
        &assistant_text(&format!("I read {} and it says hello", SECRET)),
    ]));
    let blob = flighttape::dumps::dumps_indent(&tape);
    assert!(!blob.contains("/home/synthetic-user"));
    assert_eq!(redacted(&tape), int("5"));
    let texts: Vec<Value> = events(&tape)
        .iter()
        .map(|event| {
            event
                .get("text")
                .or_else(|| event.get("content"))
                .cloned()
                .unwrap_or(text(""))
        })
        .collect();
    assert!(texts.contains(&text("please read secret-plan.md now")));
    assert!(texts.contains(&text("contents of secret-plan.md follow")));
    assert!(texts.contains(&text("I read secret-plan.md and it says hello")));
    let calls: Vec<Value> = events(&tape)
        .into_iter()
        .filter(|event| {
            event.get("kind") == Some(&text("tool_call"))
        })
        .collect();
    assert_eq!(
        calls[0].get("input"),
        Some(&object(vec![("file_path", text("secret-plan.md"))]))
    );
    assert_eq!(
        calls[1].get("input"),
        Some(&object(vec![("command", text("cat secret-plan.md | wc -l"))]))
    );
}

#[test]
fn a_path_nested_in_a_tool_input_is_redacted() {
    let tape = convert_lines(lines(&[&tool_use(
        &format!(
            "{{\"args\": [\"run\", {{\"script\": \"python {}\"}}]}}",
            SECRET
        ),
        "Read",
    )]));
    assert_eq!(
        events(&tape)[0].get("input"),
        Some(&object(vec![(
            "args",
            Value::Array(vec![
                text("run"),
                object(vec![("script", text("python secret-plan.md"))]),
            ]),
        )]))
    );
    assert_eq!(redacted(&tape), int("1"));
}

#[test]
fn two_paths_in_one_text_are_each_counted() {
    let tape = convert_lines(lines(&[&user_text(
        "compare /home/synthetic-user/a/first.md with /home/synthetic-user/b/second.md today",
    )]));
    assert_eq!(
        first_text(&tape),
        text("compare first.md with second.md today")
    );
    assert_eq!(redacted(&tape), int("2"));
}

#[test]
fn boundary_shapes_keep_their_punctuation() {
    let cases: [(&str, &str); 10] = [
        ("open \"SECRET\" please", "open \"secret-plan.md\" please"),
        ("open 'SECRET' please", "open 'secret-plan.md' please"),
        ("run `SECRET` now", "run `secret-plan.md` now"),
        ("(see SECRET)", "(see secret-plan.md)"),
        ("I read SECRET.", "I read secret-plan.md."),
        ("I read SECRET, then left", "I read secret-plan.md, then left"),
        ("file=SECRET", "file=secret-plan.md"),
        ("path:SECRET", "path:secret-plan.md"),
        ("SECRET", "secret-plan.md"),
        ("/home/synthetic-user/projects/", "projects"),
    ];
    for (given, expected) in cases {
        let given = given.replace("SECRET", SECRET);
        let tape = convert_lines(lines(&[&user_text(&given)]));
        assert_eq!(first_text(&tape), text(expected), "given {}", given);
        assert_eq!(redacted(&tape), int("1"), "given {}", given);
    }
}

#[test]
fn text_without_absolute_paths_is_byte_identical() {
    let samples = [
        "the and/or choice stays, as does input/output",
        "on 2026/11/01 the ratio was 1/2 of a/b/c",
        "see https://example.com/some/deep/page for details",
        "mirror at //cdn.example.com/assets/app.js today",
        "the /etc directory and a lone / are not paths here",
        "a Windows path C:\\Users\\synthetic\\file.txt stays",
        "no slashes at all in this one",
        "a file URL file:///home/synthetic/work/notes.md stays whole",
        "cmd>/alpha/one.md is outside the stated prefix rule",
        "<file>/alpha/one.md</file> is outside the stated prefix rule",
        "read(/alpha/one.md) is outside the stated prefix rule",
    ];
    for sample in samples {
        let tape = convert_lines(lines(&[&user_text(sample)]));
        assert_eq!(first_text(&tape), text(sample), "sample {}", sample);
        assert_eq!(redacted(&tape), int("0"), "sample {}", sample);
    }
}

#[test]
fn whole_value_paths_match_the_original_ft8_behaviour() {
    let tape = convert_lines(lines(&[&tool_use(
        "{\"path\": \"/home/synthetic-user/work/notes.md\"}",
        "Read",
    )]));
    assert_eq!(
        events(&tape)[0].get("input"),
        Some(&object(vec![("path", text("notes.md"))]))
    );
    assert_eq!(redacted(&tape), int("1"));
}

#[test]
fn a_tool_result_in_block_form_is_redacted() {
    let tape = convert_lines(lines(&[&tool_result(&format!(
        "[{{\"type\": \"text\", \"text\": \"contents of {} follow\"}}]",
        SECRET
    ))]));
    assert_eq!(
        events(&tape)[0].get("content"),
        Some(&text("contents of secret-plan.md follow"))
    );
    assert_eq!(redacted(&tape), int("1"));
}

#[test]
fn tool_input_keys_are_redacted_like_values() {
    let tape = convert_lines(lines(&[&tool_use(
        &format!(
            "{{\"mapping\": {{\"{}\": \"v1\", \"plain\": \"{}\"}}}}",
            SECRET, SECRET
        ),
        "Read",
    )]));
    assert_eq!(
        events(&tape)[0].get("input"),
        Some(&object(vec![(
            "mapping",
            object(vec![
                ("secret-plan.md", text("v1")),
                ("plain", text("secret-plan.md")),
            ]),
        )]))
    );
    assert_eq!(redacted(&tape), int("2"));
    let deep = convert_lines(lines(&[&tool_use(
        &format!(
            "{{\"outer\": [{{\"deep\": {{\"{}\": true}}}}]}}",
            SECRET
        ),
        "Read",
    )]));
    assert_eq!(
        deep.get("events").unwrap().as_array().unwrap()[0].get("input"),
        Some(&object(vec![(
            "outer",
            Value::Array(vec![object(vec![(
                "deep",
                object(vec![("secret-plan.md", Value::Bool(true))]),
            )])]),
        )]))
    );
    assert_eq!(redacted(&deep), int("1"));
}

#[test]
fn keys_that_redact_to_the_same_name_merge() {
    let tape = convert_lines(lines(&[&tool_use(
        "{\"/home/synthetic-user/a/same.md\": 1, \"/home/synthetic-user/b/same.md\": 2}",
        "Read",
    )]));
    assert_eq!(
        events(&tape)[0].get("input"),
        Some(&object(vec![("same.md", int("2"))]))
    );
    assert_eq!(redacted(&tape), int("2"));
}

#[test]
fn a_non_string_text_value_is_carried_and_its_leaves_redacted() {
    for (value_json, expected) in [
        ("123", int("123")),
        ("{\"x\": 1}", object(vec![("x", int("1"))])),
        ("true", Value::Bool(true)),
    ] {
        let tape = convert_lines(lines(&[&line(
            "assistant",
            &format!("[{{\"type\": \"text\", \"text\": {}}}]", value_json),
        )]));
        assert_eq!(events(&tape)[0].get("text"), Some(&expected));
        assert_eq!(redacted(&tape), int("0"));
    }
    let tape = convert_lines(lines(&[&line(
        "assistant",
        &format!("[{{\"type\": \"text\", \"text\": [\"{}\"]}}]", SECRET),
    )]));
    assert_eq!(
        events(&tape)[0].get("text"),
        Some(&Value::Array(vec![text("secret-plan.md")]))
    );
    assert_eq!(redacted(&tape), int("1"));
    let tape = convert_lines(lines(&[&line(
        "assistant",
        &format!(
            "[{{\"type\": \"text\", \"text\": {{\"p\": \"{}\"}}}}]",
            SECRET
        ),
    )]));
    assert_eq!(
        events(&tape)[0].get("text"),
        Some(&object(vec![("p", text("secret-plan.md"))]))
    );
    assert_eq!(redacted(&tape), int("1"));
}

#[test]
fn paths_glued_into_one_token_are_each_redacted() {
    let cases: [(&str, &str, &str); 9] = [
        ("see /alpha/one.md,/beta/two.md end", "see one.md,two.md end", "2"),
        ("paths /alpha/one.md:/beta/two.md end", "paths one.md:two.md end", "2"),
        ("set a/b=/alpha/one.md end", "set a/b=one.md end", "1"),
        (
            "open /alpha/one.md?next=http://example.com/x end",
            "open one.md?next=http://example.com/x end",
            "1",
        ),
        ("[report](/alpha/one.md) linked", "[report](one.md) linked", "1"),
        (
            "click [the plan](/alpha/one.md) now",
            "click [the plan](one.md) now",
            "1",
        ),
        ("run x='/alpha/one.md' now", "run x='one.md' now", "1"),
        ("file:=/alpha/one.md", "file:=one.md", "1"),
        ("a::/alpha/one.md", "a::one.md", "1"),
    ];
    for (given, expected, count) in cases {
        let tape = convert_lines(lines(&[&user_text(given)]));
        assert_eq!(first_text(&tape), text(expected), "given {}", given);
        assert_eq!(redacted(&tape), int(count), "given {}", given);
    }
}

#[test]
fn the_fixture_tape_is_unchanged() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/claude_session_synthetic.jsonl"
    );
    let tape = convert_file(std::path::Path::new(path)).unwrap();
    let expected = flighttape::json::parse(
        r#"{
        "tape_version": "1.0",
        "run": {
            "harness": "claude-code",
            "session_id": "synthetic-session-001",
            "source_format": "claude-code-jsonl",
            "skipped_lines": {"system": 1},
            "redacted_values": 0,
            "totals": {"input_tokens": 150, "output_tokens": 50,
                       "cache_creation_input_tokens": 0,
                       "cache_read_input_tokens": 0,
                       "tool_calls": 1, "messages": 3},
            "started_at": "2026-01-01T09:00:00.000Z"
        },
        "events": [
            {"seq": 0, "kind": "message", "sidechain": false,
             "timestamp": "2026-01-01T09:00:00.000Z", "role": "user",
             "text": "Synthetic prompt, fix the widget counter."},
            {"seq": 1, "kind": "message", "sidechain": false,
             "timestamp": "2026-01-01T09:00:05.000Z", "role": "assistant",
             "text": "Synthetic reply, reading the counter first."},
            {"seq": 2, "kind": "tool_call", "sidechain": false,
             "timestamp": "2026-01-01T09:00:05.000Z",
             "call_id": "toolu_synth_1", "tool": "Read",
             "input": {"path": "widget.py"}},
            {"seq": 3, "kind": "tool_result", "sidechain": false,
             "timestamp": "2026-01-01T09:00:06.000Z",
             "call_id": "toolu_synth_1", "is_error": false,
             "content": "counter = 0"},
            {"seq": 4, "kind": "message", "sidechain": true,
             "timestamp": "2026-01-01T09:01:00.000Z", "role": "assistant",
             "text": "Synthetic sidechain note, checking a hunch."}
        ],
        "outcome": {"status": "unknown", "basis": "unresolved-ending"}
    }"#,
    )
    .unwrap();
    assert_eq!(tape, expected);
}
