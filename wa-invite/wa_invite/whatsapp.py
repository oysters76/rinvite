"""Thin wrapper around neonize (Python bindings over Go's whatsmeow).

neonize is event-driven and `connect()` blocks until the Go side shuts down, so
we run it on a daemon thread and hand control back to the caller once the
ConnectedEv arrives. That lets the CLI stay an ordinary top-to-bottom script
instead of a long-lived bot.

The client's `name` doubles as the SQLite session path, which is where the
paired-device credentials live — pair once, reuse forever.
"""

from __future__ import annotations

import io
import threading
import time
from dataclasses import dataclass
from pathlib import Path

from wa_invite.errors import WhatsAppError

CONNECT_TIMEOUT = 45.0
PAIR_TIMEOUT = 180.0

# neonize runs the blocking Go call on a thread it creates with daemon=False,
# and that call does not return even after disconnect()/stop(). Once a session
# has been opened the interpreter can therefore never shut down on its own, so
# the CLI has to bypass normal interpreter exit. See `hard_exit_required`.
_SESSION_OPENED = False


def hard_exit_required() -> bool:
    """True once a WhatsApp session has been opened in this process."""
    return _SESSION_OPENED


@dataclass(frozen=True)
class SentMessage:
    message_id: str
    timestamp: int


def render_qr(data: str) -> str:
    """ASCII QR for the terminal."""
    import qrcode

    code = qrcode.QRCode(border=1)
    code.add_data(data)
    code.make(fit=True)
    buffer = io.StringIO()
    code.print_ascii(out=buffer, invert=True)
    return buffer.getvalue()


class WhatsAppSession:
    """A connected (or connectable) local WhatsApp client."""

    def __init__(self, session_db: Path):
        self.session_db = session_db
        session_db.parent.mkdir(parents=True, exist_ok=True)

        # Imported lazily: neonize loads a Go shared library and libmagic at
        # import time, so commands that never touch WhatsApp stay fast and
        # keep working on a machine without libmagic installed.
        try:
            from neonize.client import NewClient
        except ImportError as exc:
            raise WhatsAppError(
                f"cannot import neonize ({exc}). On macOS this usually means "
                f"libmagic is missing — run: brew install libmagic"
            ) from exc

        # neonize uses the client name as the SQLite session filename.
        self._client = NewClient(str(session_db))
        global _SESSION_OPENED
        _SESSION_OPENED = True
        self._connected = threading.Event()
        self._logged_out = threading.Event()
        self._qr_seen = threading.Event()
        self._thread: threading.Thread | None = None
        self._pair_error: str | None = None
        self._on_qr = None
        self._closed = False

    # --- lifecycle ------------------------------------------------------

    def _register(self) -> None:
        from neonize.events import ConnectedEv, LoggedOutEv, PairStatusEv

        @self._client.event(ConnectedEv)
        def _on_connected(_client, _event):  # noqa: ANN001
            self._connected.set()

        @self._client.event(LoggedOutEv)
        def _on_logged_out(_client, event):  # noqa: ANN001
            self._pair_error = f"logged out (reason {event.Reason})"
            self._logged_out.set()
            self._connected.set()

        @self._client.event(PairStatusEv)
        def _on_pair(_client, event):  # noqa: ANN001
            if event.Error:
                self._pair_error = event.Error

        @self._client.qr
        def _on_qr(_client, qr: bytes):  # noqa: ANN001
            self._qr_seen.set()
            if self._on_qr is not None:
                self._on_qr(qr.decode("utf-8", "replace"))

    def _spawn(self) -> None:
        self._register()
        self._thread = threading.Thread(
            target=self._client.connect, name="neonize-connect", daemon=True
        )
        self._thread.start()

    def connect(self, timeout: float = CONNECT_TIMEOUT) -> None:
        """Connect using the stored session. Fails if the device is unpaired.

        A QR prompt here means there is no usable session, which during a bulk
        send should be a clear error rather than a QR code appearing halfway
        through the guest list.
        """
        self._spawn()

        deadline = time.monotonic() + timeout
        while not self._connected.wait(0.25):
            if self._qr_seen.is_set():
                self.close()
                raise WhatsAppError(
                    "this WhatsApp session is not paired — run `wa-invite login` first"
                )
            if time.monotonic() >= deadline:
                self.close()
                raise WhatsAppError(
                    f"timed out connecting to WhatsApp using {self.session_db}"
                )

        if self._logged_out.is_set():
            self.close()
            raise WhatsAppError(
                f"WhatsApp session is no longer valid ({self._pair_error}) — "
                f"run `wa-invite login` to pair again"
            )

    def pair(self, on_qr, timeout: float = PAIR_TIMEOUT) -> str:
        """Pair this machine with a phone. Returns the linked number."""
        self._on_qr = on_qr
        self._spawn()

        if not self._connected.wait(timeout):
            self.close()
            raise WhatsAppError(
                f"pairing timed out after {timeout:.0f}s — no QR code was scanned"
            )
        if self._logged_out.is_set() or self._pair_error:
            self.close()
            raise WhatsAppError(f"pairing failed: {self._pair_error}")

        return self.own_number() or "unknown number"

    def own_number(self) -> str | None:
        me = getattr(self._client, "me", None)
        jid = getattr(me, "JID", None) if me is not None else None
        user = getattr(jid, "User", None)
        return f"+{user}" if user else None

    def close(self) -> None:
        """Shut the Go side down so the process can actually exit.

        neonize runs the blocking Go call on a *non-daemon* thread, and that
        call only returns once the Go context is cancelled. `disconnect()`
        merely closes the websocket, so `stop()` is what we need here —
        without it the interpreter hangs at exit waiting on that thread.
        """
        if self._closed:
            return
        self._closed = True
        for shutdown in (self._client.disconnect, self._client.stop):
            try:
                shutdown()
            except Exception:  # noqa: BLE001 - teardown must never mask a real error
                pass
        if self._thread is not None:
            self._thread.join(timeout=10.0)

    def __enter__(self) -> "WhatsAppSession":
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.close()

    # --- messaging ------------------------------------------------------

    def registered_numbers(self, digits: list[str]) -> dict[str, bool]:
        """Ask WhatsApp which of these E.164 numbers have accounts.

        Catches landlines, typos and a mis-guessed country code before we send
        anything. Returns a map keyed by the digits passed in.
        """
        if not digits:
            return {}
        try:
            responses = self._client.is_on_whatsapp(*(f"+{d}" for d in digits))
        except Exception as exc:  # noqa: BLE001 - surfaced as a WhatsAppError
            raise WhatsAppError(f"could not verify numbers on WhatsApp: {exc}") from exc

        result = {d: False for d in digits}
        for response in responses:
            key = response.Query.lstrip("+")
            if key in result:
                result[key] = bool(response.IsIn)
            elif response.JID.User in result:
                result[response.JID.User] = bool(response.IsIn)
        return result

    def send_text(self, digits: str, body: str) -> SentMessage:
        from neonize.utils import build_jid

        try:
            response = self._client.send_message(
                build_jid(digits), body, link_preview=True
            )
        except Exception as exc:  # noqa: BLE001 - surfaced as a WhatsAppError
            raise WhatsAppError(str(exc)) from exc
        return SentMessage(message_id=response.ID, timestamp=response.Timestamp)
