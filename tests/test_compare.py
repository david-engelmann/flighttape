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


def message_event(seq, role, text, timestamp):
    return {"seq": seq, "kind": "message", "role": role,
            "sidechain": False, "text": text, "timestamp": timestamp}


def tape_a():
    return {
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "a",
                "totals": {"input_tokens": 10, "messages": 2},
                "redacted_values": 0, "skipped_lines": {"system": 1}},
        "events": [
            message_event(0, "user", "hi", "2026-01-01T09:00:00.000Z"),
            message_event(1, "assistant", "hello",
                          "2026-01-01T09:00:10.000Z"),
        ],
        "outcome": {"status": "completed",
                    "basis": "final-assistant-message"},
    }


def tape_b():
    tape = tape_a()
    tape["run"]["session_id"] = "b"
    tape["run"]["totals"] = {"input_tokens": 14, "messages": 2,
                             "tool_calls": 1}
    tape["run"]["redacted_values"] = 1
    tape["run"]["skipped_lines"] = {}
    tape["events"] = [
        message_event(0, "user", "hi", "2026-01-01T09:00:00.000Z"),
        message_event(1, "assistant", "hello!",
                      "2026-01-01T09:00:10.000Z"),
        message_event(2, "assistant", "more", "2026-01-01T09:00:25.000Z"),
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
    expected = {
        "digests": {"a": tape_digest(tape_a()),
                    "b": tape_digest(tape_b())},
        "outcome": {
            "a": {"status": "completed",
                  "basis": "final-assistant-message"},
            "b": {"status": "completed",
                  "basis": "final-assistant-message"},
            "changed": False,
        },
        "duration_seconds": {"a": 10.0, "b": 25.0, "delta": 15.0},
        "measures": {
            "input_tokens": {"a": 10, "b": 14, "delta": 4},
            "messages": {"a": 2, "b": 2, "delta": 0},
            "tool_calls": {"b": 1},
        },
        "redacted_values": {"a": 0, "b": 1, "delta": 1},
        "skipped_lines": {"system": {"a": 1}},
        "events": {
            "count": {"a": 2, "b": 3},
            "by_kind": {"message": {"a": 2, "b": 3}},
            "changed": [1],
            "added": [2],
            "removed": [],
        },
    }
    assert json.loads(printed) == expected
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


def test_null_totals_and_skipped_lines_compare_cleanly():
    first = tape_a()
    first["run"]["totals"] = None
    first["run"]["skipped_lines"] = None
    assert validate_tape(first) == []
    report = compare_tapes(first, tape_b())
    assert report["measures"] == {
        "input_tokens": {"b": 14},
        "messages": {"b": 2},
        "tool_calls": {"b": 1},
    }
    assert report["skipped_lines"] == {}


def test_a_non_numeric_redacted_value_is_carried_not_dropped():
    first, second = tape_a(), tape_b()
    first["run"]["redacted_values"] = "many"
    second["run"]["redacted_values"] = "few"
    assert validate_tape(first) == []
    report = compare_tapes(first, second)
    assert report["redacted_values"] == {"a": "many", "b": "few"}


def test_an_outcome_without_a_basis_reports_a_null_basis():
    second = tape_a()
    second["outcome"] = {"status": "completed"}
    assert validate_tape(second) == []
    report = compare_tapes(tape_a(), second)
    assert report["outcome"]["b"] == {"status": "completed",
                                      "basis": None}
    assert report["outcome"]["changed"] is True


def test_self_comparison_zeroes_redacted_and_skipped_deltas():
    tape = convert_lines(FIXTURE.read_text(encoding="utf-8").splitlines())
    report = compare_tapes(tape, tape)
    assert report["redacted_values"]["delta"] == 0
    for entry in report["skipped_lines"].values():
        assert entry["delta"] == 0


def test_cli_output_is_identical_across_serializations(
        tmp_path, capsys):
    first = tmp_path / "a.json"
    second = tmp_path / "b.json"
    first.write_text(json.dumps(tape_a()), encoding="utf-8")
    second.write_text(json.dumps(tape_b(), sort_keys=True, indent=7),
                      encoding="utf-8")
    assert main(["compare", str(first), str(second)]) == 0
    printed = capsys.readouterr().out
    first.write_text(json.dumps(tape_a(), sort_keys=True, indent=3),
                     encoding="utf-8")
    assert main(["compare", str(first), str(second)]) == 0
    assert capsys.readouterr().out == printed


def test_cli_compare_names_a_missing_file_and_an_invalid_tape(
        tmp_path, capsys):
    broken = tmp_path / "broken.json"
    tape = tape_b()
    tape["events"][1]["seq"] = 9
    broken.write_text(json.dumps(tape), encoding="utf-8")
    missing = tmp_path / "nope.json"
    assert main(["compare", str(broken), str(missing)]) == 1
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "cannot read" in captured.err
    assert str(broken) in captured.err
    assert "defect" in captured.err


def test_cli_compare_refuses_an_undigestible_tape(tmp_path, capsys):
    first = tmp_path / "a.json"
    second = tmp_path / "b.json"
    first.write_text(json.dumps(tape_a()), encoding="utf-8")
    text = json.dumps(tape_b())
    text = text.replace("hello!", "\\ud800")
    second.write_text(text, encoding="utf-8")
    loaded = json.loads(second.read_text(encoding="utf-8"))
    assert validate_tape(loaded) == []
    assert main(["compare", str(first), str(second)]) == 1
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "cannot be digested" in captured.err


def test_cli_compare_names_a_null_tape_instead_of_crashing(
        tmp_path, capsys):
    null_file = tmp_path / "null.json"
    null_file.write_text("null\n", encoding="utf-8")
    valid = tmp_path / "a.json"
    valid.write_text(json.dumps(tape_a()), encoding="utf-8")
    assert main(["compare", str(null_file), str(valid)]) == 1
    captured = capsys.readouterr()
    assert captured.out == ""
    assert str(null_file) in captured.err
    assert "not a JSON object" in captured.err
    assert main(["compare", str(valid), str(null_file)]) == 1
    capsys.readouterr()


def test_an_unrepresentable_delta_is_omitted_not_emitted():
    first, second = tape_a(), tape_b()
    first["run"]["redacted_values"] = 10 ** 400
    second["run"]["redacted_values"] = 0.5
    report = compare_tapes(first, second)
    assert report["redacted_values"] == {"a": 10 ** 400, "b": 0.5}
    first["run"]["redacted_values"] = -1e308
    second["run"]["redacted_values"] = 1e308
    report = compare_tapes(first, second)
    assert report["redacted_values"] == {"a": -1e308, "b": 1e308}
    first["run"]["redacted_values"] = 10 ** 400
    second["run"]["redacted_values"] = 1
    report = compare_tapes(first, second)
    assert report["redacted_values"]["delta"] == 1 - 10 ** 400


def test_cli_compare_names_which_file_is_not_json(tmp_path, capsys):
    bad = tmp_path / "bad.json"
    bad.write_bytes(b"\xff\xfe not utf-8")
    valid = tmp_path / "a.json"
    valid.write_text(json.dumps(tape_a()), encoding="utf-8")
    assert main(["compare", str(valid), str(bad)]) == 1
    captured = capsys.readouterr()
    assert "not valid UTF-8" in captured.err
    assert str(bad) in captured.err
