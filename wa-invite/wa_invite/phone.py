"""Phone-number normalisation to E.164 digits.

The API validates a guest phone only as 1–32 characters
(`src/domain/validation.rs`), so real data arrives as anything from
"+94 71 195 4412" to "071 195 4412" to "0094-711954412".
"""

from __future__ import annotations

import re

MIN_DIGITS = 8
MAX_DIGITS = 15


class PhoneError(ValueError):
    """The value cannot be turned into a plausible E.164 number."""


def normalize(raw: str | None, default_country_code: str) -> str:
    """Return bare E.164 digits (no leading +) for `raw`.

    Raises PhoneError with a reason suitable for the skipped-guests table.
    """
    if raw is None or not raw.strip():
        raise PhoneError("no phone number on file")

    value = raw.strip()
    explicit_intl = value.startswith("+")
    digits = re.sub(r"\D", "", value)

    if not digits:
        raise PhoneError(f"no digits in {raw!r}")

    if digits.startswith("00"):
        digits = digits[2:]
        explicit_intl = True

    if not explicit_intl:
        if digits.startswith("0"):
            # National trunk prefix: 071… -> 9471…
            digits = default_country_code + digits[1:]
        elif not digits.startswith(default_country_code):
            digits = default_country_code + digits

    if len(digits) < MIN_DIGITS:
        raise PhoneError(f"too short after normalising: {raw!r} -> +{digits}")
    if len(digits) > MAX_DIGITS:
        raise PhoneError(f"too long after normalising: {raw!r} -> +{digits}")

    return digits


def display(digits: str) -> str:
    return f"+{digits}"
