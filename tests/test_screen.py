"""FT-44 to FT-48, tape screening.

Every path in this file is invented. The round's claim
is that the redactor's own detector, read in the other
direction, finds every absolute path in exactly the
strings FT-22 names as text, reports locations and
counts only, and agrees with the converter: what
conversion produces screens clean.
"""
import json
import pathlib

from flighttape.cli import main
from flighttape.convert_claude import convert_file
from flighttape.identity import tape_digest
from flighttape.screen import screen_tape
from flighttape.validate import validate_tape

FIXTURE = (pathlib.Path(__file__).resolve().parent / "fixtures"
           / "claude_session_synthetic.jsonl")
T0 = "2026-01-01T09:00:00.000Z"
SECRET = "/home/synthetic-user/project/secret.py"


def event(seq, kind, **extra):
    base = {"seq": seq, "kind": kind, "sidechain": False,
            "timestamp": T0}
    base.update(extra)
    return base


def tape(events):
    return {"tape_version": "1.0",
            "run": {"harness": "claude-code",
                    "session_id": "screen-me",
                    "source_format": "claude-code-jsonl",
                    "started_at": T0, "skipped_lines": {},
                    "redacted_values": 0,
                    "totals": {"messages": 0, "tool_calls": 0}},
            "events": events,
            "outcome": {"status": "completed",
                        "basis": "final-assistant-message"}}


def dirty_tape():
    return tape([
        event(0, "message", role="user",
              text=f"please read {SECRET} now"),
        event(1, "tool_call", tool="Read", call_id="c1",
              input={"path": SECRET,
                     SECRET: "a key that is a path",
                     "nested": [{"deep": f"see {SECRET}"}]}),
        event(2, "tool_result", call_id="c1", is_error=False,
              content=f"contents of {SECRET} and {SECRET}"),
        event(3, "message", role="assistant", text="all done"),
    ])


def test_each_carrier_reports_its_hits():
    report = screen_tape(dirty_tape())
    assert report["clean"] is False
    assert report["paths_found"] == 6
    hits = [(h["seq"], h["field"], h["in"], h["paths"])
            for h in report["hits"]]
    assert hits == [(0, "text", "value", 1),
                    (1, "input", "value", 1),
                    (1, "input", "key", 1),
                    (1, "input", "value", 1),
                    (2, "content", "value", 2)]
    assert report["tape"] == tape_digest(dirty_tape())


def test_the_report_never_carries_the_path():
    report = screen_tape(dirty_tape())
    assert SECRET not in json.dumps(report)
    assert "synthetic-user" not in json.dumps(report)


def test_clean_text_and_non_paths_screen_clean():
    clean = tape([
        event(0, "message", role="user",
              text="read src/main.py and https://example.com/a/b "
                   "and the lone /etc note"),
        event(1, "tool_call", tool="Read", call_id="c1",
              input={"path": "src/main.py"}),
        event(2, "tool_result", call_id="c1", is_error=False,
              content="no paths here"),
    ])
    report = screen_tape(clean)
    assert report["clean"] is True
    assert report["paths_found"] == 0
    assert report["hits"] == []
    assert report["strings_screened"] == 4


def test_the_converted_fixture_screens_clean():
    converted = convert_file(str(FIXTURE))
    assert validate_tape(converted) == []
    report = screen_tape(converted)
    assert report["clean"] is True
    assert report["paths_found"] == 0


def test_the_cli_reports_and_refuses(tmp_path, capsys):
    path = tmp_path / "dirty.tape.json"
    path.write_text(json.dumps(dirty_tape()), encoding="utf-8")
    assert validate_tape(dirty_tape()) == []
    assert main(["screen", str(path)]) == 1
    report = json.loads(capsys.readouterr().out)
    assert report["paths_found"] == 6
    assert main(["screen", str(path), "-o", str(path)]) == 1
    assert "never overwrites" in capsys.readouterr().err
    bad = tmp_path / "bad.tape.json"
    bad.write_text(json.dumps({"tape_version": "9.9"}),
                   encoding="utf-8")
    assert main(["screen", str(bad)]) == 1
    assert "not a valid tape" in capsys.readouterr().err
    out = tmp_path / "report.json"
    assert main(["screen", str(path), "-o", str(out)]) == 1
    assert json.loads(out.read_text(encoding="utf-8"))[
        "paths_found"] == 6
