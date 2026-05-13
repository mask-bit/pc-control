from auth_google import GoogleAuthManager, _code_challenge, _code_verifier


def test_google_auth_starts_in_local_mode():
    manager = GoogleAuthManager()

    assert manager.status_label() == "Modo local"
    assert manager.session.active is False


def test_google_pkce_values_are_urlsafe():
    verifier = _code_verifier()
    challenge = _code_challenge(verifier)

    assert len(verifier) >= 43
    assert "=" not in verifier
    assert "=" not in challenge
    assert challenge
