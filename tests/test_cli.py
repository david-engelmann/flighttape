import json, pathlib

from flighttape.cli import main

FIXTURE = (pathlib.Path(__file__).resolve().parent / "fixtures"
           / "claude_session_synthetic.jsonl")


def test_validate_valid_tape_exits_zero(tmp_path, capsys):
    out = tmp_path / "tape.json"
    assert main(["convert", str(FIXTURE), "-o", str(out)]) == 0
    assert main(["validate", str(out)]) == 0
    assert "valid tape" in capsys.readouterr().out


def test_validate_malformed_tape_exits_one(tmp_path, capsys):
    bad = tmp_path / "bad.json"
    bad.write_text(json.dumps({"tape_version": "9.9"}), encoding="utf-8")
    assert main(["validate", str(bad)]) == 1
    assert "defect" in capsys.readouterr().err


def test_convert_empty_session_exits_one(tmp_path, capsys):
    empty = tmp_path / "empty.jsonl"
    empty.write_text("", encoding="utf-8")
    assert main(["convert", str(empty)]) == 1
    assert "defect" in capsys.readouterr().err


def test_play_renders_a_valid_tape(tmp_path, capsys):
    out = tmp_path / "tape.json"
    assert main(["convert", str(FIXTURE), "-o", str(out)]) == 0
    assert main(["play", str(out)]) == 0
    assert "tape synthetic-session-001" in capsys.readouterr().out


def test_play_malformed_tape_exits_one_with_no_rendering(tmp_path, capsys):
    bad = tmp_path / "bad.json"
    bad.write_text(json.dumps({"tape_version": "9.9"}), encoding="utf-8")
    assert main(["play", str(bad)]) == 1
    captured = capsys.readouterr()
    assert "defect" in captured.err
    assert "tape " not in captured.out


def test_convert_dir_writes_a_tape_and_a_summary(tmp_path):
    src = tmp_path / "sessions"
    src.mkdir()
    (src / "a.jsonl").write_text(
        FIXTURE.read_text(encoding="utf-8"), encoding="utf-8")
    out = tmp_path / "tapes"
    assert main(["convert-dir", str(src), "-o", str(out)]) == 0
    summary = json.loads((out / "summary.json").read_text(encoding="utf-8"))
    assert summary["converted"] == 1
    assert summary["failed"] == 0
    assert summary["events"] == 5
    assert (out / "a.tape.json").exists()
