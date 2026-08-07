from wa_invite.ledger import FAILED, SENT, Ledger

EVENT = "e1"


def test_only_successful_sends_block_a_resend(tmp_path):
    with Ledger(tmp_path / "l.db") as ledger:
        ledger.record(EVENT, "g1", "Ravi", "+94711954412", SENT, message_id="m1")
        ledger.record(EVENT, "g2", "Nimal", "+94771954412", FAILED, detail="offline")

        # A failure must be retried on the next run; a success must not.
        assert ledger.sent_guest_ids(EVENT) == {"g1"}


def test_retry_upgrades_a_failure_to_sent(tmp_path):
    with Ledger(tmp_path / "l.db") as ledger:
        ledger.record(EVENT, "g1", "Ravi", "+94711954412", FAILED, detail="timeout")
        ledger.record(EVENT, "g1", "Ravi", "+94711954412", SENT, message_id="m9")

        assert ledger.sent_guest_ids(EVENT) == {"g1"}
        entries = ledger.entries(EVENT)
        assert len(entries) == 1
        assert entries[0].status == SENT
        assert entries[0].message_id == "m9"


def test_events_are_isolated(tmp_path):
    with Ledger(tmp_path / "l.db") as ledger:
        ledger.record(EVENT, "g1", "Ravi", "+94711954412", SENT)
        ledger.record("e2", "g1", "Ravi", "+94711954412", SENT)

        assert ledger.sent_guest_ids(EVENT) == {"g1"}
        assert len(ledger.entries("e2")) == 1


def test_survives_reopen(tmp_path):
    path = tmp_path / "l.db"
    with Ledger(path) as ledger:
        ledger.record(EVENT, "g1", "Ravi", "+94711954412", SENT)
    with Ledger(path) as reopened:
        assert reopened.sent_guest_ids(EVENT) == {"g1"}
