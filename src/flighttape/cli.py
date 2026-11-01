"""flighttape command line, convert and validate."""
from __future__ import annotations

import argparse
import json
import sys

from .convert_claude import convert_file
from .validate import validate_tape


def main(argv=None):
    parser = argparse.ArgumentParser(prog="flighttape")
    sub = parser.add_subparsers(dest="command", required=True)
    conv = sub.add_parser("convert", help="convert a session file to a tape")
    conv.add_argument("source")
    conv.add_argument("-o", "--output")
    val = sub.add_parser("validate", help="validate a tape file")
    val.add_argument("tape")
    args = parser.parse_args(argv)
    if args.command == "convert":
        tape = convert_file(args.source)
        defects = validate_tape(tape)
        if defects:
            for defect in defects:
                print(f"defect, {defect}", file=sys.stderr)
            return 1
        text = json.dumps(tape, indent=2)
        if args.output:
            with open(args.output, "w", encoding="utf-8") as handle:
                handle.write(text + "\n")
        else:
            print(text)
        return 0
    with open(args.tape, encoding="utf-8") as handle:
        tape = json.load(handle)
    defects = validate_tape(tape)
    if defects:
        for defect in defects:
            print(f"defect, {defect}")
        return 1
    print(f"valid tape, {len(tape['events'])} events")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
