"""Convert a Claude Code session JSONL file into a flight tape.

Mapping, per FT-6 and FT-7 of the Contract. A user or assistant line
becomes message, tool_call, and tool_result events from its content
blocks. Lines of any other type are counted in run.skipped_lines by
type and never silently dropped. The tape records the source format
name and session id only, never a path, a working directory, or a
branch (FT-8).
"""
from __future__ import annotations

import json

from . import TAPE_VERSION

SOURCE_FORMAT = "claude-code-jsonl"


def convert_lines(lines):
    """Convert an iterable of JSONL strings into a tape dict."""
    events = []
    skipped = {}
    session_id = "unknown"
    started_at = None
    totals = {"input_tokens": 0, "output_tokens": 0,
              "tool_calls": 0, "messages": 0}

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
            skipped["unparseable"] = skipped.get("unparseable", 0) + 1
            continue
        if line.get("sessionId"):
            session_id = line["sessionId"]
        if started_at is None and line.get("timestamp"):
            started_at = line["timestamp"]
        line_type = line.get("type")
        if line_type not in ("user", "assistant"):
            skipped[line_type or "missing-type"] = (
                skipped.get(line_type or "missing-type", 0) + 1)
            continue
        message = line.get("message") or {}
        role = message.get("role") or line_type
        usage = message.get("usage") or {}
        for key in ("input_tokens", "output_tokens"):
            value = usage.get(key)
            if isinstance(value, int):
                totals[key] += value
        content = message.get("content")
        if isinstance(content, str):
            content = [{"type": "text", "text": content}] if content else []
        for block in content or []:
            if not isinstance(block, dict):
                continue
            block_type = block.get("type")
            if block_type == "text":
                if block.get("text"):
                    emit("message", line, role=role, text=block["text"])
                    totals["messages"] += 1
            elif block_type == "tool_use":
                emit("tool_call", line, call_id=block.get("id"),
                     tool=block.get("name"),
                     input=block.get("input") or {})
                totals["tool_calls"] += 1
            elif block_type == "tool_result":
                emit("tool_result", line,
                     call_id=block.get("tool_use_id"),
                     is_error=bool(block.get("is_error", False)))
            else:
                key = f"block:{block_type}"
                skipped[key] = skipped.get(key, 0) + 1
    if not events:
        events.append({"seq": 0, "kind": "session_start",
                       "sidechain": False})
    run = {"harness": "claude-code", "session_id": session_id,
           "source_format": SOURCE_FORMAT, "skipped_lines": skipped,
           "totals": totals}
    if started_at:
        run["started_at"] = started_at
    return {"tape_version": TAPE_VERSION, "run": run, "events": events,
            "outcome": {"status": "unknown"}}


def convert_file(path):
    with open(path, encoding="utf-8") as handle:
        return convert_lines(handle)
