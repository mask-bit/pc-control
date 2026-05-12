"""Controle Spotify real por URI/link e teclas de midia."""

from __future__ import annotations

import re
import urllib.parse

from local_executor import open_url, press_media_key

SPOTIFY_HTTP_RE = re.compile(
    r"^https://open\.spotify\.com/(playlist|album|track|artist)/([A-Za-z0-9]+)(?:[/?#].*)?$",
    re.I,
)
SPOTIFY_URI_RE = re.compile(r"^spotify:(playlist|album|track|artist):[A-Za-z0-9]+$", re.I)


def to_spotify_uri(value: str) -> str | None:
    value = (value or "").strip()
    if SPOTIFY_URI_RE.match(value):
        return value
    match = SPOTIFY_HTTP_RE.match(value)
    if match:
        kind, item_id = match.groups()
        return f"spotify:{kind}:{item_id}"
    return None


def play(value: str) -> None:
    value = (value or "").strip()
    uri = to_spotify_uri(value)
    if uri:
        open_url(uri)
        return
    if value.startswith("http"):
        open_url(value)
        return
    query = urllib.parse.quote_plus(value or "lo-fi focus")
    open_url(f"spotify:search:{query}")


def pause() -> None:
    press_media_key("play_pause")


def next_track() -> None:
    press_media_key("next")


def previous_track() -> None:
    press_media_key("previous")


def volume_up(step: int = 5) -> None:
    press_media_key("volume_up", max(1, round(step / 2)))


def volume_down(step: int = 5) -> None:
    press_media_key("volume_down", max(1, round(step / 2)))
