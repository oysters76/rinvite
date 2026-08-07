"""Exceptions that `cli.main` turns into a one-line message and an exit code."""


class WaInviteError(Exception):
    """Base for every error we expect and can explain to the user."""

    exit_code = 1


class ConfigError(WaInviteError):
    """A required setting is missing or malformed in .env."""

    exit_code = 2


class ApiError(WaInviteError):
    """The rinvite API rejected a request or was unreachable."""

    exit_code = 3


class WhatsAppError(WaInviteError):
    """The local WhatsApp client could not connect or send."""

    exit_code = 4


class TemplateError(WaInviteError):
    """The message template is unusable — e.g. an unresolved placeholder."""

    exit_code = 5
