import pathlib

from flighttape.convert_claude import convert_file
from flighttape.play import render_tape

FIXTURE = (pathlib.Path(__file__).resolve().parent / "fixtures"
           / "claude_session_synthetic.jsonl")
GOLDEN = (pathlib.Path(__file__).resolve().parent / "fixtures"
          / "playback_golden.txt")


def fixture_tape():
    return convert_file(FIXTURE)


def test_rendering_matches_the_golden_file():
    assert render_tape(fixture_tape()) == GOLDEN.read_text(encoding="utf-8")


def test_rendering_is_deterministic():
    tape = fixture_tape()
    assert render_tape(tape) == render_tape(tape)


def test_header_carries_run_facts():
    out = render_tape(fixture_tape())
    assert "synthetic-session-001" in out
    assert "claude-code" in out
    assert "outcome unknown" in out
    assert "tool calls 1" in out
    assert "input tokens 150" in out


def test_every_carried_field_appears():
    out = render_tape(fixture_tape())
    for needle in ("Synthetic prompt, fix the widget counter.",
                   "Read", "toolu_synth_1", "counter = 0",
                   "sidechain", "Synthetic sidechain note"):
        assert needle in out, needle


def test_offsets_are_seconds_from_the_first_timestamp():
    out = render_tape(fixture_tape())
    assert "[+0.0s] #0" in out
    assert "[+5.0s] #1" in out
    assert "[+60.0s]" in out


def test_error_results_are_marked():
    tape = fixture_tape()
    for event in tape["events"]:
        if event["kind"] == "tool_result":
            event["is_error"] = True
    assert "ERROR" in render_tape(tape)
