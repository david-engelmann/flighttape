"""flighttape command line.

Commands, convert, convert-dir, validate, play, digest, verify,
compare, rollup, slice.

Read failures, missing sources, entries in a converted directory
that are not files, and output write failures are named errors
with exit 1, never tracebacks (FT-21), and the converter and
validator behind these commands never raise on malformed input
(FT-18, FT-19).
"""
from __future__ import annotations

import argparse
import json
import pathlib
import sys

from .compare import compare_tapes
from .convert_claude import convert_file
from .identity import DigestError, tape_digest
from .play import render_tape
from .rollup import rollup_report
from .slice import SliceError, check_selection, slice_tape
from .validate import validate_tape


def _write_tape(tape, output):
    text = json.dumps(tape, indent=2)
    if output:
        pathlib.Path(output).write_text(text + "\n", encoding="utf-8")
    else:
        print(text)


def _convert_one(source, output):
    if not pathlib.Path(source).is_file():
        print(f"error, no such source file, {source}", file=sys.stderr)
        return 1
    try:
        tape = convert_file(source)
    except OSError:
        print(f"error, cannot read the source file, {source}",
              file=sys.stderr)
        return 1
    defects = validate_tape(tape)
    if defects:
        for defect in defects:
            print(f"defect, {defect}", file=sys.stderr)
        return 1
    try:
        _write_tape(tape, output)
    except OSError:
        print(f"error, cannot write the tape file, {output}",
              file=sys.stderr)
        return 1
    return 0


def _convert_dir(source_dir, output_dir):
    if not pathlib.Path(source_dir).is_dir():
        print(f"error, no such source directory, {source_dir}",
              file=sys.stderr)
        return 1
    out = pathlib.Path(output_dir)
    try:
        out.mkdir(parents=True, exist_ok=True)
    except OSError:
        print(f"error, cannot create the output directory, {output_dir}",
              file=sys.stderr)
        return 1
    summary = {"converted": 0, "failed": 0, "events": 0, "files": []}
    for source in sorted(pathlib.Path(source_dir).glob("*.jsonl")):
        if not source.is_file():
            print(f"error, not a file, skipped, {source}",
                  file=sys.stderr)
            summary["files"].append({"file": source.name, "events": 0,
                                      "defects": [],
                                      "error": "not a file"})
            summary["failed"] += 1
            continue
        try:
            tape = convert_file(source)
        except OSError:
            print(f"error, cannot read the source file, {source}",
                  file=sys.stderr)
            summary["files"].append({"file": source.name, "events": 0,
                                      "defects": [],
                                      "error": "cannot read"})
            summary["failed"] += 1
            continue
        defects = validate_tape(tape)
        entry = {"file": source.name, "events": len(tape["events"]),
                 "defects": defects}
        summary["files"].append(entry)
        if defects:
            summary["failed"] += 1
        else:
            summary["converted"] += 1
            summary["events"] += len(tape["events"])
            try:
                (out / (source.stem + ".tape.json")).write_text(
                    json.dumps(tape, indent=2) + "\n", encoding="utf-8")
            except OSError:
                print(f"error, cannot write the tape file for, {source}",
                      file=sys.stderr)
                return 1
    try:
        (out / "summary.json").write_text(
            json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    except OSError:
        print(f"error, cannot write the summary file in, {output_dir}",
              file=sys.stderr)
        return 1
    print(f"converted {summary['converted']}, "
          f"failed {summary['failed']}, events {summary['events']}")
    return 1 if summary["failed"] else 0


FAILED = object()


def _load(path):
    """Load a tape file without printing anything.

    Returns (tape, None) on success and (FAILED, reason) on
    failure, the reason being the loader's own phrase for the
    failure. A file holding the JSON value null loads as
    None, which is a tape to be named by the validator, not
    a load failure, so None cannot be the sentinel.
    """
    try:
        with open(path, encoding="utf-8") as handle:
            return json.load(handle), None
    except OSError:
        return FAILED, "cannot read the tape file"
    except UnicodeDecodeError:
        return FAILED, "the file is not valid UTF-8"
    except (RecursionError, ValueError):
        return FAILED, "the file is not valid JSON"


def _print_load_failure(path, reason):
    if reason == "cannot read the tape file":
        print(f"error, {reason}, {path}", file=sys.stderr)
    else:
        print(f"defect, {reason}, {path}", file=sys.stderr)


def _read_tape(path):
    """Load a tape file, printing the named error on failure (FT-21)."""
    tape, reason = _load(path)
    if tape is FAILED:
        _print_load_failure(path, reason)
    return tape


def _digest_of(tape):
    """The tape's digest, or None with the named error printed (FT-30)."""
    try:
        return tape_digest(tape)
    except DigestError as error:
        print(f"error, the tape cannot be digested, {error}",
              file=sys.stderr)
        return None


def _compare(first_path, second_path):
    """Compare two tape files and print the report (FT-31).

    Every failure on either side is named before the command
    refuses, load errors first, then validation defects, then
    a side that cannot be digested.
    """
    sides = []
    refused = False
    for path in (first_path, second_path):
        tape = _read_tape(path)
        if tape is FAILED:
            refused = True
        sides.append((path, tape))
    for path, tape in sides:
        if tape is FAILED:
            continue
        defects = validate_tape(tape)
        if defects:
            refused = True
            print(f"error, {path} is not a valid tape", file=sys.stderr)
            for defect in defects:
                print(f"defect, {defect}", file=sys.stderr)
            continue
        try:
            tape_digest(tape)
        except DigestError as error:
            refused = True
            print(f"error, {path} cannot be digested, {error}",
                  file=sys.stderr)
    if refused:
        return 1
    print(json.dumps(compare_tapes(sides[0][1], sides[1][1]), indent=2))
    return 0


def _rollup(source_dir):
    """Roll up every tape file in a directory (FT-35).

    Each .json entry is classified as a rolled tape or an
    exclusion with its reason named on stderr, and the report
    is printed either way; the exit code follows convert-dir,
    1 when any file was excluded.
    """
    root = pathlib.Path(source_dir)
    if not root.is_dir():
        print(f"error, no such source directory, {source_dir}",
              file=sys.stderr)
        return 1
    records = []
    for path in root.glob("*.json"):
        if not path.is_file():
            print(f"error, not a file, skipped, {path}", file=sys.stderr)
            records.append({"file": path.name, "reason": "not a file"})
            continue
        tape, reason = _load(path)
        if tape is FAILED:
            _print_load_failure(path, reason)
            records.append({"file": path.name, "reason": "unreadable",
                            "error": reason})
            continue
        defects = validate_tape(tape)
        if defects:
            print(f"error, {path} is not a valid tape", file=sys.stderr)
            for defect in defects:
                print(f"defect, {defect}", file=sys.stderr)
            records.append({"file": path.name, "reason": "invalid",
                            "defects": defects})
            continue
        try:
            digest = tape_digest(tape)
        except DigestError as error:
            print(f"error, {path} cannot be digested, {error}",
                  file=sys.stderr)
            records.append({"file": path.name,
                            "reason": "undigestible",
                            "error": str(error)})
            continue
        records.append({"file": path.name, "tape": tape,
                        "digest": digest})
    report = rollup_report(records, source_dir)
    print(json.dumps(report, indent=2))
    return 1 if report["excluded"] else 0


def _slice(path, output, from_seq, to_seq, kinds):
    """Cut a slice out of a tape and write it (FT-39 to FT-43).

    The selection is checked before the file is read, the
    source must load and validate, and the built slice must
    validate before anything is written.
    """
    try:
        check_selection(from_seq, to_seq, kinds)
    except SliceError as error:
        print(f"error, {error}", file=sys.stderr)
        return 1
    tape = _read_tape(path)
    if tape is FAILED:
        return 1
    defects = validate_tape(tape)
    if defects:
        print(f"error, {path} is not a valid tape", file=sys.stderr)
        for defect in defects:
            print(f"defect, {defect}", file=sys.stderr)
        return 1
    try:
        result = slice_tape(tape, from_seq, to_seq, kinds)
    except SliceError as error:
        print(f"error, {error}", file=sys.stderr)
        return 1
    except DigestError as error:
        print(f"error, {path} cannot be digested, {error}",
              file=sys.stderr)
        return 1
    defects = validate_tape(result)
    if defects:
        print("error, the slice is not a valid tape", file=sys.stderr)
        for defect in defects:
            print(f"defect, {defect}", file=sys.stderr)
        return 1
    try:
        _write_tape(result, output)
    except OSError:
        print(f"error, cannot write the tape file, {output}",
              file=sys.stderr)
        return 1
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(prog="flighttape")
    sub = parser.add_subparsers(dest="command", required=True)
    conv = sub.add_parser("convert", help="convert a session file to a tape")
    conv.add_argument("source")
    conv.add_argument("-o", "--output")
    batch = sub.add_parser(
        "convert-dir", help="convert every session file in a directory")
    batch.add_argument("source_dir")
    batch.add_argument("-o", "--output-dir", required=True)
    val = sub.add_parser("validate", help="validate a tape file")
    val.add_argument("tape")
    play = sub.add_parser("play", help="play a tape back as a timeline")
    play.add_argument("tape")
    dig = sub.add_parser("digest", help="print a tape's digest")
    dig.add_argument("tape")
    ver = sub.add_parser("verify", help="check a tape against a digest")
    ver.add_argument("tape")
    ver.add_argument("expected")
    comp = sub.add_parser("compare", help="compare two tapes")
    comp.add_argument("first")
    comp.add_argument("second")
    roll = sub.add_parser(
        "rollup", help="roll up every tape in a directory")
    roll.add_argument("source_dir")
    cut = sub.add_parser("slice", help="cut a slice out of a tape")
    cut.add_argument("tape")
    cut.add_argument("--from", dest="from_seq", type=int)
    cut.add_argument("--to", dest="to_seq", type=int)
    cut.add_argument("--kind", action="append", dest="kinds")
    cut.add_argument("-o", "--output")
    args = parser.parse_args(argv)
    if args.command == "convert":
        return _convert_one(args.source, args.output)
    if args.command == "convert-dir":
        return _convert_dir(args.source_dir, args.output_dir)
    if args.command == "compare":
        return _compare(args.first, args.second)
    if args.command == "rollup":
        return _rollup(args.source_dir)
    if args.command == "slice":
        return _slice(args.tape, args.output, args.from_seq,
                      args.to_seq, args.kinds)
    tape = _read_tape(args.tape)
    if tape is FAILED:
        return 1
    if args.command == "digest":
        digest = _digest_of(tape)
        if digest is None:
            return 1
        print(digest)
        return 0
    if args.command == "verify":
        digest = _digest_of(tape)
        if digest is None:
            return 1
        if args.expected.strip().lower() == digest:
            print(f"digest matches, {digest}")
            return 0
        print(f"defect, digest mismatch, the tape digests to {digest}",
              file=sys.stderr)
        return 1
    defects = validate_tape(tape)
    if defects:
        for defect in defects:
            print(f"defect, {defect}", file=sys.stderr)
        return 1
    if args.command == "play":
        print(render_tape(tape), end="")
        return 0
    print(f"valid tape, {len(tape['events'])} events")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
