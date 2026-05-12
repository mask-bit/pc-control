"""Armazenamento local de segredos do Jarvis.

No Windows, usa DPAPI do proprio usuario para criptografar os valores antes de
salvar no AppData. Se a biblioteca keyring estiver instalada, ela e usada
primeiro por integrar melhor com o gerenciador de credenciais do sistema.
"""

from __future__ import annotations

import base64
import ctypes
import json
import os
from ctypes import wintypes
from pathlib import Path

APP_NAME = "JarvisAssistant"
SERVICE_NAME = "JarvisAssistant"


class DATA_BLOB(ctypes.Structure):
    _fields_ = [("cbData", wintypes.DWORD), ("pbData", ctypes.POINTER(ctypes.c_char))]


def _store_path() -> Path:
    base = Path(os.environ.get("APPDATA", Path.home())) / APP_NAME
    base.mkdir(parents=True, exist_ok=True)
    return base / "secrets.json"


def _blob_from_bytes(data: bytes) -> DATA_BLOB:
    buffer = ctypes.create_string_buffer(data)
    return DATA_BLOB(len(data), ctypes.cast(buffer, ctypes.POINTER(ctypes.c_char)))


def _protect(data: bytes) -> bytes:
    if os.name != "nt":
        return data
    crypt32 = ctypes.windll.crypt32
    in_blob = _blob_from_bytes(data)
    out_blob = DATA_BLOB()
    if not crypt32.CryptProtectData(
        ctypes.byref(in_blob),
        None,
        None,
        None,
        None,
        0,
        ctypes.byref(out_blob),
    ):
        raise OSError("Falha ao proteger segredo com DPAPI.")
    try:
        return ctypes.string_at(out_blob.pbData, out_blob.cbData)
    finally:
        ctypes.windll.kernel32.LocalFree(out_blob.pbData)


def _unprotect(data: bytes) -> bytes:
    if os.name != "nt":
        return data
    crypt32 = ctypes.windll.crypt32
    in_blob = _blob_from_bytes(data)
    out_blob = DATA_BLOB()
    if not crypt32.CryptUnprotectData(
        ctypes.byref(in_blob),
        None,
        None,
        None,
        None,
        0,
        ctypes.byref(out_blob),
    ):
        raise OSError("Falha ao ler segredo protegido com DPAPI.")
    try:
        return ctypes.string_at(out_blob.pbData, out_blob.cbData)
    finally:
        ctypes.windll.kernel32.LocalFree(out_blob.pbData)


def _read_file_store() -> dict[str, str]:
    path = _store_path()
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return {}


def _write_file_store(data: dict[str, str]) -> None:
    _store_path().write_text(json.dumps(data, indent=2), encoding="utf-8")


def set_secret(name: str, value: str) -> None:
    name = name.strip()
    if not name:
        raise ValueError("Nome do segredo vazio.")
    try:
        import keyring

        keyring.set_password(SERVICE_NAME, name, value)
        return
    except Exception:
        pass

    store = _read_file_store()
    encrypted = _protect(value.encode("utf-8"))
    store[name] = base64.b64encode(encrypted).decode("ascii")
    _write_file_store(store)


def get_secret(name: str) -> str:
    name = name.strip()
    if not name:
        return ""
    try:
        import keyring

        value = keyring.get_password(SERVICE_NAME, name)
        if value:
            return value
    except Exception:
        pass

    store = _read_file_store()
    encoded = store.get(name)
    if not encoded:
        return ""
    try:
        encrypted = base64.b64decode(encoded.encode("ascii"))
        return _unprotect(encrypted).decode("utf-8")
    except Exception:
        return ""


def delete_secret(name: str) -> None:
    try:
        import keyring

        keyring.delete_password(SERVICE_NAME, name)
    except Exception:
        pass
    store = _read_file_store()
    if name in store:
        del store[name]
        _write_file_store(store)


def has_secret(name: str) -> bool:
    return bool(get_secret(name))
