from command_router import CommandRouter
from intent_parser import parse_command

CONFIG = {
    "apps": {"chrome": "chrome"},
    "sites": {"youtube": "https://www.youtube.com"},
    "pastas": {"downloads": "%USERPROFILE%\\Downloads"},
    "spotify": {
        "playlist foco": "https://open.spotify.com/playlist/example",
        "default_query": "lo-fi focus",
    },
    "rotinas": {
        "modo estudo": [
            {
                "type": "spotify_play",
                "label": "Tocar foco",
                "args": {"query": "playlist foco"},
                "risk": "low",
                "requires_confirmation": False,
            }
        ]
    },
}


def router(tmp_path):
    return CommandRouter(CONFIG, str(tmp_path / "assistant_logs.jsonl"))


def test_open_app_action(tmp_path):
    result = router(tmp_path).intent_to_action(parse_command("abre o Chrome", CONFIG))

    assert result is not None
    assert result.type == "open_app"
    assert result.args["target"] == "chrome"


def test_open_site_action(tmp_path):
    result = router(tmp_path).intent_to_action(parse_command("abre YouTube", CONFIG))

    assert result is not None
    assert result.type == "open_url"
    assert result.args["url"] == "https://www.youtube.com"


def test_spotify_action_uses_configured_playlist(tmp_path):
    result = router(tmp_path).intent_to_action(parse_command("toca playlist foco", CONFIG))

    assert result is not None
    assert result.type == "spotify_play"
    assert result.args["query"] == "https://open.spotify.com/playlist/example"


def test_routine_action(tmp_path):
    result = router(tmp_path).intent_to_action(parse_command("ativar modo estudo", CONFIG))

    assert result is not None
    assert result.type == "run_routine"
    assert result.args["name"] == "modo estudo"


def test_unknown_command_does_not_execute(tmp_path):
    result = router(tmp_path).handle_text("faz uma coisa misteriosa")

    assert result.success is False
    assert result.action is None
