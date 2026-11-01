"""Tape identity, the canonical form and the digest (FT-26 to FT-28).

A consumer cites a tape by its digest, the SHA-256 of the
tape's canonical form, computed over the parsed tape so any
file that parses to the same tape digests identically. The
canonical form is language portable on purpose, a verifier in
any language that follows FT-26 reaches the same bytes.
"""

import hashlib
import json
import math


class DigestError(ValueError):
    """The tape cannot be canonicalized (FT-28)."""


def _render_number(value):
    """Render a finite float as ECMAScript renders it (FT-26)."""
    if value == 0:
        return "0"
    sign = "-" if value < 0 else ""
    text = repr(abs(value))
    if "e" in text:
        mantissa, _, exponent = text.partition("e")
        exp10 = int(exponent)
    else:
        mantissa, exp10 = text, None
    if "." in mantissa:
        whole, _, fraction = mantissa.partition(".")
    else:
        whole, fraction = mantissa, ""
    digits = (whole + fraction).lstrip("0")
    if exp10 is None:
        point = len(whole) - ((len(whole) + len(fraction)) - len(digits))
        exp10 = point - 1
    digits = digits.rstrip("0") or "0"
    count = len(digits)
    top = exp10 + 1
    if count <= top <= 21:
        return sign + digits + "0" * (top - count)
    if 0 < top <= 21:
        return sign + digits[:top] + "." + digits[top:]
    if -6 < top <= 0:
        return sign + "0." + "0" * (-top) + digits
    head = digits if count == 1 else digits[0] + "." + digits[1:]
    return sign + head + "e" + ("+" if exp10 >= 0 else "-") + str(abs(exp10))


def _render(value, out):
    if value is None:
        out.append("null")
    elif value is True:
        out.append("true")
    elif value is False:
        out.append("false")
    elif isinstance(value, str):
        out.append(json.dumps(value, ensure_ascii=False))
    elif isinstance(value, int):
        out.append(str(value))
    elif isinstance(value, float):
        if not math.isfinite(value):
            raise DigestError("the tape holds a non-finite number")
        out.append(_render_number(value))
    elif isinstance(value, list):
        out.append("[")
        for index, item in enumerate(value):
            if index:
                out.append(",")
            _render(item, out)
        out.append("]")
    elif isinstance(value, dict):
        keys = list(value)
        for key in keys:
            if not isinstance(key, str):
                raise DigestError("the tape holds an object key that is not a string")
        out.append("{")
        for index, key in enumerate(sorted(keys)):
            if index:
                out.append(",")
            out.append(json.dumps(key, ensure_ascii=False))
            out.append(":")
            _render(value[key], out)
        out.append("}")
    else:
        raise DigestError(f"the tape holds a value of an unsupported type, {type(value).__name__}")


def canonical_bytes(tape):
    """The tape's canonical form as UTF-8 bytes (FT-26)."""
    out = []
    try:
        _render(tape, out)
    except RecursionError:
        raise DigestError("the tape is nested beyond the interpreter limit") from None
    return "".join(out).encode("utf-8")


def tape_digest(tape):
    """The tape's digest, lowercase hex SHA-256 (FT-27)."""
    return hashlib.sha256(canonical_bytes(tape)).hexdigest()
