"""Play a flight tape back as a readable timeline (FT-10 to FT-12).

The header also states the outcome basis the tape carries
(FT-17). The renderer reads the outcome defensively, a status or
basis that is not a string renders as the bare unknown rather
than breaking the rendering of a tape that was never validated.

The rendering is a pure function of the tape. The same tape always
renders byte identical output, and tests/test_play.py holds the
rendering to a stored golden file built from the synthetic fixture.
The golden file provenance is recorded in tests/test_play.py.
"""
from __future__ import annotations

import json

from .convert_claude import TOKEN_KEYS
from .validate import parse_timestamp

COUNTED = (("tool_call", "tool calls"), ("message", "messages"))
KNOWN_FIELDS = {"seq", "kind", "sidechain", "timestamp", "call_id",
                "tool", "role", "text", "content", "input", "is_error"}


def _offset(stamp, origin):
    if stamp is None or origin is None:
        return "no time"
    delta = stamp.timestamp() - origin.timestamp()
    if abs(delta) < 10:
        return f"{delta:+.2f}s"
    return f"{delta:+.1f}s"


def _header_lines(tape):
    run = tape.get("run", {})
    totals = run.get("totals") or {}
    events = tape.get("events", [])
    parts = [f"events {len(events)}"]
    for kind, label in COUNTED:
        counted = sum(1 for e in events if e.get("kind") == kind)
        parts.append(f"{label} {counted}")
    for key in TOKEN_KEYS:
        if key in totals:
            parts.append(f"{key.replace('_', ' ')} {totals[key]}")
    outcome = tape.get("outcome")
    if not isinstance(outcome, dict):
        outcome = {}
    status = outcome.get("status")
    if isinstance(status, str):
        outcome_text = status
        basis = outcome.get("basis")
        if isinstance(basis, str) and basis:
            outcome_text += f" ({basis})"
    else:
        outcome_text = "unknown"
    lines = [
        f"tape {run.get('session_id', 'unknown')} "
        f"from {run.get('harness', 'unknown')}",
        f"outcome {outcome_text}, " + ", ".join(parts),
    ]
    meta = []
    if run.get("started_at"):
        meta.append(f"started {run['started_at']}")
    if run.get("source_format"):
        meta.append(f"source {run['source_format']}")
    skipped = run.get("skipped_lines") or {}
    if skipped:
        detail = ", ".join(f"{k} {v}" for k, v in sorted(skipped.items()))
        meta.append(f"skipped lines {sum(skipped.values())} ({detail})")
    if run.get("redacted_values"):
        meta.append(f"redacted values {run['redacted_values']}")
    if meta:
        lines.append(", ".join(meta))
    lines.append("")
    return lines


def render_tape(tape):
    """Render one validated tape as timeline text."""
    lines = _header_lines(tape)
    events = tape.get("events", [])
    stamps = []
    for event in events:
        ts = event.get("timestamp")
        stamps.append(parse_timestamp(ts) if isinstance(ts, str) else None)
    origin = next((s for s in stamps if s is not None), None)
    for event, stamp in zip(events, stamps):
        kind = event.get("kind", "note")
        side = " sidechain" if event.get("sidechain") else ""
        head = f"[{_offset(stamp, origin)}] #{event.get('seq', '?')} {kind}{side}"
        body = None
        if kind == "message":
            head += f" {event.get('role', 'unknown')}"
            body = event.get("text")
        elif kind == "tool_call":
            head += (f" {event.get('tool', 'unknown')} "
                     f"call {event.get('call_id', 'unknown')}")
            if event.get("input") is not None:
                body = json.dumps(event["input"], sort_keys=True)
        elif kind == "tool_result":
            head += f" call {event.get('call_id', 'unknown')}"
            if event.get("is_error"):
                head += " ERROR"
            body = event.get("content")
        else:
            body = event.get("content", event.get("text"))
        lines.append(head)
        if body:
            lines.append(str(body))
        extra = {k: event[k] for k in sorted(event) if k not in KNOWN_FIELDS}
        if extra:
            lines.append(json.dumps(extra, sort_keys=True))
    return "\n".join(lines) + "\n"
