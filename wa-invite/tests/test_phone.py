import pytest

from wa_invite.phone import PhoneError, normalize

LK = "94"


@pytest.mark.parametrize(
    ("raw", "expected"),
    [
        ("+94 71 195 4412", "94711954412"),
        ("+94711954412", "94711954412"),
        ("94711954412", "94711954412"),
        ("071 195 4412", "94711954412"),
        ("0711954412", "94711954412"),
        ("711954412", "94711954412"),
        ("0094711954412", "94711954412"),
        ("00 94 71-195-4412", "94711954412"),
        ("(071) 195 4412", "94711954412"),
        ("+1 415 555 0132", "14155550132"),
    ],
)
def test_normalize(raw, expected):
    assert normalize(raw, LK) == expected


def test_explicit_plus_is_not_re_prefixed():
    # A UK number must not become 9444...
    assert normalize("+44 20 7946 0958", LK) == "442079460958"


@pytest.mark.parametrize("raw", [None, "", "   ", "abc", "12345", "0" * 20])
def test_rejects_unusable(raw):
    with pytest.raises(PhoneError):
        normalize(raw, LK)


def test_error_message_mentions_the_input():
    with pytest.raises(PhoneError, match="1234"):
        normalize("1234", LK)
