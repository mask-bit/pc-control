from intent_parser import parse_command

CONFIG = {
    "wake_words": ["jarvis", "assistente"],
    "sites": {"youtube": "https://www.youtube.com"},
}


def test_parse_open_app_with_article():
    intent = parse_command("abre o Chrome", CONFIG)

    assert intent.type == "open_app"
    assert intent.target == "o chrome"


def test_parse_open_site():
    intent = parse_command("abre YouTube", CONFIG)

    assert intent.type == "open_site"
    assert intent.target == "youtube"


def test_parse_spotify_playlist():
    intent = parse_command("toca playlist foco", CONFIG)

    assert intent.type == "spotify_play"
    assert intent.target == "playlist foco"


def test_parse_wake_word_inline_command():
    intent = parse_command("Jarvis abre YouTube", CONFIG)

    assert intent.type == "open_site"
    assert intent.target == "youtube"
