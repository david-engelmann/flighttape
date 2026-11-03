//! FT-44 to FT-48, tape screening, ported from the
//! Python suite. Every path in this file is
//! invented.

use std::path::{Path, PathBuf};

use flighttape::convert::convert_file;
use flighttape::dumps::dumps_indent;
use flighttape::identity::tape_digest;
use flighttape::json::parse;
use flighttape::screen::screen_tape;
use flighttape::validate::validate_tape;
use flighttape::value::{object, Text, Value};

const T0: &str = "2026-01-01T09:00:00.000Z";
const SECRET: &str = concat!("/home/synthetic", "-user/work/secret-plan.md");
const SECRET_KEY: &str = concat!("/home/synthetic", "-user/work/secret-key.md");

fn text(value: &str) -> Value {
    Value::Str(Text::from_str(value))
}

fn int(value: &str) -> Value {
    Value::Int(value.to_string())
}

fn tape(events: Vec<Value>) -> Value {
    object(vec![
        ("tape_version", text("1.0")),
        (
            "run",
            object(vec![
                ("harness", text("claude-code")),
                ("session_id", text("screen-me")),
                ("source_format", text("claude-code-jsonl")),
                ("started_at", text(T0)),
                ("skipped_lines", Value::Object(vec![])),
                ("redacted_values", int("0")),
                (
                    "totals",
                    object(vec![("messages", int("0")), ("tool_calls", int("0"))]),
                ),
            ]),
        ),
        ("events", Value::Array(events)),
        (
            "outcome",
            object(vec![
                ("status", text("completed")),
                ("basis", text("final-assistant-message")),
            ]),
        ),
    ])
}

fn event(seq: i64, kind: &str, extra: Vec<(&str, Value)>) -> Value {
    let mut pairs = vec![
        ("seq", Value::Int(seq.to_string())),
        ("kind", text(kind)),
        ("sidechain", Value::Bool(false)),
        ("timestamp", text(T0)),
    ];
    pairs.extend(extra);
    object(pairs)
}

fn dirty_tape() -> Value {
    tape(vec![
        event(
            0,
            "message",
            vec![
                ("role", text("user")),
                ("text", text(&format!("please read {} now", SECRET))),
            ],
        ),
        event(
            1,
            "tool_call",
            vec![
                ("tool", text("Read")),
                ("call_id", text("c1")),
                (
                    "input",
                    object(vec![
                        ("path", text(SECRET)),
                        (SECRET_KEY, text("v")),
                        (
                            "nested",
                            Value::Array(vec![object(vec![(
                                "deep",
                                text(&format!("see {}", SECRET)),
                            )])]),
                        ),
                    ]),
                ),
            ],
        ),
        event(
            2,
            "tool_result",
            vec![
                ("call_id", text("c1")),
                ("is_error", Value::Bool(false)),
                (
                    "content",
                    text(&format!("contents of {} and {}", SECRET, SECRET)),
                ),
            ],
        ),
        event(
            3,
            "message",
            vec![("role", text("assistant")), ("text", text("all done"))],
        ),
    ])
}

fn hits_of(report: &Value) -> Vec<(Value, Value, Value, Value)> {
    report
        .get("hits")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| {
            (
                hit.get("seq").unwrap().clone(),
                hit.get("field").unwrap().clone(),
                hit.get("in").unwrap().clone(),
                hit.get("paths").unwrap().clone(),
            )
        })
        .collect()
}

#[test]
fn each_carrier_reports_its_hits() {
    let dirty = dirty_tape();
    let report = screen_tape(&dirty).unwrap();
    assert_eq!(report.get("clean"), Some(&Value::Bool(false)));
    assert_eq!(report.get("paths_found"), Some(&int("6")));
    assert_eq!(
        hits_of(&report),
        vec![
            (int("0"), text("text"), text("value"), int("1")),
            (int("1"), text("input"), text("value"), int("1")),
            (int("1"), text("input"), text("key"), int("1")),
            (int("1"), text("input"), text("value"), int("1")),
            (int("2"), text("content"), text("value"), int("2")),
        ]
    );
    assert_eq!(
        report.get("tape"),
        Some(&text(&tape_digest(&dirty).unwrap()))
    );
}

#[test]
fn the_report_never_carries_the_path() {
    let report = screen_tape(&dirty_tape()).unwrap();
    let blob = dumps_indent(&report);
    assert!(!blob.contains(SECRET));
    assert!(!blob.contains("synthetic-user"));
}

#[test]
fn clean_text_and_non_paths_screen_clean() {
    let clean = tape(vec![
        event(
            0,
            "message",
            vec![
                ("role", text("user")),
                (
                    "text",
                    text(
                        "read src/main.py and https://example.com/a/b \
                         and the lone /etc note",
                    ),
                ),
            ],
        ),
        event(
            1,
            "tool_call",
            vec![
                ("tool", text("Read")),
                ("call_id", text("c1")),
                ("input", object(vec![("path", text("src/main.py"))])),
            ],
        ),
        event(
            2,
            "tool_result",
            vec![
                ("call_id", text("c1")),
                ("is_error", Value::Bool(false)),
                ("content", text("no paths here")),
            ],
        ),
    ]);
    let report = screen_tape(&clean).unwrap();
    assert_eq!(report.get("clean"), Some(&Value::Bool(true)));
    assert_eq!(report.get("paths_found"), Some(&int("0")));
    assert_eq!(report.get("hits"), Some(&Value::Array(vec![])));
    assert_eq!(report.get("strings_screened"), Some(&int("4")));
}

#[test]
fn the_converted_fixture_screens_clean() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/claude_session_synthetic.jsonl"
    );
    let converted = convert_file(std::path::Path::new(path)).unwrap();
    assert!(validate_tape(&converted).is_empty());
    let report = screen_tape(&converted).unwrap();
    assert_eq!(report.get("clean"), Some(&Value::Bool(true)));
    assert_eq!(report.get("paths_found"), Some(&int("0")));
}

#[test]
fn a_path_inside_a_container_carrier_is_a_hit() {
    let blocked = tape(vec![
        event(
            0,
            "message",
            vec![
                ("role", text("user")),
                (
                    "text",
                    Value::Array(vec![object(vec![
                        ("type", text("text")),
                        ("text", text(&format!("read {} now", SECRET))),
                    ])]),
                ),
            ],
        ),
        event(
            1,
            "message",
            vec![
                ("role", text("user")),
                ("text", object(vec![(SECRET, text("x"))])),
            ],
        ),
    ]);
    let report = screen_tape(&blocked).unwrap();
    assert_eq!(report.get("clean"), Some(&Value::Bool(false)));
    assert_eq!(report.get("paths_found"), Some(&int("2")));
    assert_eq!(
        hits_of(&report),
        vec![
            (int("0"), text("text"), text("value"), int("1")),
            (int("1"), text("text"), text("key"), int("1")),
        ]
    );
}

// ---- the CLI, exercised through the built binary ----

fn tmpdir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("flighttape-screen-{}-{}", std::process::id(), tag));
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

fn write_dirty(dir: &Path) -> String {
    let path = dir.join("dirty.tape.json");
    std::fs::write(&path, dumps_indent(&dirty_tape())).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn cli_reports_and_refuses() {
    let dir = tmpdir("basic");
    let path = write_dirty(&dir);
    assert!(validate_tape(&dirty_tape()).is_empty());
    let (code, out, _) = run_cli(&["screen".into(), path.clone()]);
    assert_eq!(code, 1);
    let report = parse(out.trim_end()).unwrap();
    assert_eq!(report.get("paths_found"), Some(&int("6")));
    let (code, _, err) = run_cli(&["screen".into(), path.clone(), "-o".into(), path.clone()]);
    assert_eq!(code, 1);
    assert!(err.contains("never overwrites"), "stderr {}", err);
    let bad = dir.join("bad.tape.json");
    std::fs::write(&bad, "{\"tape_version\": \"9.9\"}").unwrap();
    let (code, _, err) = run_cli(&["screen".into(), bad.to_string_lossy().into_owned()]);
    assert_eq!(code, 1);
    assert!(err.contains("not a valid tape"), "stderr {}", err);
    let out_path = dir.join("report.json");
    let (code, _, _) = run_cli(&[
        "screen".into(),
        path,
        "-o".into(),
        out_path.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 1);
    let written = parse(&std::fs::read_to_string(&out_path).unwrap()).unwrap();
    assert_eq!(written.get("paths_found"), Some(&int("6")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn deep_nesting_screens_without_a_traceback() {
    let dir = tmpdir("deep");
    let nested_tape = |depth: usize| -> Value {
        let mut deep = text(SECRET);
        for _ in 0..depth {
            deep = Value::Array(vec![deep]);
        }
        tape(vec![event(
            0,
            "tool_call",
            vec![
                ("tool", text("Read")),
                ("call_id", text("c1")),
                ("input", object(vec![("nest", deep)])),
            ],
        )])
    };
    let path = dir.join("deep.tape.json");
    std::fs::write(&path, dumps_indent(&nested_tape(800))).unwrap();
    let path_arg = path.to_string_lossy().into_owned();
    let (code, out, err) = run_cli(&["screen".into(), path_arg.clone()]);
    assert_eq!(code, 1);
    assert!(!err.contains("Traceback"));
    let report = parse(out.trim_end()).unwrap();
    assert_eq!(report.get("paths_found"), Some(&int("1")));
    // Past the canonical limit of 1000 containers the
    // digest refuses; the Python refuses earlier, at
    // its interpreter limit (FT-54 names the port's
    // deterministic counterpart).
    std::fs::write(&path, dumps_indent(&nested_tape(1000))).unwrap();
    let (code, _, err) = run_cli(&["screen".into(), path_arg]);
    assert_eq!(code, 1);
    assert!(!err.contains("Traceback"));
    assert!(err.contains("cannot be digested"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_clean_cli_arm_exits_zero() {
    let dir = tmpdir("clean");
    let clean = tape(vec![event(
        0,
        "message",
        vec![("role", text("user")), ("text", text("nothing to hide"))],
    )]);
    let path = dir.join("clean.tape.json");
    std::fs::write(&path, dumps_indent(&clean)).unwrap();
    let (code, out, _) = run_cli(&["screen".into(), path.to_string_lossy().into_owned()]);
    assert_eq!(code, 0);
    let report = parse(out.trim_end()).unwrap();
    assert_eq!(report.get("clean"), Some(&Value::Bool(true)));
    assert_eq!(report.get("paths_found"), Some(&int("0")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_tape_is_not_modified() {
    let dir = tmpdir("untouched");
    let path = write_dirty(&dir);
    let path_buf = dir.join("dirty.tape.json");
    let before = std::fs::read(&path_buf).unwrap();
    let out = dir.join("report.json");
    let (code, _, _) = run_cli(&[
        "screen".into(),
        path.clone(),
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 1);
    assert_eq!(std::fs::read(&path_buf).unwrap(), before);
    let (code, _, _) = run_cli(&["screen".into(), path]);
    assert_eq!(code, 1);
    assert_eq!(std::fs::read(&path_buf).unwrap(), before);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_write_failure_is_a_named_error() {
    let dir = tmpdir("writefail");
    let path = write_dirty(&dir);
    let missing = dir
        .join("no-such-dir")
        .join("report.json")
        .to_string_lossy()
        .into_owned();
    let (code, _, err) = run_cli(&["screen".into(), path, "-o".into(), missing]);
    assert_eq!(code, 1);
    assert!(err.contains("cannot write the report"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_hardlink_output_is_refused() {
    let dir = tmpdir("hardlink");
    let path = write_dirty(&dir);
    let path_buf = dir.join("dirty.tape.json");
    let before = std::fs::read(&path_buf).unwrap();
    let alias = dir.join("alias.json");
    std::fs::hard_link(&path_buf, &alias).unwrap();
    let (code, _, err) = run_cli(&[
        "screen".into(),
        path,
        "-o".into(),
        alias.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("never overwrites"), "stderr {}", err);
    assert_eq!(std::fs::read(&path_buf).unwrap(), before);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unresolvable_output_path_is_a_named_refusal() {
    let dir = tmpdir("loop");
    let path = write_dirty(&dir);
    std::os::unix::fs::symlink("loopy", dir.join("loopy")).unwrap();
    let looped = dir
        .join("loopy")
        .join("report.json")
        .to_string_lossy()
        .into_owned();
    let (code, _, err) = run_cli(&["screen".into(), path, "-o".into(), looped]);
    assert_eq!(code, 1);
    assert!(err.contains("cannot be resolved"), "stderr {}", err);
    assert!(!err.contains("Traceback"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_symlink_output_is_replaced_not_followed() {
    let dir = tmpdir("symlink");
    let path = write_dirty(&dir);
    let target = dir.join("target.json");
    std::fs::write(&target, "original").unwrap();
    let link = dir.join("link.json");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let (code, _, _) = run_cli(&[
        "screen".into(),
        path,
        "-o".into(),
        link.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 1);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "original");
    assert!(!link.symlink_metadata().unwrap().file_type().is_symlink());
    let written = parse(&std::fs::read_to_string(&link).unwrap()).unwrap();
    assert_eq!(written.get("paths_found"), Some(&int("6")));
    let _ = std::fs::remove_dir_all(&dir);
}
