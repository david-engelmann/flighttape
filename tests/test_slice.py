"""FT-39 to FT-43, tape slicing."""
import copy
import json

import pytest

from flighttape.cli import main
from flighttape.identity import tape_digest
from flighttape.slice import SliceError, slice_tape
from flighttape.validate import validate_tape

T0 = "2026-01-01T09:00:00.000Z"
T1 = "2026-01-01T09:00:10.000Z"
T2 = "2026-01-01T09:00:20.000Z"
T3 = "2026-01-01T09:00:30.000Z"


def event(seq, kind, **extra):
    base = {"seq": seq, "kind": kind, "sidechain": False,
            "timestamp": T0}
    base.update(extra)
    return base


def source_tape():
    return {
        "tape_version": "1.0",
        "run": {"harness": "claude-code", "session_id": "slice-me",
                "source_format": "claude-code-jsonl",
                "started_at": T0, "skipped_lines": {"system": 1},
                "redacted_values": 2,
                "totals": {"input_tokens": 150, "output_tokens": 50,
                           "tool_calls": 1, "messages": 2}},
        "events": [
            event(0, "message", role="user", text="do it",
                  timestamp=T0),
            event(1, "message", role="assistant", text="done",
                  timestamp=T1),
            event(2, "tool_call", tool="run", call_id="call-1",
                  timestamp=T2),
            event(3, "note", text="later", timestamp=T3),
        ],
        "outcome": {"status": "completed",
                    "basis": "final-assistant-message"},
    }


def test_a_range_slice_renumbers_and_keeps_the_events():
    cut = slice_tape(source_tape(), from_seq=2, to_seq=3)
    assert [e["kind"] for e in cut["events"]] == ["tool_call", "note"]
    assert [e["seq"] for e in cut["events"]] == [0, 1]
    assert cut["events"][1]["text"] == "later"
    assert validate_tape(cut) == []


def test_bounds_omit_to_the_tapes_ends_and_clamp_to_what_exists():
    tape = source_tape()
    whole = slice_tape(tape)
    assert [e["seq"] for e in whole["events"]] == [0, 1, 2, 3]
    past = slice_tape(tape, to_seq=99)
    assert len(past["events"]) == 4
    with pytest.raises(SliceError):
        slice_tape(tape, from_seq=99)


def test_a_kind_filter_selects_across_the_range():
    cut = slice_tape(source_tape(), kinds=["message"])
    assert [e["seq"] for e in cut["events"]] == [0, 1]
    assert all(e["kind"] == "message" for e in cut["events"])
    both = slice_tape(source_tape(), from_seq=1,
                      kinds=["message", "note"])
    assert [e["kind"] for e in both["events"]] == ["message", "note"]


def test_provenance_records_the_source_digest_and_the_request():
    tape = source_tape()
    cut = slice_tape(tape, from_seq=1, kinds=["note"])
    info = cut["run"]["slice_of"]
    assert info == {"digest": tape_digest(tape), "from": 1,
                    "to": None, "kinds": ["note"], "events": 1}
    again = slice_tape(cut)
    assert again["run"]["slice_of"]["digest"] == tape_digest(cut)
    assert tape_digest(cut) != tape_digest(tape)


def test_the_outcome_is_recomputed_never_carried():
    cut = slice_tape(source_tape(), to_seq=0)
    assert cut["outcome"] == {"status": "unknown",
                              "basis": "unresolved-ending"}
    whole = slice_tape(source_tape())
    assert whole["outcome"] == {"status": "unknown",
                                "basis": "unresolved-ending"}


def test_totals_are_recomputed_and_token_totals_are_omitted():
    cut = slice_tape(source_tape(), to_seq=2)
    assert cut["run"]["totals"] == {"messages": 2, "tool_calls": 1}
    notes = slice_tape(source_tape(), kinds=["note"])
    assert notes["run"]["totals"] == {"messages": 0, "tool_calls": 0}


def test_run_identity_is_carried_as_recorded():
    cut = slice_tape(source_tape(), from_seq=3)
    run = cut["run"]
    assert run["harness"] == "claude-code"
    assert run["session_id"] == "slice-me"
    assert run["source_format"] == "claude-code-jsonl"
    assert run["started_at"] == T0
    assert run["skipped_lines"] == {"system": 1}
    assert run["redacted_values"] == 2


def test_the_source_is_not_mutated():
    tape = source_tape()
    before = copy.deepcopy(tape)
    cut = slice_tape(tape, from_seq=1)
    cut["events"][0]["text"] = "changed"
    cut["run"]["session_id"] = "changed"
    assert tape == before


def test_a_slice_of_only_session_end_events_has_no_outcome_events():
    tape = source_tape()
    tape["events"].append(event(4, "session_end", timestamp=T3))
    tape["outcome"] = {"status": "completed",
                        "basis": "final-assistant-message"}
    cut = slice_tape(tape, kinds=["session_end"])
    assert cut["outcome"] == {"status": "unknown",
                              "basis": "no-events"}
    assert validate_tape(cut) == []


def test_malformed_selections_are_refused_by_name():
    tape = source_tape()
    with pytest.raises(SliceError):
        slice_tape(tape, from_seq=-1)
    with pytest.raises(SliceError):
        slice_tape(tape, from_seq=3, to_seq=1)
    with pytest.raises(SliceError):
        slice_tape(tape, kinds=["bogus"])


def write(tmp_path, tape, name="tape.json"):
    path = tmp_path / name
    path.write_text(json.dumps(tape), encoding="utf-8")
    return str(path)


def test_cli_slice_to_a_file_and_to_stdout(tmp_path, capsys):
    source = write(tmp_path, source_tape())
    out = tmp_path / "cut.json"
    assert main(["slice", source, "--from", "2", "-o",
                 str(out)]) == 0
    cut = json.loads(out.read_text(encoding="utf-8"))
    assert [e["kind"] for e in cut["events"]] == ["tool_call", "note"]
    assert main(["slice", source, "--kind", "note"]) == 0
    printed = json.loads(capsys.readouterr().out)
    assert [e["kind"] for e in printed["events"]] == ["note"]


def test_cli_refusals_write_nothing(tmp_path, capsys):
    source = write(tmp_path, source_tape())
    out = tmp_path / "cut.json"
    assert main(["slice", source, "--from", "3", "--to", "1",
                 "-o", str(out)]) == 1
    assert "above the --to bound" in capsys.readouterr().err
    assert not out.exists()
    assert main(["slice", source, "--kind", "bogus",
                 "-o", str(out)]) == 1
    assert "unknown event kind" in capsys.readouterr().err
    assert not out.exists()
    assert main(["slice", source, "--from", "99",
                 "-o", str(out)]) == 1
    assert "matches no events" in capsys.readouterr().err
    assert not out.exists()
    assert main(["slice", str(tmp_path / "missing.json")]) == 1
    assert not out.exists()


def test_cli_an_invalid_source_is_refused_with_its_defects(
        tmp_path, capsys):
    bad = source_tape()
    bad["events"][1]["seq"] = 7
    source = write(tmp_path, bad)
    out = tmp_path / "cut.json"
    assert main(["slice", source, "-o", str(out)]) == 1
    err = capsys.readouterr().err
    assert "is not a valid tape" in err
    assert "defect," in err
    assert not out.exists()
