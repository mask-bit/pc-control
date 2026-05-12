"""Controle Spotify por URI local e, quando configurado, Spotify Web API PKCE."""

from __future__ import annotations

import base64
import hashlib
import http.server
import json
import re
import secrets
import threading
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass
from typing import Any

from local_executor import open_url, press_media_key
from secrets_store import get_secret, set_secret

SPOTIFY_HTTP_RE = re.compile(
    r"^https://open\.spotify\.com/(playlist|album|track|artist)/([A-Za-z0-9]+)(?:[/?#].*)?$",
    re.I,
)
SPOTIFY_URI_RE = re.compile(r"^spotify:(playlist|album|track|artist):[A-Za-z0-9]+$", re.I)
TOKEN_URL = "https://accounts.spotify.com/api/token"
API_BASE = "https://api.spotify.com/v1"
DEFAULT_SCOPES = (
    "user-read-playback-state user-read-currently-playing user-modify-playback-state "
    "playlist-read-private playlist-read-collaborative"
)


@dataclass
class SpotifyStatus:
    connected: bool
    message: str
    current_track: str = ""
    artist: str = ""
    device: str = ""
    is_playing: bool = False
    cover_url: str = ""


def to_spotify_uri(value: str) -> str | None:
    value = (value or "").strip()
    if SPOTIFY_URI_RE.match(value):
        return value
    match = SPOTIFY_HTTP_RE.match(value)
    if match:
        kind, item_id = match.groups()
        return f"spotify:{kind}:{item_id}"
    return None


def authorize_pkce(config: dict[str, Any], timeout: int = 120) -> tuple[bool, str]:
    spotify_cfg = config.get("spotify_web", {})
    client_id = (spotify_cfg.get("client_id") or "").strip()
    redirect_uri = spotify_cfg.get("redirect_uri", "http://127.0.0.1:43879/callback")
    scopes = spotify_cfg.get("scopes", DEFAULT_SCOPES)
    if not client_id:
        return False, "Informe o Client ID do Spotify nas integracoes."

    parsed = urllib.parse.urlparse(redirect_uri)
    verifier = _code_verifier()
    challenge = _code_challenge(verifier)
    state = secrets.token_urlsafe(18)
    result: dict[str, str] = {}
    event = threading.Event()

    class CallbackHandler(http.server.BaseHTTPRequestHandler):
        def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
            return

        def do_GET(self) -> None:  # noqa: N802
            query = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
            result["code"] = (query.get("code") or [""])[0]
            result["state"] = (query.get("state") or [""])[0]
            result["error"] = (query.get("error") or [""])[0]
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.end_headers()
            self.wfile.write(
                b"<html><body style='font-family:Segoe UI;background:#101827;color:#f8fafc'>"
                b"<h2>Spotify conectado ao Jarvis.</h2><p>Voce ja pode fechar esta janela.</p>"
                b"</body></html>"
            )
            event.set()

    server = http.server.ThreadingHTTPServer((parsed.hostname or "127.0.0.1", parsed.port or 43879), CallbackHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()

    auth_url = "https://accounts.spotify.com/authorize?" + urllib.parse.urlencode(
        {
            "client_id": client_id,
            "response_type": "code",
            "redirect_uri": redirect_uri,
            "code_challenge_method": "S256",
            "code_challenge": challenge,
            "state": state,
            "scope": scopes,
        }
    )
    open_url(auth_url)
    try:
        if not event.wait(timeout):
            return False, "Tempo esgotado aguardando login do Spotify."
        if result.get("error"):
            return False, f"Spotify recusou login: {result['error']}"
        if result.get("state") != state or not result.get("code"):
            return False, "Retorno do Spotify invalido."
        token = _exchange_code(client_id, redirect_uri, verifier, result["code"])
        _store_token(token)
        return True, "Spotify conectado com sucesso."
    finally:
        server.shutdown()
        server.server_close()


def is_connected(config: dict[str, Any] | None = None) -> bool:
    return bool(get_secret("spotify_refresh_token") and _client_id(config or {}))


def get_status(config: dict[str, Any]) -> SpotifyStatus:
    if not is_connected(config):
        return SpotifyStatus(False, "Spotify Web API nao conectado.")
    try:
        payload = api_request("GET", "/me/player/currently-playing", config, allow_empty=True)
    except Exception as exc:
        return SpotifyStatus(False, f"Falha ao consultar Spotify: {exc}")
    if not payload:
        return SpotifyStatus(True, "Nenhuma faixa tocando agora.")
    item = payload.get("item") or {}
    artists = ", ".join(artist.get("name", "") for artist in item.get("artists", []) if artist.get("name"))
    album = item.get("album") or {}
    images = album.get("images") or []
    device = (payload.get("device") or {}).get("name", "")
    return SpotifyStatus(
        True,
        "Spotify conectado.",
        current_track=item.get("name", ""),
        artist=artists,
        device=device,
        is_playing=bool(payload.get("is_playing")),
        cover_url=(images[0].get("url") if images else ""),
    )


def play(value: str, config: dict[str, Any] | None = None) -> None:
    config = config or {}
    value = (value or "").strip()
    uri = to_spotify_uri(value)
    if is_connected(config):
        try:
            uri = uri or _search_best_uri(value or config.get("spotify", {}).get("default_query", "lo-fi focus"), config)
            _play_uri(uri, config)
            return
        except Exception:
            pass
    if uri:
        open_url(uri)
        return
    if value.startswith("http"):
        open_url(value)
        return
    query = urllib.parse.quote_plus(value or "lo-fi focus")
    open_url(f"spotify:search:{query}")


def pause(config: dict[str, Any] | None = None) -> None:
    if is_connected(config or {}):
        try:
            api_request("PUT", "/me/player/pause", config or {}, allow_empty=True)
            return
        except Exception:
            pass
    press_media_key("play_pause")


def next_track(config: dict[str, Any] | None = None) -> None:
    if is_connected(config or {}):
        try:
            api_request("POST", "/me/player/next", config or {}, allow_empty=True)
            return
        except Exception:
            pass
    press_media_key("next")


def previous_track(config: dict[str, Any] | None = None) -> None:
    if is_connected(config or {}):
        try:
            api_request("POST", "/me/player/previous", config or {}, allow_empty=True)
            return
        except Exception:
            pass
    press_media_key("previous")


def set_volume(level: int, config: dict[str, Any] | None = None) -> None:
    level = max(0, min(100, int(level)))
    if is_connected(config or {}):
        try:
            api_request("PUT", f"/me/player/volume?volume_percent={level}", config or {}, allow_empty=True)
            return
        except Exception:
            pass


def volume_up(step: int = 5) -> None:
    press_media_key("volume_up", max(1, round(step / 2)))


def volume_down(step: int = 5) -> None:
    press_media_key("volume_down", max(1, round(step / 2)))


def api_request(
    method: str,
    path: str,
    config: dict[str, Any],
    data: dict[str, Any] | None = None,
    allow_empty: bool = False,
) -> dict[str, Any]:
    token = _access_token(config)
    body = json.dumps(data).encode("utf-8") if data is not None else None
    req = urllib.request.Request(
        API_BASE + path,
        data=body,
        headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json"},
        method=method,
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as res:
            raw = res.read().decode("utf-8")
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as exc:
        if exc.code == 401:
            _refresh_token(config)
            return api_request(method, path, config, data, allow_empty)
        if allow_empty and exc.code in {202, 204}:
            return {}
        raise


def _client_id(config: dict[str, Any]) -> str:
    return (config.get("spotify_web", {}) or {}).get("client_id", "").strip()


def _access_token(config: dict[str, Any]) -> str:
    token = get_secret("spotify_access_token")
    expires_at = float(get_secret("spotify_expires_at") or "0")
    if token and time.time() < expires_at - 60:
        return token
    return _refresh_token(config)


def _refresh_token(config: dict[str, Any]) -> str:
    client_id = _client_id(config)
    refresh = get_secret("spotify_refresh_token")
    if not client_id or not refresh:
        raise RuntimeError("Spotify nao conectado.")
    body = urllib.parse.urlencode(
        {
            "grant_type": "refresh_token",
            "refresh_token": refresh,
            "client_id": client_id,
        }
    ).encode("utf-8")
    req = urllib.request.Request(TOKEN_URL, data=body, method="POST")
    req.add_header("Content-Type", "application/x-www-form-urlencoded")
    with urllib.request.urlopen(req, timeout=15) as res:
        token = json.loads(res.read().decode("utf-8"))
    _store_token(token)
    return token["access_token"]


def _exchange_code(client_id: str, redirect_uri: str, verifier: str, code: str) -> dict[str, Any]:
    body = urllib.parse.urlencode(
        {
            "client_id": client_id,
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": redirect_uri,
            "code_verifier": verifier,
        }
    ).encode("utf-8")
    req = urllib.request.Request(TOKEN_URL, data=body, method="POST")
    req.add_header("Content-Type", "application/x-www-form-urlencoded")
    with urllib.request.urlopen(req, timeout=15) as res:
        return json.loads(res.read().decode("utf-8"))


def _store_token(token: dict[str, Any]) -> None:
    if token.get("access_token"):
        set_secret("spotify_access_token", str(token["access_token"]))
    if token.get("refresh_token"):
        set_secret("spotify_refresh_token", str(token["refresh_token"]))
    if token.get("expires_in"):
        set_secret("spotify_expires_at", str(time.time() + int(token["expires_in"])))


def _search_best_uri(query: str, config: dict[str, Any]) -> str:
    q = urllib.parse.quote(query)
    payload = api_request("GET", f"/search?q={q}&type=playlist,track,album,artist&limit=1", config)
    for group in ("playlists", "tracks", "albums", "artists"):
        items = (payload.get(group) or {}).get("items") or []
        if items:
            return items[0]["uri"]
    raise RuntimeError("Nao encontrei esse conteudo no Spotify.")


def _play_uri(uri: str, config: dict[str, Any]) -> None:
    if ":track:" in uri:
        api_request("PUT", "/me/player/play", config, {"uris": [uri]}, allow_empty=True)
    else:
        api_request("PUT", "/me/player/play", config, {"context_uri": uri}, allow_empty=True)


def _code_verifier() -> str:
    return base64.urlsafe_b64encode(secrets.token_bytes(48)).decode("ascii").rstrip("=")


def _code_challenge(verifier: str) -> str:
    digest = hashlib.sha256(verifier.encode("ascii")).digest()
    return base64.urlsafe_b64encode(digest).decode("ascii").rstrip("=")
