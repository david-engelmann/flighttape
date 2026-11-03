//! The digest and verify CLI cases from the
//! identity suite, ported, run against the built
//! binary.

use std::path::{Path, PathBuf};

use flighttape::convert::convert_file;
use flighttape::dumps::dumps_indent;
use flighttape::value::Value;

const GOLDEN: &str = "cc9c4e01fca418dcc33114caf75fae1f6bf0d401f297b5d61f6776903fa955a3";
const NULL_DIGEST: &str = "74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b";

fn tmpdir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("flighttape-digest-{}-{}", std::process::id(), tag));
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

fn fixture_file(dir: &Path) -> String {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/claude_session_synthetic.jsonl"
    );
    let tape = convert_file(std::path::Path::new(source)).unwrap();
    let path = dir.join("tape.json");
    std::fs::write(&path, dumps_indent(&tape)).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn cli_digest_prints_exactly_the_digest() {
    let dir = tmpdir("digest");
    let path = fixture_file(&dir);
    let (code, out, _) = run_cli(&["digest".into(), path]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("{}\n", GOLDEN));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_verify_accepts_and_rejects() {
    let dir = tmpdir("verify");
    let path = fixture_file(&dir);
    let (code, out, _) = run_cli(&["verify".into(), path.clone(), GOLDEN.into()]);
    assert_eq!(code, 0);
    assert!(out.contains("digest matches"), "stdout {}", out);
    let padded_upper = format!("  {}", GOLDEN.to_uppercase());
    let (code, _, _) = run_cli(&["verify".into(), path.clone(), padded_upper]);
    assert_eq!(code, 0);
    let (code, _, err) = run_cli(&["verify".into(), path, "0".repeat(64)]);
    assert_eq!(code, 1);
    assert!(err.contains("digest mismatch"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_digest_of_an_undigestible_tape_is_a_named_error() {
    let dir = tmpdir("nan");
    let path = dir.join("tape.json");
    std::fs::write(&path, "{\"events\": [{\"input\": {\"x\": NaN}}]}").unwrap();
    let path_arg = path.to_string_lossy().into_owned();
    let (code, out, err) = run_cli(&["digest".into(), path_arg.clone()]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.contains("cannot be digested"), "stderr {}", err);
    let (code, _, err) = run_cli(&["validate".into(), path_arg]);
    assert_eq!(code, 1);
    assert!(err.contains("non-finite"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_digest_of_a_surrogate_tape_is_a_named_error() {
    let dir = tmpdir("surrogate");
    let path = dir.join("tape.json");
    std::fs::write(&path, "{\"text\": \"\\ud800\"}").unwrap();
    let (code, out, err) = run_cli(&["digest".into(), path.to_string_lossy().into_owned()]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.contains("cannot be digested"), "stderr {}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_null_tape_file_is_named_not_silent() {
    let dir = tmpdir("null");
    let path = dir.join("tape.json");
    std::fs::write(&path, "null\n").unwrap();
    let path_arg = path.to_string_lossy().into_owned();
    let (code, _, err) = run_cli(&["validate".into(), path_arg.clone()]);
    assert_eq!(code, 1);
    assert!(err.contains("not a JSON object"), "stderr {}", err);
    let (code, _, err) = run_cli(&["play".into(), path_arg.clone()]);
    assert_eq!(code, 1);
    assert!(err.contains("not a JSON object"), "stderr {}", err);
    let (code, out, _) = run_cli(&["digest".into(), path_arg.clone()]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("{}\n", NULL_DIGEST));
    let (code, _, _) = run_cli(&["verify".into(), path_arg, NULL_DIGEST.into()]);
    assert_eq!(code, 0);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = dumps_indent(&Value::Null);
}
