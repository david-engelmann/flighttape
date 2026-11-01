"""Convert a Claude Code session JSONL file into a flight tape.

Mapping, per FT-6 and FT-7 of the Contract. A user or assistant line
becomes message, tool_call, and tool_result events from its content
blocks. Tool result content is carried into the tape as text. Lines
the converter does not turn into events are counted in
run.skipped_lines by reason, so a conversion can be audited for
silent loss. A line from a second session is counted and skipped,
never merged. Malformed lines are counted, never fatal. The tape
records the source format name and the session id only, never a
path, a working directory, or a branch, and absolute paths inside
tool inputs are reduced to their final component and counted in
run.redacted_values (FT-8). The outcome is determined from the
events by flighttape.outcome (FT-14 to FT-16), never a constant.

Robustness, per FT-18. The converter never raises on its input.
Each line is converted on its own and merged only when the whole
line converts, so a line is converted whole or counted whole.
Input nested beyond the interpreter limit is counted as
too-deep, and a session file is read as bytes so a line that is
not valid UTF-8 is counted as undecodable instead of raising.
"""
from __future__ import annotations

import json
import pathlib
import posixpath

from . import TAPE_VERSION
from .outcome import determine_outcome

SOURCE_FORMAT = "claude-code-jsonl"
TOKEN_KEYS = ("input_tokens", "output_tokens",
              "cache_creation_input_tokens", "cache_read_input_tokens")
TOTAL_KEYS = TOKEN_KEYS + ("tool_calls", "messages")


def _sanitize(value, counter):
    if isinstance(value, str) and value.startswith("/") and len(value) > 1:
        counter[0] += 1
        return posixpath.basename(value.rstrip("/")) or value
    if isinstance(value, dict):
        return {key: _sanitize(item, counter) for key, item in value.items()}
    if isinstance(value, list):
        return [_sanitize(item, counter) for item in value]
    return value


def _result_text(content):
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts = []
        for block in content:
            if isinstance(block, dict) and block.get("type") == "text":
                parts.append(str(block.get("text", "")))
        return "\n".join(parts)
    return ""


def _convert_line(line, session_id, base_seq):
    """Convert one parsed session line into tape fragments.

    Returns (events, skipped, totals, redactions, adopt_session,
    started_at). Nothing is merged by the caller when the line
    raises RecursionError, so a line lands whole or is counted
    whole. The session and start time a line establishes are
    returned even when the line itself converts to no events,
    matching the order the checks have always run in.
    """
    events = []
    skipped = {}
    totals = {key: 0 for key in TOTAL_KEYS}
    redactions = [0]
    adopt_session = None
    started_at = None

    def bump(key):
        skipped[key] = skipped.get(key, 0) + 1

    def emit(kind, **fields):
        event = {"seq": base_seq + len(events), "kind": kind,
                 "sidechain": bool(line.get("isSidechain", False))}
        if line.get("timestamp"):
            event["timestamp"] = line["timestamp"]
        event.update(fields)
        events.append(event)

    seen_session = line.get("sessionId")
    if seen_session:
        if session_id is None:
            adopt_session = seen_session
        elif seen_session != session_id:
            bump("foreign-session")
            return events, skipped, totals, 0, None, None
    if line.get("timestamp"):
        started_at = line["timestamp"]
    line_type = line.get("type")
    if line_type not in ("user", "assistant"):
        bump(line_type or "missing-type")
        return events, skipped, totals, redactions[0], adopt_session, started_at
    message = line.get("message")
    if not isinstance(message, dict):
        bump("malformed:message")
        return events, skipped, totals, redactions[0], adopt_session, started_at
    role = message.get("role") or line_type
    usage = message.get("usage")
    if isinstance(usage, dict):
        for key in TOKEN_KEYS:
            value = usage.get(key)
            if isinstance(value, int) and not isinstance(value, bool):
                totals[key] += value
    elif usage is not None:
        bump("malformed:usage")
    content = message.get("content")
    if isinstance(content, str):
        content = [{"type": "text", "text": content}] if content else []
    if not isinstance(content, list) or not content:
        bump(f"contentless:{line_type}")
        return events, skipped, totals, redactions[0], adopt_session, started_at
    made = 0
    for block in content:
        if not isinstance(block, dict):
            bump("block:non-object")
            continue
        block_type = block.get("type")
        if block_type == "text":
            if block.get("text"):
                emit("message", role=role, text=block["text"])
                totals["messages"] += 1
                made += 1
        elif block_type == "tool_use":
            emit("tool_call", call_id=block.get("id"),
                 tool=block.get("name"),
                 input=_sanitize(block.get("input") or {}, redactions))
            totals["tool_calls"] += 1
            made += 1
        elif block_type == "tool_result":
            emit("tool_result",
                 call_id=block.get("tool_use_id"),
                 is_error=bool(block.get("is_error", False)),
                 content=_result_text(block.get("content")))
            made += 1
        else:
            bump(f"block:{block_type}")
    if made == 0:
        bump(f"contentless:{line_type}")
    return events, skipped, totals, redactions[0], adopt_session, started_at


def convert_lines(lines):
    """Convert an iterable of JSONL strings into a tape dict.

    An entry of None marks a line that was not valid UTF-8, which
    convert_file produces from the raw bytes; it is counted as
    undecodable. No entry, however malformed, raises (FT-18).
    """
    events = []
    skipped = {}
    redactions = 0
    session_id = None
    started_at = None
    totals = {key: 0 for key in TOTAL_KEYS}

    def bump(key):
        skipped[key] = skipped.get(key, 0) + 1

    for raw in lines:
        if raw is None:
            bump("undecodable")
            continue
        raw = raw.strip()
        if not raw:
            continue
        try:
            line = json.loads(raw)
        except RecursionError:
            bump("too-deep")
            continue
        except ValueError:
            bump("unparseable")
            continue
        if not isinstance(line, dict):
            bump("non-object")
            continue
        try:
            (line_events, line_skipped, line_totals, line_redactions,
             adopt_session, line_started) = _convert_line(
                line, session_id, len(events))
        except RecursionError:
            bump("too-deep")
            continue
        events.extend(line_events)
        for key, value in line_skipped.items():
            skipped[key] = skipped.get(key, 0) + value
        for key, value in line_totals.items():
            totals[key] += value
        redactions += line_redactions
        if session_id is None and adopt_session is not None:
            session_id = adopt_session
        if started_at is None and line_started is not None:
            started_at = line_started
    run = {"harness": "claude-code",
           "session_id": session_id or "unknown",
           "source_format": SOURCE_FORMAT, "skipped_lines": skipped,
           "redacted_values": redactions, "totals": totals}
    if started_at:
        run["started_at"] = started_at
    return {"tape_version": TAPE_VERSION, "run": run, "events": events,
            "outcome": determine_outcome(events)}


def convert_file(path):
    """Convert a session file, reading it as bytes (FT-18).

    A line that is not valid UTF-8 is handed to convert_lines as
    None and counted as undecodable, never raised.
    """
    data = pathlib.Path(path).read_bytes()
    lines = []
    for raw in data.split(b"\n"):
        try:
            lines.append(raw.decode("utf-8"))
        except UnicodeDecodeError:
            lines.append(None)
    return convert_lines(lines)
