"""Login Google opcional e efemero para o PC Control."""

from __future__ import annotations

import base64
import hashlib
import http.server
import json
import secrets
import threading
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass
from typing import Any

from app_paths import SESSION_DIR, clear_ephemeral_session
from local_executor import open_url

GOOGLE_AUTH_URL = "https://accounts.google.com/o/oauth2/v2/auth"
GOOGLE_TOKEN_URL = "https://oauth2.googleapis.com/token"
GOOGLE_USERINFO_URL = "https://www.googleapis.com/oauth2/v3/userinfo"
GOOGLE_REDIRECT_URI = "http://127.0.0.1:43880/google/callback"
GOOGLE_SCOPES = "openid email profile"


@dataclass
class GoogleSession:
    active: bool = False
    email: str = ""
    name: str = ""
    picture: str = ""
    expires_at: float = 0.0
    access_token: str = ""

    @property
    def label(self) -> str:
        if not self.active:
            return "Modo local"
        return self.email or self.name or "Sessao Google ativa"


class GoogleAuthManager:
    def __init__(self) -> None:
        self.session = GoogleSession()

    def status_label(self) -> str:
        if self.session.active and time.time() < self.session.expires_at:
            return f"Sessao ativa: {self.session.label}"
        if self.session.active:
            self.logout()
        return "Modo local"

    def login(self, client_id: str, timeout: int = 120) -> tuple[bool, str]:
        client_id = (client_id or "").strip()
        if not client_id:
            return False, "Informe o Google OAuth Client ID."

        verifier = _code_verifier()
        challenge = _code_challenge(verifier)
        state = secrets.token_urlsafe(18)
        result: dict[str, str] = {}
        event = threading.Event()

        class CallbackHandler(http.server.BaseHTTPRequestHandler):
            def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
                return

            def do_GET(self) -> None:  # noqa: N802
                parsed_path = urllib.parse.urlparse(self.path)
                if parsed_path.path != "/google/callback":
                    self.send_response(404)
                    self.end_headers()
                    event.set()
                    return
                query = urllib.parse.parse_qs(parsed_path.query)
                result["code"] = (query.get("code") or [""])[0]
                result["state"] = (query.get("state") or [""])[0]
                result["error"] = (query.get("error") or [""])[0]
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.end_headers()
                self.wfile.write(
                    b"<html><body style='font-family:Segoe UI;background:#101827;color:#f8fafc'>"
                    b"<h2>PC Control conectado ao Google.</h2><p>Voce ja pode fechar esta janela.</p>"
                    b"</body></html>"
                )
                event.set()

        try:
            server = http.server.ThreadingHTTPServer(("127.0.0.1", 43880), CallbackHandler)
        except OSError as exc:
            return False, f"Porta local do login Google indisponivel: {exc}"
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()

        auth_url = GOOGLE_AUTH_URL + "?" + urllib.parse.urlencode(
            {
                "client_id": client_id,
                "response_type": "code",
                "redirect_uri": GOOGLE_REDIRECT_URI,
                "scope": GOOGLE_SCOPES,
                "state": state,
                "code_challenge": challenge,
                "code_challenge_method": "S256",
                "access_type": "online",
                "prompt": "select_account",
            }
        )
        try:
            open_url(auth_url)
            if not event.wait(timeout):
                return False, "Tempo esgotado aguardando login Google."
            if result.get("error"):
                return False, f"Google recusou login: {result['error']}"
            if result.get("state") != state or not result.get("code"):
                return False, "Retorno do Google invalido."
            token = _exchange_code(client_id, verifier, result["code"])
            profile = _fetch_profile(token["access_token"])
            self.session = GoogleSession(
                active=True,
                email=profile.get("email", ""),
                name=profile.get("name", ""),
                picture=profile.get("picture", ""),
                expires_at=time.time() + int(token.get("expires_in", 3600)),
                access_token=token["access_token"],
            )
            _write_ephemeral_marker(self.session)
            return True, f"Sessao ativa: {self.session.label}"
        finally:
            server.shutdown()
            server.server_close()

    def logout(self) -> None:
        self.session = GoogleSession()
        clear_ephemeral_session()


def _exchange_code(client_id: str, verifier: str, code: str) -> dict[str, Any]:
    body = urllib.parse.urlencode(
        {
            "client_id": client_id,
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": GOOGLE_REDIRECT_URI,
            "code_verifier": verifier,
        }
    ).encode("utf-8")
    req = urllib.request.Request(GOOGLE_TOKEN_URL, data=body, method="POST")
    req.add_header("Content-Type", "application/x-www-form-urlencoded")
    with urllib.request.urlopen(req, timeout=15) as res:
        return json.loads(res.read().decode("utf-8"))


def _fetch_profile(access_token: str) -> dict[str, str]:
    req = urllib.request.Request(
        GOOGLE_USERINFO_URL,
        headers={"Authorization": f"Bearer {access_token}"},
        method="GET",
    )
    with urllib.request.urlopen(req, timeout=15) as res:
        return json.loads(res.read().decode("utf-8"))


def _write_ephemeral_marker(session: GoogleSession) -> None:
    SESSION_DIR.mkdir(parents=True, exist_ok=True)
    marker = {
        "email": session.email,
        "name": session.name,
        "expires_at": session.expires_at,
        "temporary": True,
    }
    (SESSION_DIR / "google_session.json").write_text(json.dumps(marker, indent=2), encoding="utf-8")


def _code_verifier() -> str:
    return base64.urlsafe_b64encode(secrets.token_bytes(48)).decode("ascii").rstrip("=")


def _code_challenge(verifier: str) -> str:
    digest = hashlib.sha256(verifier.encode("ascii")).digest()
    return base64.urlsafe_b64encode(digest).decode("ascii").rstrip("=")
