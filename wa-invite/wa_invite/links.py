"""Invite-link resolution.

The server builds each link as `{INVITE_BASE_URL}/i/{token}` (see
`src/domain/guest.rs`), falling back to PUBLIC_BASE_URL and then to
http://localhost:3000. Behind a reverse proxy that can be an origin guests
cannot reach, so we let the operator override the origin locally while keeping
the path exactly as issued — the 12-char base62 token is case-sensitive.
"""

from __future__ import annotations

from urllib.parse import urlparse, urlunparse


def resolve(invite_url: str, override: str | None) -> str:
    """Swap the origin of `invite_url` for `override`, preserving the path."""
    if not override:
        return invite_url

    target = urlparse(override if "//" in override else f"https://{override}")
    if not target.netloc:
        raise ValueError(f"INVITE_BASE_URL is not a usable origin: {override!r}")

    original = urlparse(invite_url)
    # An override may carry a path prefix (e.g. https://host/invites).
    prefix = target.path.rstrip("/")
    path = f"{prefix}{original.path}" if prefix else original.path

    return urlunparse(
        (
            target.scheme or "https",
            target.netloc,
            path,
            original.params,
            original.query,
            original.fragment,
        )
    )


def is_insecure(url: str) -> bool:
    """True for links guests would be asked to open over plain HTTP."""
    return urlparse(url).scheme != "https"
