import json
import os

DEFAULT_CONFIG = {
    "czulosc_klasniecia": 70,
    "hotkey": "Win+Shift+W",
    "zdarzenie_dzwiekowe": "Clapping",
    "liczba_zdarzen": 2,
    "cooldown": 3,
    "czulosc_nn": 0.12,
    "slowa_kluczowe": "",
    "jezyk_mowy": "pl",
    "profil_aktywny": "Default",
    "profile": {
        "Default": {
            "aplikacje": [],
            "terminale": [],
        }
    },
}

def _copy_config_defaults(raw):
    cfg = json.loads(json.dumps(DEFAULT_CONFIG))
    if isinstance(raw, dict):
        for key, value in raw.items():
            if key not in ("profile", "profiles"):
                cfg[key] = value
    return cfg

def _as_list(value):
    if value is None:
        return []
    if isinstance(value, list):
        return value
    return [value]

def _legacy_screen(value, default=1):
    try:
        return max(1, int(value) + 1)
    except (TypeError, ValueError):
        return default

def _legacy_profile_to_generic(name, data):
    if not isinstance(data, dict):
        return {"aplikacje": [], "terminale": []}

    apps = list(data.get("aplikacje", []) or [])
    terms = list(data.get("terminale", []) or [])

    vscode = data.get("vscode")
    if isinstance(vscode, dict):
        screen = _legacy_screen(vscode.get("monitor", 0))
        for project in _as_list(vscode.get("projects")):
            if project:
                apps.append({
                    "nazwa": "VS Code",
                    "exe": "code",
                    "argumenty": str(project),
                    "ekran": screen,
                    "polowa": "",
                    "warstwa": "Normal",
                    "kolejnosc": 0,
                    "minimalizuj": False,
                })

    teams = data.get("teams")
    if isinstance(teams, dict) and teams.get("enabled"):
        apps.append({
            "nazwa": "Microsoft Teams",
            "exe": "",
            "argumenty": "msteams:",
            "ekran": _legacy_screen(teams.get("monitor", 0)),
            "polowa": "",
            "warstwa": "Normal",
            "kolejnosc": 0,
            "minimalizuj": False,
        })

    spotify = data.get("spotify")
    if isinstance(spotify, dict) and spotify.get("enabled"):
        search = str(spotify.get("search", "") or "").strip()
        apps.append({
            "nazwa": "Spotify",
            "exe": "",
            "argumenty": f"spotify:search:{search}" if search else "spotify:",
            "ekran": _legacy_screen(spotify.get("monitor", 0)),
            "polowa": "",
            "warstwa": "Normal",
            "kolejnosc": 0,
            "minimalizuj": False,
        })

    claude = data.get("claude")
    if isinstance(claude, dict) and claude.get("enabled"):
        terms.append({
            "nazwa": "Claude",
            "terminal_typ": "Git Bash",
            "folder": str(claude.get("path", "") or os.path.expanduser("~")),
            "komenda": str(claude.get("komenda", "") or claude.get("command", "") or "claude"),
            "ekran": _legacy_screen(claude.get("monitor", 0)),
            "polowa": "",
            "warstwa": "Normal",
        })

    return {"aplikacje": apps, "terminale": terms}

def normalize_config(raw):
    changed = False
    cfg = _copy_config_defaults(raw)

    source_profiles = {}
    if isinstance(raw, dict):
        if isinstance(raw.get("profile"), dict):
            source_profiles = raw["profile"]
        elif isinstance(raw.get("profiles"), dict):
            source_profiles = raw["profiles"]
            changed = True
        elif raw.get("aplikacje") is not None or raw.get("terminale") is not None:
            active = raw.get("profil_aktywny") or "Default"
            source_profiles = {
                active: {
                    "aplikacje": raw.get("aplikacje", []),
                    "terminale": raw.get("terminale", []),
                }
            }
            changed = True
        else:
            source_profiles = cfg["profile"]
            changed = True
    else:
        source_profiles = cfg["profile"]
        changed = True

    profiles = {}
    for name, data in source_profiles.items():
        profile_name = str(name or "Default")
        profiles[profile_name] = _legacy_profile_to_generic(profile_name, data)
        legacy_keys = {"vscode", "teams", "spotify", "claude"}
        if not isinstance(data, dict) or legacy_keys.intersection(data.keys()):
            changed = True

    if not profiles:
        profiles = cfg["profile"]
        changed = True

    active = cfg.get("profil_aktywny") or next(iter(profiles.keys()))
    if active not in profiles:
        active = next(iter(profiles.keys()))
        changed = True

    cfg["profil_aktywny"] = active
    cfg["profile"] = profiles
    if isinstance(raw, dict) and "profiles" in raw:
        changed = True
    return cfg, changed

def load_config(path, save_normalized=True):
    if not os.path.exists(path):
        return None
    with open(path, "r", encoding="utf-8") as f:
        raw = json.load(f)
    cfg, changed = normalize_config(raw)
    if save_normalized and changed:
        with open(path, "w", encoding="utf-8") as f:
            json.dump(cfg, f, indent=2, ensure_ascii=False)
    return cfg
