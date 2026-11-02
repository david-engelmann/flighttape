"""Tape slicing (FT-39 to FT-43).

A slice is an excerpt of a valid tape that is itself a valid
tape, cut for citation and sharing: the moments a retro or a
bug report needs, without the whole run. The slice renumbers
its events from 0 because the validator requires consecutive
seq values, recomputes its outcome and its countable totals
over the selected events, and records where it was cut from
at run.slice_of, the source tape's digest under canonical
form 1, so a citation can always be walked back to the exact
source it came from.
"""
from __future__ import annotations

import copy

from .identity import tape_digest
from .outcome import determine_outcome

EVENT_KINDS = (
    "session_start",
    "message",
    "tool_call",
    "tool_result",
    "diff",
    "spend",
    "note",
    "session_end",
)


class SliceError(ValueError):
    """A selection that cannot be cut, named for the CLI."""


def check_selection(from_seq, to_seq, kinds):
    """Refuse a malformed selection before any file is read."""
    if from_seq is not None and from_seq < 0:
        raise SliceError(f"the --from bound is below zero, {from_seq}")
    if to_seq is not None and to_seq < 0:
        raise SliceError(f"the --to bound is below zero, {to_seq}")
    if (from_seq is not None and to_seq is not None
            and from_seq > to_seq):
        raise SliceError(
            f"the --from bound {from_seq} is above the --to bound "
            f"{to_seq}")
    for kind in kinds or ():
        if kind not in EVENT_KINDS:
            raise SliceError(f"unknown event kind, {kind}")


def slice_tape(tape, from_seq=None, to_seq=None, kinds=None):
    """Cut the selected events out of a valid tape (FT-40).

    The source is not mutated. The digest of the source is
    taken before anything is built, so a source that cannot
    be digested raises DigestError here, the way compare and
    rollup refuse an undigestible tape.
    """
    check_selection(from_seq, to_seq, kinds)
    digest = tape_digest(tape)
    wanted = set(kinds) if kinds else None
    selected = []
    for event in tape["events"]:
        seq = event["seq"]
        if from_seq is not None and seq < from_seq:
            continue
        if to_seq is not None and seq > to_seq:
            continue
        if wanted is not None and event["kind"] not in wanted:
            continue
        clone = copy.deepcopy(event)
        clone["seq"] = len(selected)
        selected.append(clone)
    if not selected:
        raise SliceError("the selection matches no events")
    result = copy.deepcopy(tape)
    run = result["run"]
    run["totals"] = {
        "messages": sum(1 for event in selected
                        if event["kind"] == "message"),
        "tool_calls": sum(1 for event in selected
                          if event["kind"] == "tool_call"),
    }
    run["slice_of"] = {
        "digest": digest,
        "from": from_seq,
        "to": to_seq,
        "kinds": list(kinds) if kinds else None,
        "events": len(selected),
    }
    result["events"] = selected
    result["outcome"] = determine_outcome(selected)
    return result
