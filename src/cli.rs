//! flighttape command line.
//!
//! Commands: convert, convert-dir, validate, play,
//! digest, verify, compare, rollup, slice, screen.
//!
//! Read failures, missing sources, entries in a
//! converted directory that are not files, and output
//! write failures are named errors with exit 1, never
//! tracebacks (FT-21), and the converter and validator
//! behind these commands never raise on malformed
//! input (FT-18, FT-19).

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::compare::compare_tapes;
use crate::convert::convert_file;
use crate::dumps::dumps_indent;
use crate::helptext::{sub_help, sub_usage, TOP_HELP, TOP_USAGE};
use crate::identity::tape_digest;
use crate::json::{parse, ParseError};
use crate::play::render_tape;
use crate::rollup::rollup_report;
use crate::screen::screen_tape;
use crate::slice::{
    check_selection, parse_bound, slice_tape, SliceErrorOrDigest,
};
use crate::validate::validate_tape;
use crate::value::{object, Text, Value};

fn out_line(text: &str) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(text.as_bytes());
    let _ = handle.write_all(b"\n");
    let _ = handle.flush();
}

fn out_raw(text: &str) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(text.as_bytes());
    let _ = handle.flush();
}

fn err_line(text: &str) {
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    let _ = handle.write_all(text.as_bytes());
    let _ = handle.write_all(b"\n");
    let _ = handle.flush();
}

/// The loader's outcome: a tape, or FAILED with the
/// loader's own phrase for the failure. A file holding
/// the JSON value null loads as a null tape, which the
/// validator names, not a load failure.
enum Load {
    Tape(Value),
    Failed(&'static str),
}

fn load(path: &Path) -> Load {
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(_) => return Load::Failed("cannot read the tape file"),
    };
    let text = match String::from_utf8(data) {
        Ok(text) => text,
        Err(_) => return Load::Failed("the file is not valid UTF-8"),
    };
    match parse(&text) {
        Ok(tape) => Load::Tape(tape),
        Err(ParseError::TooLarge) => {
            Load::Failed("the file holds a number too large to read")
        }
        Err(ParseError::Invalid) => Load::Failed("the file is not valid JSON"),
    }
}

fn print_load_failure(path: &str, reason: &str) {
    if reason == "cannot read the tape file" {
        err_line(&format!("error, {}, {}", reason, path));
    } else {
        err_line(&format!("defect, {}, {}", reason, path));
    }
}

fn read_tape(path: &str) -> Load {
    match load(Path::new(path)) {
        Load::Failed(reason) => {
            print_load_failure(path, reason);
            Load::Failed(reason)
        }
        tape => tape,
    }
}

fn digest_of(tape: &Value) -> Option<String> {
    match tape_digest(tape) {
        Ok(digest) => Some(digest),
        Err(error) => {
            err_line(&format!(
                "error, the tape cannot be digested, {}",
                error.0
            ));
            None
        }
    }
}

fn report_defects(path: &str, tape: &Value) -> Vec<String> {
    let defects = validate_tape(tape);
    if !defects.is_empty() {
        err_line(&format!("error, {} is not a valid tape", path));
        for defect in &defects {
            err_line(&format!("defect, {}", defect));
        }
    }
    defects
}

fn digest_or_refuse(path: &str, tape: &Value) -> Option<String> {
    match tape_digest(tape) {
        Ok(digest) => Some(digest),
        Err(error) => {
            err_line(&format!("error, {} cannot be digested, {}", path, error.0));
            None
        }
    }
}

fn write_tape(tape: &Value, output: Option<&str>) -> Result<(), ()> {
    let text = dumps_indent(tape);
    match output {
        Some(output) => {
            std::fs::write(output, format!("{}\n", text)).map_err(|_| ())
        }
        None => {
            out_line(&text);
            Ok(())
        }
    }
}

fn convert_one(source: &str, output: Option<&str>) -> i32 {
    if !Path::new(source).is_file() {
        err_line(&format!("error, no such source file, {}", source));
        return 1;
    }
    let tape = match convert_file(Path::new(source)) {
        Ok(tape) => tape,
        Err(_) => {
            err_line(&format!(
                "error, cannot read the source file, {}",
                source
            ));
            return 1;
        }
    };
    let defects = validate_tape(&tape);
    if !defects.is_empty() {
        for defect in &defects {
            err_line(&format!("defect, {}", defect));
        }
        return 1;
    }
    if write_tape(&tape, output).is_err() {
        err_line(&format!(
            "error, cannot write the tape file, {}",
            output.unwrap_or("None")
        ));
        return 1;
    }
    0
}

/// Join a directory and a name the way pathlib prints
/// the joined path.
fn path_display(dir: &str, name: &str) -> String {
    let trimmed = dir.trim_end_matches('/');
    if trimmed.is_empty() {
        if dir.starts_with('/') {
            return format!("/{}", name);
        }
        return name.to_string();
    }
    if trimmed == "." {
        return name.to_string();
    }
    let trimmed = trimmed.strip_prefix("./").unwrap_or(trimmed);
    format!("{}/{}", trimmed, name)
}

fn dir_entries(
    dir: &str,
    suffix: &str,
    sorted: bool,
) -> Vec<(String, PathBuf, bool)> {
    // (file name, full path, is_file) for entries whose
    // name ends with the suffix. Sorted by name when the
    // caller sorts (convert-dir does, as the Python's
    // sorted glob does); rollup passes false and keeps
    // directory read order, the Python glob's own order,
    // which its per-file stderr lines follow.
    let mut entries: Vec<(String, PathBuf, bool)> = Vec::new();
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(suffix) {
                let is_file = entry
                    .file_type()
                    .map(|kind| kind.is_file())
                    .unwrap_or(false);
                entries.push((name, entry.path(), is_file));
            }
        }
    }
    if sorted {
        entries.sort_by(|a, b| a.0.cmp(&b.0));
    }
    entries
}

fn convert_dir(source_dir: &str, output_dir: &str) -> i32 {
    if !Path::new(source_dir).is_dir() {
        err_line(&format!(
            "error, no such source directory, {}",
            source_dir
        ));
        return 1;
    }
    let out = Path::new(output_dir);
    if std::fs::create_dir_all(out).is_err() {
        err_line(&format!(
            "error, cannot create the output directory, {}",
            output_dir
        ));
        return 1;
    }
    let mut converted: u64 = 0;
    let mut failed: u64 = 0;
    let mut total_events: u64 = 0;
    let mut files: Vec<Value> = Vec::new();
    for (name, path, is_file) in dir_entries(source_dir, ".jsonl", true) {
        let display = path_display(source_dir, &name);
        if !is_file {
            err_line(&format!("error, not a file, skipped, {}", display));
            files.push(object(vec![
                ("file", Value::Str(Text::from_str(&name))),
                ("events", Value::Int("0".to_string())),
                ("defects", Value::Array(Vec::new())),
                ("error", Value::Str(Text::from_str("not a file"))),
            ]));
            failed += 1;
            continue;
        }
        let tape = match convert_file(&path) {
            Ok(tape) => tape,
            Err(_) => {
                err_line(&format!(
                    "error, cannot read the source file, {}",
                    display
                ));
                files.push(object(vec![
                    ("file", Value::Str(Text::from_str(&name))),
                    ("events", Value::Int("0".to_string())),
                    ("defects", Value::Array(Vec::new())),
                    ("error", Value::Str(Text::from_str("cannot read"))),
                ]));
                failed += 1;
                continue;
            }
        };
        let defects = validate_tape(&tape);
        let events_count = tape
            .get("events")
            .and_then(|events| events.as_array())
            .map(|items| items.len() as u64)
            .unwrap_or(0);
        files.push(object(vec![
            ("file", Value::Str(Text::from_str(&name))),
            ("events", Value::Int(events_count.to_string())),
            (
                "defects",
                Value::Array(
                    defects
                        .iter()
                        .map(|defect| Value::Str(Text::from_str(defect)))
                        .collect(),
                ),
            ),
        ]));
        if !defects.is_empty() {
            failed += 1;
        } else {
            converted += 1;
            total_events += events_count;
            let stem = name.strip_suffix(".jsonl").unwrap_or(&name);
            let target = out.join(format!("{}.tape.json", stem));
            let text = format!("{}\n", dumps_indent(&tape));
            if std::fs::write(&target, text).is_err() {
                err_line(&format!(
                    "error, cannot write the tape file for, {}",
                    display
                ));
                return 1;
            }
        }
    }
    let summary = object(vec![
        ("converted", Value::Int(converted.to_string())),
        ("failed", Value::Int(failed.to_string())),
        ("events", Value::Int(total_events.to_string())),
        ("files", Value::Array(files)),
    ]);
    let summary_text = format!("{}\n", dumps_indent(&summary));
    if std::fs::write(out.join("summary.json"), summary_text).is_err() {
        err_line(&format!(
            "error, cannot write the summary file in, {}",
            output_dir
        ));
        return 1;
    }
    out_line(&format!(
        "converted {}, failed {}, events {}",
        converted, failed, total_events
    ));
    if failed > 0 {
        1
    } else {
        0
    }
}

fn compare_command(first_path: &str, second_path: &str) -> i32 {
    let mut sides: Vec<(&str, Load)> = Vec::new();
    let mut refused = false;
    for path in [first_path, second_path] {
        let tape = read_tape(path);
        if matches!(tape, Load::Failed(_)) {
            refused = true;
        }
        sides.push((path, tape));
    }
    let mut tapes: Vec<Value> = Vec::new();
    for (path, tape) in &sides {
        match tape {
            Load::Failed(_) => {}
            Load::Tape(tape) => {
                if !report_defects(path, tape).is_empty() {
                    refused = true;
                    continue;
                }
                if digest_or_refuse(path, tape).is_none() {
                    refused = true;
                    continue;
                }
                tapes.push(tape.clone());
            }
        }
    }
    if refused {
        return 1;
    }
    let report =
        compare_tapes(&tapes[0], &tapes[1]).expect("digest-checked sides");
    out_line(&dumps_indent(&report));
    0
}

fn rollup_command(source_dir: &str) -> i32 {
    let root = Path::new(source_dir);
    if !root.is_dir() {
        err_line(&format!(
            "error, no such source directory, {}",
            source_dir
        ));
        return 1;
    }
    let mut records: Vec<Value> = Vec::new();
    for (name, path, is_file) in dir_entries(source_dir, ".json", false) {
        let display = path_display(source_dir, &name);
        if !is_file {
            err_line(&format!("error, not a file, skipped, {}", display));
            records.push(object(vec![
                ("file", Value::Str(Text::from_str(&name))),
                ("reason", Value::Str(Text::from_str("not a file"))),
            ]));
            continue;
        }
        match load(&path) {
            Load::Failed(reason) => {
                print_load_failure(&display, reason);
                records.push(object(vec![
                    ("file", Value::Str(Text::from_str(&name))),
                    ("reason", Value::Str(Text::from_str("unreadable"))),
                    ("error", Value::Str(Text::from_str(reason))),
                ]));
            }
            Load::Tape(tape) => {
                let defects = report_defects(&display, &tape);
                if !defects.is_empty() {
                    records.push(object(vec![
                        ("file", Value::Str(Text::from_str(&name))),
                        ("reason", Value::Str(Text::from_str("invalid"))),
                        (
                            "defects",
                            Value::Array(
                                defects
                                    .iter()
                                    .map(|defect| {
                                        Value::Str(Text::from_str(defect))
                                    })
                                    .collect(),
                            ),
                        ),
                    ]));
                    continue;
                }
                match tape_digest(&tape) {
                    Err(error) => {
                        err_line(&format!(
                            "error, {} cannot be digested, {}",
                            display, error.0
                        ));
                        records.push(object(vec![
                            ("file", Value::Str(Text::from_str(&name))),
                            (
                                "reason",
                                Value::Str(Text::from_str("undigestible")),
                            ),
                            ("error", Value::Str(Text::from_str(&error.0))),
                        ]));
                    }
                    Ok(digest) => {
                        records.push(object(vec![
                            ("file", Value::Str(Text::from_str(&name))),
                            ("tape", tape),
                            ("digest", Value::Str(Text::from_str(&digest))),
                        ]));
                    }
                }
            }
        }
    }
    let report = rollup_report(records, source_dir);
    out_line(&dumps_indent(&report));
    let excluded = report
        .get("excluded")
        .and_then(|excluded| excluded.as_array())
        .map(|items| items.len())
        .unwrap_or(0);
    if excluded > 0 {
        1
    } else {
        0
    }
}

fn slice_command(
    path: &str,
    output: Option<&str>,
    from_text: Option<&str>,
    to_text: Option<&str>,
    kinds: Option<Vec<String>>,
) -> i32 {
    let (from_seq, to_seq) = match (
        parse_bound(from_text, "--from"),
        parse_bound(to_text, "--to"),
    ) {
        (Ok(from_seq), Ok(to_seq)) => (from_seq, to_seq),
        (Err(error), _) | (_, Err(error)) => {
            err_line(&format!("error, {}", error.0));
            return 1;
        }
    };
    if let Err(error) = check_selection(from_seq, to_seq, kinds.as_deref()) {
        err_line(&format!("error, {}", error.0));
        return 1;
    }
    let tape = match read_tape(path) {
        Load::Failed(_) => return 1,
        Load::Tape(tape) => tape,
    };
    if !report_defects(path, &tape).is_empty() {
        return 1;
    }
    let result = match slice_tape(&tape, from_seq, to_seq, kinds) {
        Ok(result) => result,
        Err(SliceErrorOrDigest::Slice(error)) => {
            err_line(&format!("error, {}", error.0));
            return 1;
        }
        Err(SliceErrorOrDigest::Digest(error)) => {
            err_line(&format!(
                "error, {} cannot be digested, {}",
                path, error.0
            ));
            return 1;
        }
    };
    let defects = validate_tape(&result);
    if !defects.is_empty() {
        err_line("error, the slice is not a valid tape");
        for defect in &defects {
            err_line(&format!("defect, {}", defect));
        }
        return 1;
    }
    let text = format!("{}\n", dumps_indent(&result));
    match output {
        Some(output) => {
            if std::fs::write(output, text).is_err() {
                err_line(&format!(
                    "error, cannot write the tape file, {}",
                    output
                ));
                return 1;
            }
        }
        None => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            if handle.write_all(text.as_bytes()).is_err()
                || handle.flush().is_err()
            {
                err_line("error, cannot write the tape to stdout");
                return 1;
            }
        }
    }
    0
}

/// Resolve a path the way pathlib's resolve does for
/// the screen guard: absolute, with symlinks in the
/// existing prefix resolved and the rest lexical.
/// Whether two existing paths name the same file,
/// the os.path.samefile test: device and inode.
#[cfg(unix)]
fn same_file(first: &Path, second: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::metadata(first), std::fs::metadata(second)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn same_file(_first: &Path, _second: &Path) -> bool {
    false
}

/// Resolve a path the way the guard needs, or None
/// when resolution itself fails, as it does on a
/// symlink loop (the Python's resolve raises there
/// and the command names the path unresolvable).
fn resolve_lexical(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Some(canonical);
    }
    if let Err(error) = std::fs::canonicalize(path) {
        if error.raw_os_error() == Some(libc_eloop()) {
            return None;
        }
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
    };
    // Canonicalize the longest existing ancestor and
    // re-attach the remainder, normalizing . and ...
    let mut existing = absolute.clone();
    let mut remainder: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if let Ok(canonical) = std::fs::canonicalize(&existing) {
            let mut resolved = canonical;
            for part in remainder.iter().rev() {
                resolved.push(part);
            }
            return Some(normalize_lexical(&resolved));
        }
        if std::fs::symlink_metadata(&existing).is_ok() {
            if let Err(error) = std::fs::canonicalize(&existing) {
                if error.raw_os_error() == Some(libc_eloop()) {
                    return None;
                }
            }
        }
        match existing.file_name() {
            Some(name) => {
                remainder.push(name.to_os_string());
                existing = existing
                    .parent()
                    .map(|parent| parent.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("/"));
            }
            None => return Some(normalize_lexical(&absolute)),
        }
    }
}

/// The ELOOP errno without a libc dependency.
fn libc_eloop() -> i32 {
    40
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            part => out.push(part.as_os_str()),
        }
    }
    out
}

fn screen_command(path: &str, output: Option<&str>) -> i32 {
    let tape_path = Path::new(path);
    if let Some(output) = output {
        let out_path = Path::new(output);
        let (resolved_out, resolved_tape) =
            (resolve_lexical(out_path), resolve_lexical(tape_path));
        let (resolved_out, resolved_tape) =
            match (resolved_out, resolved_tape) {
                (Some(out), Some(tape)) => (out, tape),
                _ => {
                    err_line(&format!(
                        "error, the output path cannot be resolved, {}",
                        output
                    ));
                    return 1;
                }
            };
        let same = resolved_out == resolved_tape
            || (tape_path.exists()
                && out_path.exists()
                && same_file(out_path, tape_path));
        if same {
            err_line("error, a screen report never overwrites the tape");
            return 1;
        }
    }
    let tape = match read_tape(path) {
        Load::Failed(_) => return 1,
        Load::Tape(tape) => tape,
    };
    if !report_defects(path, &tape).is_empty() {
        return 1;
    }
    let report = match screen_tape(&tape) {
        Ok(report) => report,
        Err(error) => {
            err_line(&format!(
                "error, {} cannot be digested, {}",
                path, error.0
            ));
            return 1;
        }
    };
    let clean = report.get("clean") == Some(&Value::Bool(true));
    let text = format!("{}\n", dumps_indent(&report));
    match output {
        Some(output) => {
            // The report lands atomically (FT-47): a
            // sibling temp file renamed over the output.
            let out_path = Path::new(output);
            let dir = out_path.parent().unwrap_or_else(|| Path::new("."));
            let name = out_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let temp = dir.join(format!(
                "{}.{}",
                name,
                std::process::id()
            ));
            let written = std::fs::write(&temp, &text)
                .and_then(|_| std::fs::rename(&temp, out_path));
            if written.is_err() {
                let _ = std::fs::remove_file(&temp);
                err_line(&format!(
                    "error, cannot write the report, {}",
                    output
                ));
                return 1;
            }
        }
        None => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            if handle.write_all(text.as_bytes()).is_err()
                || handle.flush().is_err()
            {
                err_line("error, cannot write the report to stdout");
                return 1;
            }
        }
    }
    if clean {
        0
    } else {
        1
    }
}

fn print_top_help() {
    out_raw(TOP_HELP);
}

fn print_sub_help(command: &str) {
    out_raw(sub_help(command));
}

/// A top-level usage error: the top usage block and a
/// bare `flighttape: error:` line. Argparse reports
/// unrecognized arguments and a missing or unknown
/// command this way.
fn top_error(message: &str) -> i32 {
    err_line(TOP_USAGE);
    err_line(&format!("flighttape: error: {}", message));
    2
}

/// A subcommand usage error: the subcommand's usage
/// block and a `flighttape <command>: error:` line.
/// Argparse reports missing arguments and an option
/// left without its value this way.
fn sub_error(command: &str, message: &str) -> i32 {
    err_line(sub_usage(command));
    err_line(&format!("flighttape {}: error: {}", command, message));
    2
}

/// Whether a token reads as a negative number, the
/// one dash-led shape argparse treats as a value or a
/// positional rather than an option: `-1`, `-1.5` and
/// `-.5` read as numbers, `-1.` does not.
fn is_negative_number(token: &str) -> bool {
    let rest = match token.strip_prefix('-') {
        Some(rest) if !rest.is_empty() && !rest.starts_with('-') => rest,
        _ => return false,
    };
    let digits = |s: &str| {
        !s.is_empty() && s.bytes().all(|byte| byte.is_ascii_digit())
    };
    match rest.split_once('.') {
        None => digits(rest),
        Some((whole, frac)) => (whole.is_empty() || digits(whole)) && digits(frac),
    }
}

/// Whether a token would be read as an option string
/// where a value is expected: dash-led, longer than
/// the bare dash, and not a negative number.
fn looks_like_option(token: &str) -> bool {
    token.starts_with('-') && token.len() > 1 && !is_negative_number(token)
}

/// A value-taking option of a subcommand.
#[derive(Clone, Copy)]
enum Opt {
    Output,
    From,
    To,
    Kind,
}

/// The long options of a subcommand, help first. A
/// None value marks the help option, which takes no
/// value.
fn long_options(command: &str) -> Vec<(&'static str, Option<Opt>)> {
    let mut options = vec![("help", None)];
    match command {
        "convert" | "slice" | "screen" => {
            options.push(("output", Some(Opt::Output)))
        }
        "convert-dir" => options.push(("output-dir", Some(Opt::Output))),
        _ => {}
    }
    if command == "slice" {
        options.push(("from", Some(Opt::From)));
        options.push(("to", Some(Opt::To)));
        options.push(("kind", Some(Opt::Kind)));
    }
    options
}

/// How argparse names an option in its errors.
fn opt_display(command: &str, opt: &Opt) -> &'static str {
    match opt {
        Opt::Output => {
            if command == "convert-dir" {
                "-o/--output-dir"
            } else {
                "-o/--output"
            }
        }
        Opt::From => "--from",
        Opt::To => "--to",
        Opt::Kind => "--kind",
    }
}

struct Parsed {
    positional: Vec<String>,
    output: Option<String>,
    from_seq: Option<String>,
    to_seq: Option<String>,
    kinds: Vec<String>,
}

fn assign(parsed: &mut Parsed, opt: Opt, value: String) {
    match opt {
        Opt::Output => parsed.output = Some(value),
        Opt::From => parsed.from_seq = Some(value),
        Opt::To => parsed.to_seq = Some(value),
        Opt::Kind => parsed.kinds.push(value),
    }
}

/// Parse one subcommand's arguments the way argparse
/// does for the shapes these commands declare: long
/// options answer to any unambiguous prefix, a value
/// is never taken from an option-shaped token, tokens
/// past the declared positional count and unknown
/// options collect as unrecognized, a missing required
/// argument is reported by the subcommand first, and
/// the unrecognized are reported by the top parser.
/// On a usage problem, prints the argparse-shaped
/// error and returns its exit code instead.
fn parse_args(command: &str, args: &[String]) -> Result<Parsed, i32> {
    let expected = match command {
        "verify" | "compare" => 2,
        _ => 1,
    };
    let wants_output =
        matches!(command, "convert" | "convert-dir" | "slice" | "screen");
    let options = long_options(command);
    let mut parsed = Parsed {
        positional: Vec::new(),
        output: None,
        from_seq: None,
        to_seq: None,
        kinds: Vec::new(),
    };
    let mut unrecognized: Vec<String> = Vec::new();
    let mut after_separator = false;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].clone();
        if !after_separator {
            if arg == "--" {
                after_separator = true;
                index += 1;
                continue;
            }
            if arg.starts_with("--") && arg.len() > 2 {
                let body = &arg[2..];
                let (name, inline) = match body.find('=') {
                    Some(eq) => {
                        (&body[..eq], Some(body[eq + 1..].to_string()))
                    }
                    None => (body, None),
                };
                let resolved = match options.iter().find(|(known, _)| *known == name) {
                    Some((_, opt)) => Some(*opt),
                    None => {
                        let mut hits = options
                            .iter()
                            .filter(|(known, _)| known.starts_with(name));
                        match (hits.next(), hits.next()) {
                            (Some((_, opt)), None) => Some(*opt),
                            _ => None,
                        }
                    }
                };
                match resolved {
                    None => unrecognized.push(arg.clone()),
                    Some(None) => {
                        if let Some(value) = inline {
                            return Err(sub_error(
                                command,
                                &format!(
                                    "argument -h/--help: ignored explicit argument '{}'",
                                    value
                                ),
                            ));
                        }
                        print_sub_help(command);
                        return Err(0);
                    }
                    Some(Some(opt)) => {
                        let value = match inline {
                            Some(value) => value,
                            None => match args.get(index + 1) {
                                Some(next) if !looks_like_option(next) => {
                                    index += 1;
                                    next.clone()
                                }
                                _ => {
                                    return Err(sub_error(
                                        command,
                                        &format!(
                                            "argument {}: expected one argument",
                                            opt_display(command, &opt)
                                        ),
                                    ))
                                }
                            },
                        };
                        assign(&mut parsed, opt, value);
                    }
                }
                index += 1;
                continue;
            }
            if arg.starts_with('-') && arg.len() > 1 && !is_negative_number(&arg) {
                let second = arg[1..].chars().next().unwrap_or('-');
                if second == 'h' {
                    print_sub_help(command);
                    return Err(0);
                }
                if second == 'o' && wants_output {
                    let rest: String = arg[1..].chars().skip(1).collect();
                    let value = if !rest.is_empty() {
                        match rest.strip_prefix('=') {
                            Some(stripped) => stripped.to_string(),
                            None => rest,
                        }
                    } else {
                        match args.get(index + 1) {
                            Some(next) if !looks_like_option(next) => {
                                index += 1;
                                next.clone()
                            }
                            _ => {
                                return Err(sub_error(
                                    command,
                                    &format!(
                                        "argument {}: expected one argument",
                                        opt_display(command, &Opt::Output)
                                    ),
                                ))
                            }
                        }
                    };
                    parsed.output = Some(value);
                    index += 1;
                    continue;
                }
                unrecognized.push(arg.clone());
                index += 1;
                continue;
            }
        }
        if parsed.positional.len() < expected {
            parsed.positional.push(arg);
        } else {
            unrecognized.push(arg);
        }
        index += 1;
    }
    let names: &[&str] = match command {
        "convert" => &["source"],
        "convert-dir" => &["source_dir"],
        "verify" => &["tape", "expected"],
        "compare" => &["first", "second"],
        "rollup" => &["source_dir"],
        _ => &["tape"],
    };
    let mut missing: Vec<String> = names[parsed.positional.len()..]
        .iter()
        .map(|name| name.to_string())
        .collect();
    if command == "convert-dir" && parsed.output.is_none() {
        missing.push("-o/--output-dir".to_string());
    }
    if !missing.is_empty() {
        return Err(sub_error(
            command,
            &format!(
                "the following arguments are required: {}",
                missing.join(", ")
            ),
        ));
    }
    if !unrecognized.is_empty() {
        return Err(top_error(&format!(
            "unrecognized arguments: {}",
            unrecognized.join(" ")
        )));
    }
    Ok(parsed)
}

/// Whether a token before the command word asks for
/// the top-level help: `-h`, a short cluster opening
/// with h, or a long token naming help exactly or by
/// unambiguous prefix, the top parser's only option.
fn is_top_help(arg: &str) -> bool {
    if let Some(body) = arg.strip_prefix("--") {
        let name = body.split('=').next().unwrap_or("");
        return !name.is_empty() && "help".starts_with(name);
    }
    if arg.starts_with('-') && arg.len() > 1 && !is_negative_number(arg) {
        return arg[1..].starts_with('h');
    }
    false
}

/// The command line entry: returns the exit code.
pub fn run_cli() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    const COMMANDS: [&str; 10] = [
        "convert", "convert-dir", "validate", "play", "digest", "verify",
        "compare", "rollup", "slice", "screen",
    ];
    // The top parser reads option-shaped tokens until
    // the command word: help answers where it stands,
    // an unknown option is set aside and reported only
    // once the subcommand has parsed clean, and a word
    // that is no command is refused where it stands,
    // argparse's own order throughout.
    let mut before: Vec<String> = Vec::new();
    let mut after_separator = false;
    let mut index = 0;
    let command: &str = loop {
        if index >= args.len() {
            return top_error("the following arguments are required: command");
        }
        let arg = &args[index];
        if !after_separator && arg == "--" {
            after_separator = true;
            index += 1;
            continue;
        }
        if !after_separator && looks_like_option(arg) {
            if is_top_help(arg) {
                if let Some(eq) = arg.find('=') {
                    if arg.starts_with("--") {
                        return top_error(&format!(
                            "argument -h/--help: ignored explicit argument '{}'",
                            &arg[eq + 1..]
                        ));
                    }
                }
                print_top_help();
                return 0;
            }
            before.push(arg.clone());
            index += 1;
            continue;
        }
        break arg.as_str();
    };
    if !COMMANDS.contains(&command) {
        return top_error(&format!(
            "argument command: invalid choice: '{}' (choose from {})",
            command,
            COMMANDS
                .iter()
                .map(|name| format!("'{}'", name))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let parsed = match parse_args(command, &args[index + 1..]) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    if !before.is_empty() {
        return top_error(&format!(
            "unrecognized arguments: {}",
            before.join(" ")
        ));
    }
    match command {
        "convert" => convert_one(&parsed.positional[0], parsed.output.as_deref()),
        "convert-dir" => convert_dir(
            &parsed.positional[0],
            parsed.output.as_deref().unwrap_or(""),
        ),
        "compare" => compare_command(&parsed.positional[0], &parsed.positional[1]),
        "rollup" => rollup_command(&parsed.positional[0]),
        "slice" => slice_command(
            &parsed.positional[0],
            parsed.output.as_deref(),
            parsed.from_seq.as_deref(),
            parsed.to_seq.as_deref(),
            if parsed.kinds.is_empty() {
                None
            } else {
                Some(parsed.kinds.clone())
            },
        ),
        "screen" => screen_command(&parsed.positional[0], parsed.output.as_deref()),
        _ => {
            // validate, play, digest, verify share the
            // read; digest and verify never validate.
            let path = &parsed.positional[0];
            let tape = match read_tape(path) {
                Load::Failed(_) => return 1,
                Load::Tape(tape) => tape,
            };
            if command == "digest" || command == "verify" {
                let digest = match digest_of(&tape) {
                    Some(digest) => digest,
                    None => return 1,
                };
                if command == "digest" {
                    out_line(&digest);
                    return 0;
                }
                let expected = parsed.positional[1]
                    .trim_matches(|ch: char| {
                        matches!(ch,
                            '\u{09}'..='\u{0D}' | '\u{20}' | '\u{85}' | '\u{A0}'
                            | '\u{1680}' | '\u{2000}'..='\u{200A}' | '\u{2028}'
                            | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}')
                    })
                    .to_lowercase();
                if expected == digest {
                    out_line(&format!("digest matches, {}", digest));
                    return 0;
                }
                err_line(&format!(
                    "defect, digest mismatch, the tape digests to {}",
                    digest
                ));
                return 1;
            }
            let defects = validate_tape(&tape);
            if !defects.is_empty() {
                for defect in &defects {
                    err_line(&format!("defect, {}", defect));
                }
                return 1;
            }
            if command == "play" {
                out_raw(&render_tape(&tape));
                return 0;
            }
            let events = tape
                .get("events")
                .and_then(|events| events.as_array())
                .map(|items| items.len())
                .unwrap_or(0);
            out_line(&format!("valid tape, {} events", events));
            0
        }
    }
}
