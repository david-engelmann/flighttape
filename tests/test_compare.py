"""Tests for tape comparison, FT-31 to FT-34.

Every test can fail for a real reason. The side by side report
is pinned field by field on a hand-built pair, so a measure
dropped, a delta mis-signed, or an event misjudged as changed
fails loudly. The refusal tests fail if an invalid tape ever
produces a report.
"""
import json
import pathlib

from flighttape.cli import main
from flighttape.compare import compare_tapes
from flighttape.convert_claude import convert_lines
from flighttape.identity import tape_digest
from flighttape.validate import validate_tape

FIXTURE = (pathlib.Path(__file__).parent / "fixtures"
           / "claude_session_synthetic.jsonl")


def message(seq, role, text, timestamp):
    return {"seq": seq, "kind": "message", "role": role,
            "sidechain": False, "text": text, "timestamp": timestamp}


def tape_a():
    return {
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "a",
                "totals": {"input_tokens": 10, "messages": 2},
                "redacted_values": 0, "skipped_lines": {"system": 1}},
        "events": [
            message(0, "user", "hi", "2026-01-01T09:00:00.000Z"),
            message(1, "assistant", "hello", "2026-01-01T09:00:10.000Z"),
        ],
        "outcome": {"status": "completed",
                    "basis": "final-assistant-message"},
    }


def tape_b():
    tape = tape_a()
    tape["run"] = {"harness": "claude-code", "session_id": "b",
                   "totals": {"input_tokens": 14, "messages": 2,
                              "tool_calls": 1},
                   "redacted_values": 1, "skipped_lines": {}}
    tape["events"] = [
        message(0, "user", "hi", "2026-01-01T09:00:00.000Z"),
        message(1, "assistant", "hello!", "2026-01-01T09:00:10.000Z"),
        message(2, "assistant", "more", "2026-01-01T09:00:25.000Z"),
    ]
    return tape


def test_the_hand_built_pair_is_valid():
    assert validate_tape(tape_a()) == []
    assert validate_tape(tape_b()) == []


def test_the_report_is_pinned_field_by_field():
    first, second = tape_a(), tape_b()
    report = compare_tapes(first, second)
    assert report["digests"] == {"a": tape_digest(first),
                                 "b": tape_digest(second)}
    assert report["outcome"] == {
        "a": {"status": "completed", "basis": "final-assistant-message"},
        "b": {"status": "completed", "basis": "final-assistant-message"},
        "changed": False,
    }
    assert report["duration_seconds"] == {"a": 10.0, "b": 25.0,
                                          "delta": 15.0}
    assert report["measures"] == {
        "input_tokens": {"a": 10, "b": 14, "delta": 4},
        "messages": {"a": 2, "b": 2, "delta": 0},
        "tool_calls": {"b": 1},
    }
    assert report["redacted_values"] == {"a": 0, "b": 1, "delta": 1}
    assert report["skipped_lines"] == {"system": {"a": 1}}
    assert report["events"] == {
        "count": {"a": 2, "b": 3},
        "by_kind": {"message": {"a": 2, "b": 3}},
        "changed": [1],
        "added": [2],
        "removed": [],
    }


def test_an_outcome_change_is_reported():
    second = tape_a()
    second["outcome"] = {"status": "unknown",
                         "basis": "unresolved-ending"}
    assert validate_tape(second) == []
    report = compare_tapes(tape_a(), second)
    assert report["outcome"]["changed"] is True
    assert report["outcome"]["b"]["status"] == "unknown"


def test_a_tape_compared_with_itself_reports_no_differences():
    tape = convert_lines(FIXTURE.read_text(encoding="utf-8").splitlines())
    report = compare_tapes(tape, tape)
    assert report["digests"]["a"] == report["digests"]["b"]
    assert report["outcome"]["changed"] is False
    assert report["events"]["changed"] == []
    assert report["events"]["added"] == []
    assert report["events"]["removed"] == []
    for entry in report["measures"].values():
        assert entry["delta"] == 0
    assert report["duration_seconds"]["delta"] == 0.0


def test_one_changed_line_changes_exactly_its_event():
    lines = FIXTURE.read_text(encoding="utf-8").splitlines()
    variant = list(lines)
    changed_line = json.loads(variant[0])
    changed_line["message"]["content"][0]["text"] = "A different prompt"
    variant[0] = json.dumps(changed_line)
    report = compare_tapes(convert_lines(lines), convert_lines(variant))
    assert report["events"]["changed"] == [0]
    assert report["events"]["added"] == []
    assert report["events"]["removed"] == []


def test_duration_is_null_below_two_timestamps():
    single = tape_a()
    single["events"] = single["events"][:1]
    single["outcome"] = {"status": "unknown",
                         "basis": "unresolved-ending"}
    assert validate_tape(single) == []
    report = compare_tapes(single, tape_a())
    assert report["duration_seconds"]["a"] is None
    assert "delta" not in report["duration_seconds"]


def test_cli_compare_prints_the_report(tmp_path, capsys):
    first = tmp_path / "a.json"
    second = tmp_path / "b.json"
    first.write_text(json.dumps(tape_a()), encoding="utf-8")
    second.write_text(json.dumps(tape_b()), encoding="utf-8")
    assert main(["compare", str(first), str(second)]) == 0
    printed = capsys.readouterr().out
    assert json.loads(printed) == compare_tapes(tape_a(), tape_b())
    assert main(["compare", str(first), str(second)]) == 0
    assert capsys.readouterr().out == printed


def test_cli_compare_refuses_an_invalid_tape(tmp_path, capsys):
    first = tmp_path / "a.json"
    second = tmp_path / "b.json"
    broken = tape_b()
    broken["events"][1]["seq"] = 9
    first.write_text(json.dumps(tape_a()), encoding="utf-8")
    second.write_text(json.dumps(broken), encoding="utf-8")
    assert main(["compare", str(first), str(second)]) == 1
    captured = capsys.readouterr()
    assert captured.out == ""
    assert str(second) in captured.err
    assert "defect" in captured.err


def test_cli_compare_names_a_missing_file(tmp_path, capsys):
    present = tmp_path / "a.json"
    present.write_text(json.dumps(tape_a()), encoding="utf-8")
    missing = tmp_path / "nope.json"
    assert main(["compare", str(present), str(missing)]) == 1
    assert "cannot read" in capsys.readouterr().err
