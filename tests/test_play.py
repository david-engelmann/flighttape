"""Playback tests.

Golden provenance, tests/fixtures/playback_golden.txt was produced
by rendering the synthetic fixture tape with the Round 2 renderer
and then verified by hand against the fixture before it was kept,
offsets 0, 5, 5, 6, and 60 seconds from the fixture timestamps and
header token sums 150 and 50 from the fixture usage. Any later
regeneration must repeat that hand check, and the regeneration is
one command, render convert_file on the fixture and write the
result over the golden file. Round 3 regenerated the golden once,
the only change is the header outcome segment, which now carries
the determined basis (unresolved-ending), hand checked against the
fixture, whose main chain ends on a tool result with no following
main chain reply, the sidechain reply does not count.
"""
import copy, pathlib

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


def test_rendering_is_deterministic_and_does_not_mutate_the_tape():
    tape = fixture_tape()
    before = copy.deepcopy(tape)
    first = render_tape(tape)
    assert tape == before
    assert render_tape(tape) == first


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
    assert "[+0.00s] #0" in out
    assert "[+5.00s] #1" in out
    assert "[+60.0s]" in out


def test_sub_second_offsets_stay_visible():
    tape = fixture_tape()
    tape["events"][1]["timestamp"] = "2026-01-01T09:00:00.040Z"
    out = render_tape(tape)
    assert "[+0.04s] #1" in out


def test_untimestamped_events_render_no_time():
    tape = fixture_tape()
    for event in tape["events"]:
        event.pop("timestamp", None)
    out = render_tape(tape)
    assert "[no time] #0" in out


def test_tool_call_input_is_rendered():
    out = render_tape(fixture_tape())
    assert '"path": "widget.py"' in out


def test_diff_and_spend_content_is_rendered():
    tape = fixture_tape()
    tape["events"].append({"seq": 5, "kind": "diff", "sidechain": False,
                            "timestamp": "2026-01-01T09:02:00.000Z",
                            "content": "counter = 0 becomes counter = 1"})
    tape["events"].append({"seq": 6, "kind": "spend", "sidechain": False,
                            "timestamp": "2026-01-01T09:02:01.000Z",
                            "content": "0.0042 USD"})
    out = render_tape(tape)
    assert "counter = 0 becomes counter = 1" in out
    assert "0.0042 USD" in out


def test_extra_event_fields_are_rendered_not_dropped():
    tape = fixture_tape()
    tape["events"][0]["model"] = "synthetic-model"
    out = render_tape(tape)
    assert '"model": "synthetic-model"' in out


def test_zero_totals_render_and_cache_labels_are_exact():
    tape = fixture_tape()
    assert "cache creation input tokens 0" in render_tape(tape)
    tape["run"]["totals"]["cache_creation_input_tokens"] = 500
    tape["run"]["totals"]["cache_read_input_tokens"] = 900
    out = render_tape(tape)
    assert "cache creation input tokens 500" in out
    assert "cache read input tokens 900" in out


def test_counted_figures_come_from_the_events_not_the_totals():
    tape = fixture_tape()
    tape["run"]["totals"]["tool_calls"] = 99
    out = render_tape(tape)
    assert "tool calls 1" in out
    assert "tool calls 99" not in out


def test_totals_absent_fallback_counts_events():
    tape = fixture_tape()
    tape["run"].pop("totals")
    out = render_tape(tape)
    assert "tool calls 1" in out
    assert "messages 3" in out


def test_header_states_run_metadata_and_loss_counts():
    out = render_tape(fixture_tape())
    assert "started 2026-01-01T09:00:00.000Z" in out
    assert "source claude-code-jsonl" in out
    assert "skipped lines 1 (system 1)" in out


def test_error_results_are_marked():
    tape = fixture_tape()
    for event in tape["events"]:
        if event["kind"] == "tool_result":
            event["is_error"] = True
    assert "ERROR" in render_tape(tape)


def test_header_states_the_outcome_basis():
    tape = fixture_tape()
    header = render_tape(tape).splitlines()[1]
    assert header.startswith("outcome unknown (unresolved-ending), ")
    tape["outcome"] = {"status": "completed"}
    header = render_tape(tape).splitlines()[1]
    assert header.startswith("outcome completed, ")


def test_header_survives_an_unvalidated_outcome_shape():
    tape = fixture_tape()
    tape["outcome"] = {"status": 5, "basis": "final-error-result"}
    header = render_tape(tape).splitlines()[1]
    assert header.startswith("outcome unknown, ")
    tape["outcome"] = {"status": "completed", "basis": 7}
    header = render_tape(tape).splitlines()[1]
    assert header.startswith("outcome completed, ")
    tape["outcome"] = "completed"
    header = render_tape(tape).splitlines()[1]
    assert header.startswith("outcome unknown, ")
