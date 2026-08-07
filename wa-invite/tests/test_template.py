"""The fixture mirrors the Rust renderer's test in
src/adapter/outbound/message/mod.rs so the two stay interchangeable.
"""

from datetime import time

import pytest

from wa_invite.errors import TemplateError
from wa_invite.template import fmt_time, ordered_names, ordinal, render

EVENT = {
    "bride_name": "Hansika",
    "bride_family_name": "J",
    "groom_name": "Chirath",
    "groom_family_name": "N",
    "precedence": "bride",
    "event_date": "2026-09-25",
    "start_time": "10:00:00",
    "end_time": "15:00:00",
    "hall_name": "Kings Ballroom",
    "venue_name": "Kandy",
    "rsvp_by": "2026-08-20",
}

GUEST = {
    "id": "g1",
    "name": "Ravi",
    "channel": "einvite",
    "phone": "+94711954412",
    "max_party_size": 2,
    "rsvp_status": "pending",
}

URL = "https://rinvite.ceykod.com/i/4VHmLjQrcrav"


@pytest.mark.parametrize(
    ("day", "expected"),
    [
        (1, "1st"), (2, "2nd"), (3, "3rd"), (4, "4th"),
        (11, "11th"), (12, "12th"), (13, "13th"),
        (20, "20th"), (21, "21st"), (22, "22nd"), (23, "23rd"), (31, "31st"),
    ],
)
def test_ordinal(day, expected):
    assert ordinal(day) == expected


@pytest.mark.parametrize(
    ("value", "expected"),
    [
        (time(0, 0), "12:00 AM"),
        (time(0, 30), "12:30 AM"),
        (time(9, 5), "9:05 AM"),
        (time(11, 59), "11:59 AM"),
        (time(12, 0), "12:00 PM"),
        (time(13, 0), "1:00 PM"),
        (time(23, 45), "11:45 PM"),
    ],
)
def test_fmt_time(value, expected):
    assert fmt_time(value) == expected


def test_ordered_names_follows_precedence():
    assert ordered_names(EVENT) == ("Hansika", "Chirath")
    assert ordered_names({**EVENT, "precedence": "groom"}) == ("Chirath", "Hansika")


def test_render_fills_every_placeholder():
    body = render(
        "{couple} | {guest_name} | {date} | {time} | {hall}, {venue} | "
        "{rsvp_by} | {invite_url} | {max_party_size}",
        EVENT,
        GUEST,
        URL,
    )
    assert body == (
        "Hansika & Chirath | Ravi | Friday, 25 September 2026 | "
        "10:00 AM to 3:00 PM | Kings Ballroom, Kandy | 20th August | "
        f"{URL} | 2"
    )


def test_render_does_not_html_escape():
    # WhatsApp bodies are plain text; the server escapes only for email HTML.
    body = render("{guest_name}", EVENT, {**GUEST, "name": "Ravi <b> & co"}, URL)
    assert body == "Ravi <b> & co"


def test_unknown_placeholder_is_fatal():
    with pytest.raises(TemplateError, match="venu_name"):
        render("Come to {venu_name}", EVENT, GUEST, URL)


def test_shipped_template_renders(tmp_path):
    from pathlib import Path

    from wa_invite.template import load_template

    shipped = Path(__file__).parent.parent / "templates" / "whatsapp.txt"
    body = render(load_template(shipped), EVENT, GUEST, URL)
    assert "Hansika & Chirath" in body
    assert URL in body
    assert "{" not in body
