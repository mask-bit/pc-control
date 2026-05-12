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


def test_parse_youtube_search():
    intent = parse_command("pesquisar lo-fi no YouTube", CONFIG)

    assert intent.type == "search_youtube"
    assert intent.target == "lo-fi"


def test_parse_compound_command():
    intent = parse_command("abrir Chrome e Spotify", CONFIG)

    assert intent.type == "sequence"
    assert intent.parameters["steps"] == ["abrir chrome", "abrir spotify"]


def test_parse_compound_with_then_command():
    intent = parse_command("abrir WhatsApp e depois tocar lo-fi", CONFIG)

    assert intent.type == "sequence"
    assert intent.parameters["steps"] == ["abrir whatsapp", "tocar lo-fi"]


def test_parse_close_app():
    intent = parse_command("fechar Spotify", CONFIG)

    assert intent.type == "close_app"
    assert intent.target == "spotify"
