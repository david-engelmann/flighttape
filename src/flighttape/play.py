"""Play a flight tape back as a readable timeline (FT-10 to FT-12).

The rendering is a pure function of the tape. The same tape always
renders byte identical output, and tests/test_play.py holds the
rendering to a stored golden file built from the synthetic fixture.
"""
from __future__ import annotations

from .validate import _parse_ts


def _totals_line(tape):
    totals = tape.get("run", {}).get("totals") or {}
    events = tape.get("events", [])
    tool_calls = totals.get("tool_calls")
    if tool_calls is None:
        tool_calls = sum(1 for e in events if e.get("kind") == "tool_call")
    parts = [f"events {len(events)}", f"tool calls {tool_calls}"]
    for key, label in (("input_tokens", "input tokens"),
                       ("output_tokens", "output tokens"),
                       ("cache_creation_input_tokens", "cache creation tokens"),
                       ("cache_read_input_tokens", "cache read tokens")):
        if totals.get(key):
            parts.append(f"{label} {totals[key]}")
    return ", ".join(parts)


def render_tape(tape):
    """Render one validated tape as timeline text."""
    run = tape.get("run", {})
    outcome = tape.get("outcome", {}).get("status", "unknown")
    lines = [
        f"tape {run.get('session_id', 'unknown')} "
        f"from {run.get('harness', 'unknown')}",
        f"outcome {outcome}, {_totals_line(tape)}",
        "",
    ]
    stamps = []
    for event in tape.get("events", []):
        ts = event.get("timestamp")
        stamps.append(_parse_ts(ts) if isinstance(ts, str) else None)
    origin = next((s for s in stamps if s is not None), None)
    for event, stamp in zip(tape.get("events", []), stamps):
        if origin is not None and stamp is not None:
            offset = f"+{stamp.timestamp() - origin.timestamp():.1f}s"
        else:
            offset = "no time"
        kind = event.get("kind", "note")
        side = " sidechain" if event.get("sidechain") else ""
        head = f"[{offset}] #{event.get('seq', '?')} {kind}{side}"
        if kind == "message":
            head += f" {event.get('role', 'unknown')}"
            lines.append(head)
            if event.get("text"):
                lines.append(str(event["text"]))
        elif kind == "tool_call":
            head += f" {event.get('tool', 'unknown')} call {event.get('call_id', 'unknown')}"
            lines.append(head)
        elif kind == "tool_result":
            head += f" call {event.get('call_id', 'unknown')}"
            if event.get("is_error"):
                head += " ERROR"
            lines.append(head)
            if event.get("content"):
                lines.append(str(event["content"]))
        else:
            lines.append(head)
            if event.get("text"):
                lines.append(str(event["text"]))
    return "\n".join(lines) + "\n"
