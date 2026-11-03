//! CLI tests, ported from the Python suite, run
//! against the built binary.

use std::path::PathBuf;

use flighttape::dumps::dumps_indent;
use flighttape::json::parse;
use flighttape::play::render_tape;
use flighttape::value::Value;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/claude_session_synthetic.jsonl"
);

fn tmpdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "flighttape-cli-{}-{}",
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

#[test]
fn validate_valid_tape_exits_zero() {
    let dir = tmpdir("valid");
    let out = dir.join("tape.json");
    let (code, _, _) = run_cli(&[
        "convert".into(),
        FIXTURE.into(),
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 0);
    let (code, stdout, _) =
        run_cli(&["validate".into(), out.to_string_lossy().into_owned()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("valid tape"), "stdout {}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_malformed_tape_exits_one() {
    let dir = tmpdir("malformed");
    let bad = dir.join("bad.json");
    std::fs::write(&bad, "{\"tape_version\": \"9.9\"}").unwrap();
    let (code, _, err) =
        run_cli(&["validate".into(), bad.to_string_lossy().into_owned()]);
    assert_eq!(code, 1);
    assert!(err.contains("defect"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn convert_empty_session_exits_one() {
    let dir = tmpdir("empty");
    let empty = dir.join("empty.jsonl");
    std::fs::write(&empty, "").unwrap();
    let (code, _, err) =
        run_cli(&["convert".into(), empty.to_string_lossy().into_owned()]);
    assert_eq!(code, 1);
    assert!(err.contains("defect"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn play_renders_a_valid_tape() {
    let dir = tmpdir("play");
    let out = dir.join("tape.json");
    let (code, _, _) = run_cli(&[
        "convert".into(),
        FIXTURE.into(),
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 0);
    let (code, stdout, _) =
        run_cli(&["play".into(), out.to_string_lossy().into_owned()]);
    assert_eq!(code, 0);
    let tape = parse(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(stdout, render_tape(&tape));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn play_and_validate_reject_a_non_json_file() {
    let dir = tmpdir("nonjson");
    let bad = dir.join("bad.json");
    std::fs::write(&bad, "{not json").unwrap();
    let bad_arg = bad.to_string_lossy().into_owned();
    let (code, _, _) = run_cli(&["play".into(), bad_arg.clone()]);
    assert_eq!(code, 1);
    let (code, out, err) = run_cli(&["validate".into(), bad_arg]);
    assert_eq!(code, 1);
    assert!(err.contains("not valid JSON"), "stderr {}", err);
    assert_eq!(out, "");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn play_malformed_tape_exits_one_with_no_rendering() {
    let dir = tmpdir("playbad");
    let bad = dir.join("bad.json");
    std::fs::write(&bad, "{\"tape_version\": \"9.9\"}").unwrap();
    let (code, out, err) =
        run_cli(&["play".into(), bad.to_string_lossy().into_owned()]);
    assert_eq!(code, 1);
    assert!(err.contains("defect"), "stderr {}", err);
    assert!(!out.contains("tape "), "stdout {}", out);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn convert_dir_writes_a_tape_and_a_summary() {
    let dir = tmpdir("batch");
    let src = dir.join("sessions");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("a.jsonl"),
        std::fs::read_to_string(FIXTURE).unwrap(),
    )
    .unwrap();
    let out = dir.join("tapes");
    let (code, _, _) = run_cli(&[
        "convert-dir".into(),
        src.to_string_lossy().into_owned(),
        "-o".into(),
        out.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 0);
    let summary =
        parse(&std::fs::read_to_string(out.join("summary.json")).unwrap())
            .unwrap();
    assert_eq!(
        summary.get("converted"),
        Some(&Value::Int("1".to_string()))
    );
    assert_eq!(
        summary.get("failed"),
        Some(&Value::Int("0".to_string()))
    );
    assert_eq!(
        summary.get("events"),
        Some(&Value::Int("5".to_string()))
    );
    assert!(out.join("a.tape.json").exists());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = dumps_indent(&Value::Null);
}
