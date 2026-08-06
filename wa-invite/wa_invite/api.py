"""Read-only client for the rinvite HTTP API.

Deliberately does not call POST /events/{id}/invites/send — that is the
server's metered Twilio path, which this tool exists to replace.
"""

from __future__ import annotations

from typing import Any

import httpx

from wa_invite.config import Config
from wa_invite.errors import ApiError

EINVITE = "einvite"

# Statuses the server uses that would otherwise read as a generic failure.
STATUS_HINTS = {
    401: "invalid or expired credentials",
    402: "plan limit reached — upgrade the rinvite account",
    403: "email address is not verified",
    423: "account is pending approval by the rinvite owner",
}


class RinviteApi:
    def __init__(self, config: Config, timeout: float = 30.0):
        self._config = config
        self._token = config.token
        self._client = httpx.Client(base_url=config.api_base_url, timeout=timeout)
        self._relogin_attempted = False

    def close(self) -> None:
        self._client.close()

    def __enter__(self) -> "RinviteApi":
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.close()

    # --- auth -----------------------------------------------------------

    def _ensure_token(self) -> str:
        if self._token:
            return self._token
        return self._login()

    def _login(self) -> str:
        cfg = self._config
        if not (cfg.email and cfg.password):
            raise ApiError(
                "no RINVITE_TOKEN and no RINVITE_EMAIL/RINVITE_PASSWORD to log in with"
            )
        payload = self._request(
            "POST",
            "/auth/login",
            json={"email": cfg.email, "password": cfg.password},
            authenticated=False,
        )
        token = payload.get("token")
        if not token:
            raise ApiError("login succeeded but returned no token")
        self._token = token
        return token

    # --- transport ------------------------------------------------------

    def _request(
        self,
        method: str,
        path: str,
        *,
        json: Any = None,
        authenticated: bool = True,
    ) -> Any:
        headers = {}
        if authenticated:
            # The server's prefix check is case-sensitive.
            headers["Authorization"] = f"Bearer {self._ensure_token()}"

        try:
            response = self._client.request(method, path, json=json, headers=headers)
        except httpx.RequestError as exc:
            raise ApiError(
                f"cannot reach {self._config.api_base_url}{path}: {exc}"
            ) from exc

        if response.status_code == 401 and authenticated and not self._relogin_attempted:
            # A cached token can expire mid-run (24h TTL); try exactly once more.
            if self._config.email and self._config.password:
                self._relogin_attempted = True
                self._token = None
                return self._request(method, path, json=json, authenticated=True)

        if response.is_success:
            if response.status_code == 204 or not response.content:
                return None
            return response.json()

        raise ApiError(self._describe(response, path))

    def _describe(self, response: httpx.Response, path: str) -> str:
        detail = ""
        try:
            body = response.json()
            if isinstance(body, dict) and "error" in body:
                detail = str(body["error"])
        except ValueError:
            detail = response.text.strip()[:200]

        hint = STATUS_HINTS.get(response.status_code)
        parts = [f"{response.status_code} from {path}"]
        if detail:
            parts.append(detail)
        if hint and hint not in detail:
            parts.append(f"({hint})")
        return ": ".join(parts[:2]) + (f" {parts[2]}" if len(parts) > 2 else "")

    # --- endpoints ------------------------------------------------------

    def list_events(self) -> list[dict[str, Any]]:
        return self._request("GET", "/events") or []

    def get_event(self, event_id: str) -> dict[str, Any]:
        return self._request("GET", f"/events/{event_id}")

    def list_guests(self, event_id: str) -> list[dict[str, Any]]:
        return self._request("GET", f"/events/{event_id}/guests") or []

    def list_einvite_guests(self, event_id: str) -> list[dict[str, Any]]:
        """Filtered client-side: the guests endpoint has no ?channel= param."""
        return [g for g in self.list_guests(event_id) if g.get("channel") == EINVITE]
