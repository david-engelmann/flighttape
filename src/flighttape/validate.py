"""Validate a flight tape against the flighttape Contract.

The validator is stdlib only and implements schema/tape.schema.json
by hand, so a tape can be checked anywhere Python runs. It returns a
list of every defect found, never just the first (FT-2).
"""
from __future__ import annotations

from . import TAPE_VERSION

EVENT_KINDS = {
    "session_start", "message", "tool_call", "tool_result",
    "diff", "spend", "note", "session_end",
}
OUTCOMES = {"completed", "failed", "aborted", "unknown"}


def _is_int(value):
    return isinstance(value, int) and not isinstance(value, bool)


def validate_tape(tape):
    """Return a list of defect strings. An empty list means valid."""
    defects = []
    if not isinstance(tape, dict):
        return ["tape is not a JSON object"]
    if tape.get("tape_version") != TAPE_VERSION:
        defects.append(
            f"tape_version must be {TAPE_VERSION!r}, "
            f"got {tape.get('tape_version')!r}")
    run = tape.get("run")
    if not isinstance(run, dict):
        defects.append("run is missing or is not an object")
        run = {}
    for field in ("harness", "session_id"):
        if not isinstance(run.get(field), str) or not run.get(field):
            defects.append(f"run.{field} must be a non-empty string")
    totals = run.get("totals")
    if totals is not None:
        if not isinstance(totals, dict):
            defects.append("run.totals must be an object when present")
        else:
            for key, value in totals.items():
                if not _is_int(value) or value < 0:
                    defects.append(
                        f"run.totals.{key} must be a non-negative integer")
    outcome = tape.get("outcome")
    if not isinstance(outcome, dict):
        defects.append("outcome is missing or is not an object")
    elif outcome.get("status") not in OUTCOMES:
        defects.append(
            f"outcome.status must be one of {sorted(OUTCOMES)}, "
            f"got {outcome.get('status')!r}")
    events = tape.get("events")
    if not isinstance(events, list):
        defects.append("events is missing or is not a list")
        return defects
    if not events:
        defects.append("events must hold at least one event")
    seen_calls = set()
    last_ts = None
    for want_seq, event in enumerate(events):
        where = f"events[{want_seq}]"
        if not isinstance(event, dict):
            defects.append(f"{where} is not an object")
            continue
        if event.get("seq") != want_seq:
            defects.append(
                f"{where}.seq must be {want_seq}, got {event.get('seq')!r}")
        kind = event.get("kind")
        if kind not in EVENT_KINDS:
            defects.append(f"{where}.kind {kind!r} is not a tape event kind")
        if not isinstance(event.get("sidechain"), bool):
            defects.append(f"{where}.sidechain must be a boolean")
        ts = event.get("timestamp")
        if ts is not None:
            if not isinstance(ts, str):
                defects.append(f"{where}.timestamp must be a string")
            elif last_ts is not None and ts < last_ts:
                defects.append(f"{where}.timestamp runs backward")
            else:
                last_ts = ts
        if kind == "tool_call":
            call_id = event.get("call_id")
            if not isinstance(call_id, str) or not call_id:
                defects.append(f"{where}.call_id must be a non-empty string")
            elif call_id in seen_calls:
                defects.append(f"{where}.call_id {call_id!r} is used twice")
            else:
                seen_calls.add(call_id)
            if not isinstance(event.get("tool"), str) or not event.get("tool"):
                defects.append(f"{where}.tool must be a non-empty string")
        if kind == "tool_result":
            call_id = event.get("call_id")
            if call_id not in seen_calls:
                defects.append(
                    f"{where}.call_id {call_id!r} answers no earlier "
                    "tool_call in this tape")
    return defects
