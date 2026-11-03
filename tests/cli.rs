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
    let dir = std::env::temp_dir().join(format!("flighttape-cli-{}-{}", std::process::id(), tag));
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
    let (code, stdout, _) = run_cli(&["validate".into(), out.to_string_lossy().into_owned()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("valid tape"), "stdout {}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_malformed_tape_exits_one() {
    let dir = tmpdir("malformed");
    let bad = dir.join("bad.json");
    std::fs::write(&bad, "{\"tape_version\": \"9.9\"}").unwrap();
    let (code, _, err) = run_cli(&["validate".into(), bad.to_string_lossy().into_owned()]);
    assert_eq!(code, 1);
    assert!(err.contains("defect"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn convert_empty_session_exits_one() {
    let dir = tmpdir("empty");
    let empty = dir.join("empty.jsonl");
    std::fs::write(&empty, "").unwrap();
    let (code, _, err) = run_cli(&["convert".into(), empty.to_string_lossy().into_owned()]);
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
    let (code, stdout, _) = run_cli(&["play".into(), out.to_string_lossy().into_owned()]);
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
    let (code, out, err) = run_cli(&["play".into(), bad.to_string_lossy().into_owned()]);
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
    let summary = parse(&std::fs::read_to_string(out.join("summary.json")).unwrap()).unwrap();
    assert_eq!(summary.get("converted"), Some(&Value::Int("1".to_string())));
    assert_eq!(summary.get("failed"), Some(&Value::Int("0".to_string())));
    assert_eq!(summary.get("events"), Some(&Value::Int("5".to_string())));
    assert!(out.join("a.tape.json").exists());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = dumps_indent(&Value::Null);
}

// The argparse surface, pinned as the differential
// harness found it: help text, the subcommand and
// top-level error split, abbreviations, and the
// required-before-unrecognized order. These strings
// are written out here on purpose, so an edit to the
// help text module cannot pass by agreeing with
// itself.

const TOP_USAGE_ERR: &str = concat!(
    "usage: flighttape [-h]\n",
    "                  {convert,convert-dir,validate,play,digest,verify,compare,rollup,slice,screen}\n",
    "                  ...\n",
);

#[test]
fn top_help_opens_with_the_wrapped_usage() {
    let (code, stdout, _) = run_cli(&["--help".into()]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with(TOP_USAGE_ERR), "stdout {}", stdout);
    assert!(stdout.contains("\npositional arguments:\n"));
    assert!(stdout.contains("screen              screen a tape for absolute paths"));
}

#[test]
fn validate_help_is_byte_exact() {
    let (code, stdout, _) = run_cli(&["validate".into(), "--help".into()]);
    assert_eq!(code, 0);
    assert_eq!(
        stdout,
        "usage: flighttape validate [-h] tape\n\npositional arguments:\n  tape\n\noptions:\n  -h, --help  show this help message and exit\n"
    );
}

#[test]
fn no_arguments_is_a_top_level_required_error() {
    let (code, _, stderr) = run_cli(&[]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        format!(
            "{}flighttape: error: the following arguments are required: command\n",
            TOP_USAGE_ERR
        )
    );
}

#[test]
fn missing_tape_is_a_subcommand_required_error() {
    let (code, _, stderr) = run_cli(&["validate".into()]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "usage: flighttape validate [-h] tape\nflighttape validate: error: the following arguments are required: tape\n"
    );
}

#[test]
fn extra_positional_is_a_top_level_unrecognized_error() {
    let (code, _, stderr) = run_cli(&["validate".into(), "a.json".into(), "extra".into()]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        format!(
            "{}flighttape: error: unrecognized arguments: extra\n",
            TOP_USAGE_ERR
        )
    );
}

#[test]
fn required_is_reported_before_unrecognized() {
    let (code, _, stderr) = run_cli(&["validate".into(), "--bogus".into()]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "usage: flighttape validate [-h] tape\nflighttape validate: error: the following arguments are required: tape\n"
    );
}

#[test]
fn unrecognized_before_the_command_waits_for_the_parse() {
    let (code, _, stderr) = run_cli(&["--bogus".into(), "validate".into(), "missing.json".into()]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        format!(
            "{}flighttape: error: unrecognized arguments: --bogus\n",
            TOP_USAGE_ERR
        )
    );
}

#[test]
fn a_long_option_answers_to_an_unambiguous_prefix() {
    let (code, _, stderr) = run_cli(&[
        "slice".into(),
        "missing.json".into(),
        "--f".into(),
        "1".into(),
    ]);
    assert_eq!(code, 1);
    assert_eq!(stderr, "error, cannot read the tape file, missing.json\n");
}

#[test]
fn the_glued_equals_form_sets_the_output() {
    let dir = tmpdir("glued-equals");
    let out = dir.join("tape.json");
    let arg = format!("-o={}", out.to_string_lossy());
    let (code, _, _) = run_cli(&["convert".into(), FIXTURE.into(), arg]);
    assert_eq!(code, 0);
    assert!(out.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn convert_dir_names_both_missing_arguments() {
    let (code, _, stderr) = run_cli(&["convert-dir".into()]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "usage: flighttape convert-dir [-h] -o OUTPUT_DIR source_dir\nflighttape convert-dir: error: the following arguments are required: source_dir, -o/--output-dir\n"
    );
}

#[test]
fn slice_usage_wraps_the_way_argparse_wraps_it() {
    let (code, _, stderr) = run_cli(&["slice".into()]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "usage: flighttape slice [-h] [--from FROM_SEQ] [--to TO_SEQ] [--kind KINDS]\n                        [-o OUTPUT]\n                        tape\nflighttape slice: error: the following arguments are required: tape\n"
    );
}

#[test]
fn rollup_output_does_not_depend_on_creation_order() {
    // Ported from the Python suite: the same two tapes
    // filed in opposite creation orders roll up to the
    // same report, byte for byte.
    fn tape(session: &str) -> String {
        format!(
            "{{\"tape_version\": \"1.0\", \"run\": {{\"harness\": \"claude-code\", \"session_id\": \"{}\", \"totals\": {{}}, \"redacted_values\": 0, \"skipped_lines\": {{}}}}, \"events\": [{{\"seq\": 0, \"kind\": \"message\", \"role\": \"user\", \"sidechain\": false, \"text\": \"hi\", \"timestamp\": \"2026-01-01T09:00:00.000Z\"}}], \"outcome\": {{\"status\": \"completed\", \"basis\": \"final-assistant-message\"}}}}",
            session
        )
    }
    let first_dir = tmpdir("rollup-order-a");
    std::fs::write(first_dir.join("b.json"), tape("b")).unwrap();
    std::fs::write(first_dir.join("a.json"), tape("a")).unwrap();
    let (code, first_out, _) =
        run_cli(&["rollup".into(), first_dir.to_string_lossy().into_owned()]);
    assert_eq!(code, 0);
    let second_dir = tmpdir("rollup-order-b");
    std::fs::write(second_dir.join("a.json"), tape("a")).unwrap();
    std::fs::write(second_dir.join("b.json"), tape("b")).unwrap();
    let (code, second_out, _) =
        run_cli(&["rollup".into(), second_dir.to_string_lossy().into_owned()]);
    assert_eq!(code, 0);
    // The reports name their directory, so compare with
    // the directory paths normalized away.
    let normalize = |text: &str, dir: &std::path::Path| {
        text.replace(&dir.to_string_lossy().into_owned(), "DIR")
    };
    assert_eq!(
        normalize(&first_out, &first_dir),
        normalize(&second_out, &second_dir)
    );
    let _ = std::fs::remove_dir_all(&first_dir);
    let _ = std::fs::remove_dir_all(&second_dir);
}

#[test]
fn the_ho_cluster_without_a_value_is_the_output_error() {
    // Red team m3: argparse settles the cluster's
    // trailing output option before the help action,
    // so a missing value is the error, not the help.
    let (code, stdout, stderr) = run_cli(&["convert".into(), FIXTURE.into(), "-ho".into()]);
    assert_eq!(code, 2);
    assert_eq!(stdout, "");
    assert_eq!(
        stderr,
        "usage: flighttape convert [-h] [-o OUTPUT] source\nflighttape convert: error: argument -o/--output: expected one argument\n"
    );
}

#[test]
fn the_ho_cluster_with_a_value_prints_the_help() {
    let dir = tmpdir("hocluster");
    let out = dir.join("tape.json");
    let (code, stdout, stderr) = run_cli(&[
        "convert".into(),
        FIXTURE.into(),
        "-ho".into(),
        out.to_string_lossy().into_owned(),
    ]);
    assert_eq!(code, 0);
    assert!(
        stdout.starts_with("usage: flighttape convert [-h] [-o OUTPUT] source\n"),
        "stdout {}",
        stdout
    );
    assert_eq!(stderr, "");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_leading_separator_is_refused_as_the_command_word() {
    // Red team m4: before the command word, the
    // separator stands where the command stands, and
    // the choice check refuses it.
    let (code, _, stderr) = run_cli(&["--".into(), "validate".into(), "tape.json".into()]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("argument command: invalid choice: '--'"),
        "stderr {}",
        stderr
    );
}
