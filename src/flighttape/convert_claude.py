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
"""
from __future__ import annotations

import json
import posixpath

from . import TAPE_VERSION
from .outcome import determine_outcome

SOURCE_FORMAT = "claude-code-jsonl"
TOKEN_KEYS = ("input_tokens", "output_tokens",
              "cache_creation_input_tokens", "cache_read_input_tokens")


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


def convert_lines(lines):
    """Convert an iterable of JSONL strings into a tape dict."""
    events = []
    skipped = {}
    redactions = [0]
    session_id = None
    started_at = None
    totals = {key: 0 for key in TOKEN_KEYS}
    totals.update({"tool_calls": 0, "messages": 0})

    def bump(key):
        skipped[key] = skipped.get(key, 0) + 1

    def emit(kind, line, **fields):
        event = {"seq": len(events), "kind": kind,
                 "sidechain": bool(line.get("isSidechain", False))}
        if line.get("timestamp"):
            event["timestamp"] = line["timestamp"]
        event.update(fields)
        events.append(event)

    for raw in lines:
        raw = raw.strip()
        if not raw:
            continue
        try:
            line = json.loads(raw)
        except json.JSONDecodeError:
            bump("unparseable")
            continue
        if not isinstance(line, dict):
            bump("non-object")
            continue
        line_session = line.get("sessionId")
        if line_session:
            if session_id is None:
                session_id = line_session
            elif line_session != session_id:
                bump("foreign-session")
                continue
        if started_at is None and line.get("timestamp"):
            started_at = line["timestamp"]
        line_type = line.get("type")
        if line_type not in ("user", "assistant"):
            bump(line_type or "missing-type")
            continue
        message = line.get("message")
        if not isinstance(message, dict):
            bump("malformed:message")
            continue
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
            continue
        made = 0
        for block in content:
            if not isinstance(block, dict):
                bump("block:non-object")
                continue
            block_type = block.get("type")
            if block_type == "text":
                if block.get("text"):
                    emit("message", line, role=role, text=block["text"])
                    totals["messages"] += 1
                    made += 1
            elif block_type == "tool_use":
                emit("tool_call", line, call_id=block.get("id"),
                     tool=block.get("name"),
                     input=_sanitize(block.get("input") or {}, redactions))
                totals["tool_calls"] += 1
                made += 1
            elif block_type == "tool_result":
                emit("tool_result", line,
                     call_id=block.get("tool_use_id"),
                     is_error=bool(block.get("is_error", False)),
                     content=_result_text(block.get("content")))
                made += 1
            else:
                bump(f"block:{block_type}")
        if made == 0:
            bump(f"contentless:{line_type}")
    run = {"harness": "claude-code",
           "session_id": session_id or "unknown",
           "source_format": SOURCE_FORMAT, "skipped_lines": skipped,
           "redacted_values": redactions[0], "totals": totals}
    if started_at:
        run["started_at"] = started_at
    return {"tape_version": TAPE_VERSION, "run": run, "events": events,
            "outcome": determine_outcome(events)}


def convert_file(path):
    with open(path, encoding="utf-8") as handle:
        return convert_lines(handle)
