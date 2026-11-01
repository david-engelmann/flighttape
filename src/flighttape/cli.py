"""flighttape command line, convert, convert-dir, validate, play.

Read failures and missing sources are named errors with exit 1,
never tracebacks (FT-21), and the converter and validator behind
these commands never raise on malformed input (FT-18, FT-19).
"""
from __future__ import annotations

import argparse
import json
import pathlib
import sys

from .convert_claude import convert_file
from .play import render_tape
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
    tape = convert_file(source)
    defects = validate_tape(tape)
    if defects:
        for defect in defects:
            print(f"defect, {defect}", file=sys.stderr)
        return 1
    _write_tape(tape, output)
    return 0


def _convert_dir(source_dir, output_dir):
    if not pathlib.Path(source_dir).is_dir():
        print(f"error, no such source directory, {source_dir}",
              file=sys.stderr)
        return 1
    out = pathlib.Path(output_dir)
    out.mkdir(parents=True, exist_ok=True)
    summary = {"converted": 0, "failed": 0, "events": 0, "files": []}
    for source in sorted(pathlib.Path(source_dir).glob("*.jsonl")):
        tape = convert_file(source)
        defects = validate_tape(tape)
        entry = {"file": source.name, "events": len(tape["events"]),
                 "defects": defects}
        summary["files"].append(entry)
        if defects:
            summary["failed"] += 1
        else:
            summary["converted"] += 1
            summary["events"] += len(tape["events"])
            (out / (source.stem + ".tape.json")).write_text(
                json.dumps(tape, indent=2) + "\n", encoding="utf-8")
    (out / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"converted {summary['converted']}, "
          f"failed {summary['failed']}, events {summary['events']}")
    return 1 if summary["failed"] else 0


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
    args = parser.parse_args(argv)
    if args.command == "convert":
        return _convert_one(args.source, args.output)
    if args.command == "convert-dir":
        return _convert_dir(args.source_dir, args.output_dir)
    try:
        with open(args.tape, encoding="utf-8") as handle:
            tape = json.load(handle)
    except OSError:
        print(f"error, cannot read the tape file, {args.tape}",
              file=sys.stderr)
        return 1
    except UnicodeDecodeError:
        print("defect, the file is not valid UTF-8", file=sys.stderr)
        return 1
    except (RecursionError, ValueError):
        print("defect, the file is not valid JSON", file=sys.stderr)
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
