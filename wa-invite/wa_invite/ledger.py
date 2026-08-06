"""Local record of what has already been sent.

The rinvite API has no "sent" flag on a guest, so without this a second `send`
run would message everyone again. Writing each result immediately also means a
crashed or interrupted run resumes exactly where it stopped.
"""

from __future__ import annotations

import sqlite3
from contextlib import closing
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

SENT = "sent"
FAILED = "failed"
SKIPPED = "skipped"

SCHEMA = """
CREATE TABLE IF NOT EXISTS sends (
  event_id   TEXT NOT NULL,
  guest_id   TEXT NOT NULL,
  guest_name TEXT NOT NULL,
  phone      TEXT NOT NULL,
  status     TEXT NOT NULL,
  detail     TEXT,
  message_id TEXT,
  sent_at    TEXT NOT NULL,
  PRIMARY KEY (event_id, guest_id)
);
"""


@dataclass(frozen=True)
class Entry:
    guest_id: str
    guest_name: str
    phone: str
    status: str
    detail: str | None
    message_id: str | None
    sent_at: str


class Ledger:
    def __init__(self, path: Path):
        self.path = path
        path.parent.mkdir(parents=True, exist_ok=True)
        self._conn = sqlite3.connect(path)
        self._conn.row_factory = sqlite3.Row
        with closing(self._conn.cursor()) as cur:
            cur.executescript(SCHEMA)
        self._conn.commit()

    def close(self) -> None:
        self._conn.close()

    def __enter__(self) -> "Ledger":
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.close()

    def record(
        self,
        event_id: str,
        guest_id: str,
        guest_name: str,
        phone: str,
        status: str,
        detail: str | None = None,
        message_id: str | None = None,
    ) -> None:
        with closing(self._conn.cursor()) as cur:
            cur.execute(
                """
                INSERT INTO sends
                  (event_id, guest_id, guest_name, phone, status, detail, message_id, sent_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(event_id, guest_id) DO UPDATE SET
                  guest_name = excluded.guest_name,
                  phone      = excluded.phone,
                  status     = excluded.status,
                  detail     = excluded.detail,
                  message_id = excluded.message_id,
                  sent_at    = excluded.sent_at
                """,
                (
                    event_id,
                    guest_id,
                    guest_name,
                    phone,
                    status,
                    detail,
                    message_id,
                    datetime.now(timezone.utc).isoformat(timespec="seconds"),
                ),
            )
        self._conn.commit()

    def sent_guest_ids(self, event_id: str) -> set[str]:
        """Only successful sends block a re-send; failures are retried."""
        with closing(self._conn.cursor()) as cur:
            cur.execute(
                "SELECT guest_id FROM sends WHERE event_id = ? AND status = ?",
                (event_id, SENT),
            )
            return {row["guest_id"] for row in cur.fetchall()}

    def entries(self, event_id: str) -> list[Entry]:
        with closing(self._conn.cursor()) as cur:
            cur.execute(
                "SELECT * FROM sends WHERE event_id = ? ORDER BY sent_at",
                (event_id,),
            )
            return [
                Entry(
                    guest_id=row["guest_id"],
                    guest_name=row["guest_name"],
                    phone=row["phone"],
                    status=row["status"],
                    detail=row["detail"],
                    message_id=row["message_id"],
                    sent_at=row["sent_at"],
                )
                for row in cur.fetchall()
            ]
