import pytest

from wa_invite.links import is_insecure, resolve

SERVER_URL = "http://localhost:3000/i/4VHmLjQrcrav"


def test_no_override_passes_through():
    assert resolve(SERVER_URL, None) == SERVER_URL
    assert resolve(SERVER_URL, "") == SERVER_URL


def test_override_replaces_origin_only():
    assert (
        resolve(SERVER_URL, "https://rinvite.ceykod.com")
        == "https://rinvite.ceykod.com/i/4VHmLjQrcrav"
    )


def test_token_case_is_preserved():
    # The 12-char base62 token is case-sensitive.
    out = resolve("http://x/i/aB3dEfGhIjKl", "https://rinvite.link")
    assert out.endswith("/i/aB3dEfGhIjKl")


def test_trailing_slash_and_bare_host():
    assert resolve(SERVER_URL, "https://x.test/") == "https://x.test/i/4VHmLjQrcrav"
    assert resolve(SERVER_URL, "x.test") == "https://x.test/i/4VHmLjQrcrav"


def test_override_with_path_prefix():
    assert (
        resolve(SERVER_URL, "https://x.test/wedding")
        == "https://x.test/wedding/i/4VHmLjQrcrav"
    )


def test_port_is_carried_over():
    assert resolve(SERVER_URL, "http://192.168.1.5:8080") == (
        "http://192.168.1.5:8080/i/4VHmLjQrcrav"
    )


def test_rejects_unusable_override():
    with pytest.raises(ValueError):
        resolve(SERVER_URL, "https://")


def test_is_insecure():
    assert is_insecure("http://x/i/a")
    assert not is_insecure("https://x/i/a")
