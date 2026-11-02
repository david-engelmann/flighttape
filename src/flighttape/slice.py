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
from .validate import EVENT_KINDS, validate_tape

_HUGE = 10 ** 18


class SliceError(ValueError):
    """A selection that cannot be cut, named for the CLI."""


def parse_bound(text, flag):
    """Read one CLI bound (FT-39).

    Bounds arrive as text so that an absurdly long digit
    string clamps to a huge sentinel instead of dying in
    the interpreter's integer parsing limit, and a
    non-integer is a named refusal, never a traceback.
    """
    if text is None:
        return None
    try:
        return int(text)
    except ValueError:
        digits = text.lstrip("+-")
        if digits.isdigit() and len(digits) > 18:
            return -_HUGE if text.startswith("-") else _HUGE
        raise SliceError(
            f"the {flag} bound is not an integer, {text}") from None


def check_selection(from_seq, to_seq, kinds):
    """Refuse a malformed selection before any file is read."""
    for flag, bound in (("--from", from_seq), ("--to", to_seq)):
        if bound is None:
            continue
        if isinstance(bound, bool) or not isinstance(bound, int):
            raise SliceError(
                f"the {flag} bound is not an integer, {bound!r}")
        if bound < 0:
            raise SliceError(f"the {flag} bound is below zero, {bound}")
    if (from_seq is not None and to_seq is not None
            and from_seq > to_seq):
        raise SliceError(
            f"the --from bound {from_seq} is above the --to bound "
            f"{to_seq}")
    for kind in kinds or ():
        if kind not in EVENT_KINDS:
            raise SliceError(f"unknown event kind, {kind}")


def slice_tape(tape, from_seq=None, to_seq=None, kinds=None):
    """Cut the selected events out of a valid tape.

    Enforces the selection rules (FT-39), builds the slice
    with its identity carried and its events renumbered
    (FT-40), records the provenance (FT-41), and recomputes
    the outcome and the countable totals (FT-42). The
    source is not mutated. The digest of the source is
    taken before anything is built, so a source that
    cannot be digested raises DigestError here, the way
    compare and rollup refuse an undigestible tape. A tape
    nested too deeply to copy raises SliceError, a named
    refusal, never a bare RecursionError.
    """
    if kinds is not None:
        kinds = list(kinds)
    check_selection(from_seq, to_seq, kinds)
    try:
        return _build_slice(tape, from_seq, to_seq, kinds)
    except RecursionError:
        raise SliceError(
            "the tape is nested too deeply to slice") from None


def _build_slice(tape, from_seq, to_seq, kinds):
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
    result = {
        key: copy.deepcopy(value)
        for key, value in tape.items()
        if key not in ("events", "run", "outcome")
    }
    run = copy.deepcopy(tape["run"])
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
    result["run"] = run
    result["events"] = selected
    result["outcome"] = determine_outcome(selected)
    defects = validate_tape(result)
    if defects:
        raise SliceError(
            "the slice is not a valid tape, " + "; ".join(defects))
    return result
