import pytest

from wa_invite.config import load
from wa_invite.errors import ConfigError

REQUIRED = (
    "RINVITE_TOKEN",
    "RINVITE_EMAIL",
    "RINVITE_PASSWORD",
    "RINVITE_API_BASE_URL",
    "INVITE_BASE_URL",
    "WA_DEFAULT_COUNTRY_CODE",
    "WA_BATCH_SIZE",
    "WA_DELAY_SECONDS",
)


@pytest.fixture
def clean_env(monkeypatch, tmp_path):
    for name in REQUIRED:
        monkeypatch.delenv(name, raising=False)
    # Run from an empty directory so a developer's real .env is never picked up.
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr("wa_invite.config._find_env_file", lambda: None)


def test_aborts_without_credentials(clean_env):
    with pytest.raises(ConfigError, match="RINVITE_TOKEN"):
        load()


def test_login_does_not_need_credentials(clean_env):
    # `wa-invite login` must work on a fresh checkout, before any API setup.
    assert load(require_api=False).token is None


def test_token_alone_is_enough(clean_env, monkeypatch):
    monkeypatch.setenv("RINVITE_TOKEN", "jwt")
    monkeypatch.setenv("RINVITE_API_BASE_URL", "https://api.rinvite.link/")
    config = load()
    assert config.has_api_credentials
    assert config.api_base_url == "https://api.rinvite.link"  # trailing slash stripped


def test_email_and_password_are_enough(clean_env, monkeypatch):
    monkeypatch.setenv("RINVITE_EMAIL", "a@b.c")
    monkeypatch.setenv("RINVITE_PASSWORD", "pw")
    monkeypatch.setenv("RINVITE_API_BASE_URL", "https://x")
    assert load().has_api_credentials


def test_email_without_password_is_not_enough(clean_env, monkeypatch):
    monkeypatch.setenv("RINVITE_EMAIL", "a@b.c")
    monkeypatch.setenv("RINVITE_API_BASE_URL", "https://x")
    with pytest.raises(ConfigError):
        load()


def test_blank_values_count_as_missing(clean_env, monkeypatch):
    monkeypatch.setenv("RINVITE_TOKEN", "   ")
    monkeypatch.setenv("RINVITE_API_BASE_URL", "https://x")
    with pytest.raises(ConfigError):
        load()


def test_rejects_non_numeric_country_code(clean_env, monkeypatch):
    monkeypatch.setenv("WA_DEFAULT_COUNTRY_CODE", "+94")
    with pytest.raises(ConfigError, match="WA_DEFAULT_COUNTRY_CODE"):
        load(require_api=False)


def test_rejects_bad_numbers(clean_env, monkeypatch):
    monkeypatch.setenv("WA_DELAY_SECONDS", "soon")
    with pytest.raises(ConfigError, match="WA_DELAY_SECONDS"):
        load(require_api=False)


def test_rejects_zero_batch_size(clean_env, monkeypatch):
    monkeypatch.setenv("WA_BATCH_SIZE", "0")
    with pytest.raises(ConfigError, match="WA_BATCH_SIZE"):
        load(require_api=False)


def test_defaults(clean_env):
    config = load(require_api=False)
    assert config.default_country_code == "94"
    assert config.batch_size == 25
    assert config.invite_base_url is None
