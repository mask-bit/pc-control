"""Identidade e caminhos do aplicativo PC Control."""

from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

APP_NAME = "PC Control"
APP_ID = "PCControl"
APP_VERSION = "0.2.0"
CONFIG_FILENAME = "assistant_config.json"
LOG_FILENAME = "assistant_logs.jsonl"


def is_frozen() -> bool:
    return bool(getattr(sys, "frozen", False))


def app_root() -> Path:
    if is_frozen():
        return Path(sys.executable).resolve().parent
    return Path(__file__).resolve().parent


def bundled_root() -> Path:
    return Path(getattr(sys, "_MEIPASS", app_root())).resolve()


def appdata_root() -> Path:
    return Path(os.environ.get("APPDATA", Path.home() / "AppData" / "Roaming")) / APP_NAME


def localappdata_root() -> Path:
    return Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData" / "Local")) / APP_NAME


APP_DATA_DIR = appdata_root()
LOCAL_DATA_DIR = localappdata_root()
CACHE_DIR = LOCAL_DATA_DIR / "Cache"
SESSION_DIR = LOCAL_DATA_DIR / "Session"
CONFIG_PATH = APP_DATA_DIR / CONFIG_FILENAME
LOG_PATH = APP_DATA_DIR / LOG_FILENAME
DEFAULT_CONFIG_PATH = app_root() / CONFIG_FILENAME
ASSETS_DIR = app_root() / "assets"
FALLBACK_ASSETS_DIR = bundled_root() / "assets"


def ensure_app_dirs() -> None:
    APP_DATA_DIR.mkdir(parents=True, exist_ok=True)
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    SESSION_DIR.mkdir(parents=True, exist_ok=True)


def resolve_asset(name: str) -> Path:
    primary = ASSETS_DIR / name
    if primary.exists():
        return primary
    return FALLBACK_ASSETS_DIR / name


def ensure_user_config() -> Path:
    ensure_app_dirs()
    if CONFIG_PATH.exists():
        return CONFIG_PATH

    candidates = [
        DEFAULT_CONFIG_PATH,
        bundled_root() / CONFIG_FILENAME,
        Path(__file__).resolve().parent / CONFIG_FILENAME,
    ]
    for candidate in candidates:
        if candidate.exists():
            shutil.copy2(candidate, CONFIG_PATH)
            return CONFIG_PATH
    CONFIG_PATH.write_text("{}", encoding="utf-8")
    return CONFIG_PATH


def clear_ephemeral_session() -> None:
    if SESSION_DIR.exists():
        shutil.rmtree(SESSION_DIR, ignore_errors=True)
    SESSION_DIR.mkdir(parents=True, exist_ok=True)


def executable_path() -> Path:
    if is_frozen():
        return Path(sys.executable).resolve()
    return Path(__file__).resolve().parent / "assistant_panel.py"


def source_pythonw() -> Path:
    python_path = Path(sys.executable).resolve()
    if python_path.name.lower() == "python.exe":
        candidate = python_path.with_name("pythonw.exe")
        if candidate.exists():
            return candidate
    return python_path
