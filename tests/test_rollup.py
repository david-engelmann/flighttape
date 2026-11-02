"""FT-35 to FT-38, the directory rollup."""
import json

from flighttape.cli import main
from flighttape.identity import tape_digest
from flighttape.rollup import rollup_report
from flighttape.validate import validate_tape


def message_event(seq, role, text, timestamp):
    return {"seq": seq, "kind": "message", "role": role,
            "sidechain": False, "text": text, "timestamp": timestamp}


def tape_one():
    return {
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "one",
                "totals": {"input_tokens": 10, "messages": 2},
                "redacted_values": 2, "skipped_lines": {"system": 1}},
        "events": [
            message_event(0, "user", "hi", "2026-01-01T09:00:00.000Z"),
            message_event(1, "assistant", "hello",
                          "2026-01-01T09:00:10.000Z"),
        ],
        "outcome": {"status": "completed",
                    "basis": "final-assistant-message"},
    }


def tape_two():
    tape = tape_one()
    tape["run"]["session_id"] = "two"
    tape["run"]["totals"] = {"input_tokens": 5, "tool_calls": 3}
    tape["run"]["redacted_values"] = "many"
    tape["run"]["skipped_lines"] = {"system": 2, "summary": 1}
    tape["events"] = [
        message_event(0, "user", "hey", "2026-01-01T10:00:00.000Z"),
    ]
    tape["outcome"] = {"status": "failed",
                       "basis": "final-error-result"}
    return tape


def records(*named_tapes):
    return [{"file": name, "tape": tape, "digest": tape_digest(tape)}
            for name, tape in named_tapes]


def test_the_hand_built_pair_is_valid():
    assert validate_tape(tape_one()) == []
    assert validate_tape(tape_two()) == []


def test_the_report_is_pinned_field_by_field():
    report = rollup_report(
        records(("b.json", tape_two()), ("a.json", tape_one())), "dir")
    assert report["directory"] == "dir"
    assert [entry["file"] for entry in report["tapes"]] == [
        "a.json", "b.json"]
    first = report["tapes"][0]
    assert first["digest"] == tape_digest(tape_one())
    assert first["outcome"] == {"status": "completed",
                                "basis": "final-assistant-message"}
    assert first["duration_seconds"] == 10.0
    assert first["events"] == 2
    assert first["totals"] == {"input_tokens": 10, "messages": 2}
    second = report["tapes"][1]
    assert second["duration_seconds"] is None
    assert report["excluded"] == []
    assert report["counts"] == {"tapes": 2, "excluded": 0, "events": 3}
    assert report["outcomes"] == {"completed": 1, "failed": 1}
    assert report["totals"] == {"input_tokens": 15, "messages": 2,
                                "tool_calls": 3}
    assert report["redacted_values"] == {"total": 2, "non_numeric": 1}
    assert report["skipped_lines"] == {"summary": 1, "system": 3}
    assert report["duration_seconds"] == {"total": 10.0, "known": 1,
                                          "unknown": 1}


def test_identical_files_are_two_tapes_not_one():
    report = rollup_report(
        records(("a.json", tape_one()), ("copy.json", tape_one())), "d")
    assert report["counts"]["tapes"] == 2
    assert report["totals"]["input_tokens"] == 20
    assert [entry["digest"] for entry in report["tapes"]] == [
        tape_digest(tape_one())] * 2


def test_record_order_does_not_change_the_report():
    forward = rollup_report(
        records(("a.json", tape_one()), ("b.json", tape_two())), "d")
    backward = rollup_report(
        records(("b.json", tape_two()), ("a.json", tape_one())), "d")
    assert json.dumps(forward, indent=2) == json.dumps(backward,
                                                       indent=2)


def test_a_null_totals_tape_rolls_up_with_empty_totals():
    tape = tape_one()
    tape["run"]["totals"] = None
    tape["run"]["skipped_lines"] = None
    assert validate_tape(tape) == []
    report = rollup_report(records(("a.json", tape)), "d")
    assert report["tapes"][0]["totals"] == {}
    assert report["totals"] == {}
    assert report["skipped_lines"] == {}


def test_an_outcome_without_a_basis_reports_a_null_basis():
    tape = tape_one()
    tape["outcome"] = {"status": "completed"}
    assert validate_tape(tape) == []
    report = rollup_report(records(("a.json", tape)), "d")
    assert report["tapes"][0]["outcome"] == {"status": "completed",
                                             "basis": None}


def test_an_unrepresentable_redacted_total_is_null():
    first, second = tape_one(), tape_two()
    first["run"]["redacted_values"] = 10 ** 400
    second["run"]["redacted_values"] = 0.5
    report = rollup_report(
        records(("a.json", first), ("b.json", second)), "d")
    assert report["redacted_values"] == {"total": None,
                                         "non_numeric": 0}
    second["run"]["redacted_values"] = 1
    report = rollup_report(
        records(("a.json", first), ("b.json", second)), "d")
    assert report["redacted_values"]["total"] == 10 ** 400 + 1


def test_cli_rollup_reports_a_mixed_directory(tmp_path, capsys):
    (tmp_path / "good.json").write_text(json.dumps(tape_one()),
                                        encoding="utf-8")
    broken = tape_two()
    broken["events"][0]["seq"] = 7
    (tmp_path / "bad.json").write_text(json.dumps(broken),
                                       encoding="utf-8")
    (tmp_path / "raw.json").write_bytes(b"\xff\xfe not utf-8")
    text = json.dumps(tape_two()).replace("hey", "\\ud800")
    (tmp_path / "surrogate.json").write_text(text, encoding="utf-8")
    (tmp_path / "folder.json").mkdir()
    (tmp_path / "notes.txt").write_text("not a tape", encoding="utf-8")
    assert main(["rollup", str(tmp_path)]) == 1
    captured = capsys.readouterr()
    report = json.loads(captured.out)
    assert [entry["file"] for entry in report["tapes"]] == ["good.json"]
    assert report["counts"] == {"tapes": 1, "excluded": 4, "events": 2}
    reasons = {entry["file"]: entry["reason"]
               for entry in report["excluded"]}
    assert reasons == {"bad.json": "invalid",
                       "folder.json": "not a file",
                       "raw.json": "unreadable",
                       "surrogate.json": "undigestible"}
    assert [entry["file"] for entry in report["excluded"]] == [
        "bad.json", "folder.json", "raw.json", "surrogate.json"]
    invalid = next(entry for entry in report["excluded"]
                   if entry["file"] == "bad.json")
    assert invalid["defects"] == validate_tape(broken)
    unreadable = next(entry for entry in report["excluded"]
                      if entry["file"] == "raw.json")
    assert unreadable["error"] == "the file is not valid UTF-8"
    assert "raw.json" in captured.err
    undigestible = next(entry for entry in report["excluded"]
                        if entry["file"] == "surrogate.json")
    assert "cannot be encoded" in undigestible["error"]
    assert "bad.json" in captured.err
    assert "folder.json" in captured.err
    assert "surrogate.json" in captured.err


def test_cli_rollup_of_an_empty_directory(tmp_path, capsys):
    assert main(["rollup", str(tmp_path)]) == 0
    report = json.loads(capsys.readouterr().out)
    assert report["tapes"] == []
    assert report["excluded"] == []
    assert report["counts"] == {"tapes": 0, "excluded": 0, "events": 0}
    assert report["redacted_values"] == {"total": 0, "non_numeric": 0}
    assert report["duration_seconds"] == {"total": 0.0, "known": 0,
                                          "unknown": 0}


def test_cli_rollup_refuses_a_missing_directory(tmp_path, capsys):
    missing = tmp_path / "nope"
    assert main(["rollup", str(missing)]) == 1
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "no such source directory" in captured.err


def test_cli_rollup_output_is_byte_identical_across_runs(
        tmp_path, capsys):
    (tmp_path / "b.json").write_text(json.dumps(tape_two()),
                                     encoding="utf-8")
    (tmp_path / "a.json").write_text(json.dumps(tape_one()),
                                     encoding="utf-8")
    assert main(["rollup", str(tmp_path)]) == 0
    printed = capsys.readouterr().out
    assert main(["rollup", str(tmp_path)]) == 0
    assert capsys.readouterr().out == printed


def test_cli_rollup_names_a_null_tape_as_invalid(tmp_path, capsys):
    (tmp_path / "null.json").write_text("null\n", encoding="utf-8")
    (tmp_path / "good.json").write_text(json.dumps(tape_one()),
                                        encoding="utf-8")
    assert main(["rollup", str(tmp_path)]) == 1
    report = json.loads(capsys.readouterr().out)
    assert report["counts"] == {"tapes": 1, "excluded": 1, "events": 2}
    excluded = report["excluded"][0]
    assert excluded["file"] == "null.json"
    assert excluded["reason"] == "invalid"
    assert excluded["defects"] == validate_tape(None)


def test_cli_rollup_refuses_a_path_that_is_a_file(tmp_path, capsys):
    target = tmp_path / "lonely.json"
    target.write_text(json.dumps(tape_one()), encoding="utf-8")
    assert main(["rollup", str(target)]) == 1
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "no such source directory" in captured.err


def test_cli_rollup_of_a_directory_without_json_files(
        tmp_path, capsys):
    (tmp_path / "notes.txt").write_text("not a tape", encoding="utf-8")
    (tmp_path / "data.JSON").write_text(json.dumps(tape_one()),
                                        encoding="utf-8")
    assert main(["rollup", str(tmp_path)]) == 0
    report = json.loads(capsys.readouterr().out)
    assert report["counts"] == {"tapes": 0, "excluded": 0, "events": 0}


def test_cli_rollup_is_identical_across_creation_orders(
        tmp_path, capsys):
    (tmp_path / "b.json").write_text(json.dumps(tape_two()),
                                     encoding="utf-8")
    (tmp_path / "a.json").write_text(json.dumps(tape_one()),
                                     encoding="utf-8")
    assert main(["rollup", str(tmp_path)]) == 0
    first = capsys.readouterr().out
    (tmp_path / "a.json").unlink()
    (tmp_path / "b.json").unlink()
    (tmp_path / "a.json").write_text(json.dumps(tape_one()),
                                     encoding="utf-8")
    (tmp_path / "b.json").write_text(json.dumps(tape_two()),
                                     encoding="utf-8")
    assert main(["rollup", str(tmp_path)]) == 0
    assert capsys.readouterr().out == first


def test_a_sum_past_the_rendering_digit_limit_reports_null():
    # Each value sits exactly at the 4300 digit rendering
    # limit, so both tapes digest; their sum crosses it.
    big = 5 * 10 ** 4299
    first, second = tape_one(), tape_two()
    first["run"]["totals"] = {"big": big}
    second["run"]["totals"] = {"big": big}
    first["run"]["skipped_lines"] = {"big": big}
    second["run"]["skipped_lines"] = {"big": big}
    report = rollup_report(
        records(("a.json", first), ("b.json", second)), "d")
    assert report["totals"] == {"big": None}
    assert report["skipped_lines"] == {"big": None}
    first["run"]["redacted_values"] = big
    second["run"]["redacted_values"] = big
    report = rollup_report(
        records(("a.json", first), ("b.json", second)), "d")
    assert report["redacted_values"]["total"] is None


def test_cli_rollup_renders_a_report_despite_a_huge_sum(
        tmp_path, capsys):
    big = int("5" + "0" * 4299)
    for name in ("a.json", "b.json"):
        tape = tape_one()
        tape["run"]["totals"] = {"big": big}
        (tmp_path / name).write_text(json.dumps(tape),
                                     encoding="utf-8")
    assert main(["rollup", str(tmp_path)]) == 0
    report = json.loads(capsys.readouterr().out)
    assert report["counts"]["tapes"] == 2
    assert report["totals"] == {"big": None}


def test_malformed_records_raise_a_named_error():
    import pytest
    tape = tape_one()
    both = {"file": "x.json", "tape": tape, "reason": "invalid",
            "digest": tape_digest(tape)}
    with pytest.raises(ValueError, match="x.json"):
        rollup_report([both], "d")
    with pytest.raises(ValueError, match="x.json"):
        rollup_report([{"file": "x.json", "tape": None,
                        "digest": "0" * 64}], "d")
    with pytest.raises(ValueError, match="x.json"):
        rollup_report([{"file": "x.json", "tape": tape}], "d")
    with pytest.raises(ValueError, match="x.json"):
        rollup_report([{"file": "x.json"}], "d")


def test_an_exclusions_defects_are_copied_not_aliased():
    defects = ["one defect"]
    record = {"file": "x.json", "reason": "invalid",
              "defects": defects}
    report = rollup_report([record], "d")
    report["excluded"][0]["defects"].append("another")
    assert defects == ["one defect"]
