"""Configuration, read from a .env file only.

There is deliberately no interactive prompt anywhere in this tool: either the
value is in the env file (or the real environment) or the command aborts. That
keeps a bulk send reproducible and safe to re-run.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path

from dotenv import load_dotenv

from wa_invite.errors import ConfigError

PROJECT_ROOT = Path(__file__).resolve().parent.parent


def _find_env_file() -> Path | None:
    """Prefer a .env beside the package, then walk up from the cwd."""
    candidate = PROJECT_ROOT / ".env"
    if candidate.is_file():
        return candidate
    for directory in [Path.cwd(), *Path.cwd().parents]:
        candidate = directory / ".env"
        if candidate.is_file():
            return candidate
    return None


@dataclass(frozen=True)
class Config:
    api_base_url: str
    token: str | None
    email: str | None
    password: str | None
    invite_base_url: str | None
    session_db: Path
    default_country_code: str
    delay_seconds: float
    jitter_seconds: float
    batch_size: int
    batch_pause_seconds: float
    template_file: Path
    ledger_db: Path
    env_file: Path | None

    @property
    def has_api_credentials(self) -> bool:
        return bool(self.token) or bool(self.email and self.password)


def _get(name: str, default: str | None = None) -> str | None:
    value = os.environ.get(name, default)
    if value is None:
        return None
    value = value.strip()
    return value or None


def _get_number(name: str, default: float, *, minimum: float = 0.0) -> float:
    raw = _get(name)
    if raw is None:
        return default
    try:
        value = float(raw)
    except ValueError as exc:
        raise ConfigError(f"{name} must be a number, got {raw!r}") from exc
    if value < minimum:
        raise ConfigError(f"{name} must be >= {minimum}, got {value}")
    return value


def _resolve_path(raw: str, base: Path) -> Path:
    path = Path(raw).expanduser()
    return path if path.is_absolute() else (base / path).resolve()


def load(require_api: bool = True) -> Config:
    """Read .env and the process environment into a validated Config.

    `require_api` is False for commands that never talk to the API (`login`),
    so a fresh checkout can pair WhatsApp before any credentials exist.
    """
    env_file = _find_env_file()
    if env_file is not None:
        load_dotenv(env_file, override=False)

    base = env_file.parent if env_file is not None else PROJECT_ROOT

    token = _get("RINVITE_TOKEN")
    email = _get("RINVITE_EMAIL")
    password = _get("RINVITE_PASSWORD")

    if require_api and not (token or (email and password)):
        where = env_file if env_file is not None else base / ".env"
        raise ConfigError(
            f"no API credentials. Set RINVITE_TOKEN, or both RINVITE_EMAIL and "
            f"RINVITE_PASSWORD, in {where}. Copy .env.example to get started."
        )

    api_base_url = _get("RINVITE_API_BASE_URL")
    if require_api and not api_base_url:
        raise ConfigError("RINVITE_API_BASE_URL is not set")

    country_code = _get("WA_DEFAULT_COUNTRY_CODE", "94") or "94"
    if not country_code.isdigit():
        raise ConfigError(
            f"WA_DEFAULT_COUNTRY_CODE must be digits only, got {country_code!r}"
        )

    batch_size = int(_get_number("WA_BATCH_SIZE", 25, minimum=1))

    return Config(
        api_base_url=(api_base_url or "").rstrip("/"),
        token=token,
        email=email,
        password=password,
        invite_base_url=_get("INVITE_BASE_URL"),
        session_db=_resolve_path(_get("WA_SESSION_DB") or "./session/wa.db", base),
        default_country_code=country_code,
        delay_seconds=_get_number("WA_DELAY_SECONDS", 8.0),
        jitter_seconds=_get_number("WA_JITTER_SECONDS", 4.0),
        batch_size=batch_size,
        batch_pause_seconds=_get_number("WA_BATCH_PAUSE_SECONDS", 120.0),
        template_file=_resolve_path(
            _get("WA_TEMPLATE_FILE") or "./templates/whatsapp.txt", base
        ),
        ledger_db=_resolve_path(_get("WA_LEDGER_DB") or "./ledger.db", base),
        env_file=env_file,
    )
