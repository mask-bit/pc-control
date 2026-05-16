use crate::models::MusicCommandResult;
use crate::spotify;
use crate::storage::Database;
use crate::system_control;

pub async fn play_music(db: &Database, query: &str) -> Result<MusicCommandResult, String> {
    let app_opened = ensure_spotify_open()?;
    let resolved = resolve_music_query(db, query)?;
    if spotify::has_refresh_token() {
        if resolved.starts_with("spotify:") {
            match spotify::play_uri(db, resolved.clone()).await {
                Ok(_) => {
                    return Ok(MusicCommandResult {
                        app_opened,
                        playback_started: true,
                        method_used: "spotify_api".to_string(),
                        message: "Spotify abriu e comecei a tocar o item salvo.".to_string(),
                        limitation: None,
                    });
                }
                Err(error) => {
                    return desktop_search_result(app_opened, &resolved, Some(error));
                }
            }
        }
        match spotify::search_tracks(db, resolved.clone()).await {
            Ok(tracks) => {
                if let Some(track) = tracks.first().and_then(|track| track.uri.clone()) {
                    match spotify::play_uri(db, track).await {
                        Ok(_) => {
                            return Ok(MusicCommandResult {
                                app_opened,
                                playback_started: true,
                                method_used: "spotify_api".to_string(),
                                message: format!("Spotify abriu e comecei a tocar: {resolved}."),
                                limitation: None,
                            });
                        }
                        Err(error) => {
                            return desktop_search_result(app_opened, &resolved, Some(error));
                        }
                    }
                }
            }
            Err(error) => {
                return desktop_search_result(app_opened, &resolved, Some(error));
            }
        }
    }

    desktop_search_result(app_opened, &resolved, None)
}

pub async fn play_liked_music(db: &Database) -> Result<MusicCommandResult, String> {
    let app_opened = ensure_spotify_open()?;
    if spotify::has_refresh_token() {
        match spotify::play_liked_tracks(db).await {
            Ok(_) => {
                return Ok(MusicCommandResult {
                    app_opened,
                    playback_started: true,
                    method_used: "spotify_api".to_string(),
                    message: "Spotify abriu e comecei a tocar suas musicas curtidas.".to_string(),
                    limitation: None,
                });
            }
            Err(error) => {
                let _ = system_control::open_spotify_target(Some("spotify:collection:tracks"));
                return Ok(MusicCommandResult {
                    app_opened,
                    playback_started: false,
                    method_used: "spotify_desktop_navigation".to_string(),
                    message: "Abri o Spotify nas musicas curtidas, mas nao consegui confirmar a reproducao automatica."
                        .to_string(),
                    limitation: Some(error),
                });
            }
        }
    }

    let _ = system_control::open_spotify_target(Some("spotify:collection:tracks"));
    let desktop_result = system_control::send_media_command("play_pause");
    Ok(MusicCommandResult {
        app_opened,
        playback_started: desktop_result.is_ok(),
        method_used: "spotify_desktop_media_key".to_string(),
        message: if desktop_result.is_ok() {
            "Abri o Spotify nas musicas curtidas e enviei o comando de reproduzir pelo app instalado.".to_string()
        } else {
            "Abri o Spotify nas musicas curtidas, mas nao consegui controlar a reproducao pelo app instalado.".to_string()
        },
        limitation: desktop_result.err(),
    })
}

pub async fn pause_music(db: &Database) -> Result<MusicCommandResult, String> {
    transport_result(db, "pause").await
}

pub async fn next_track(db: &Database) -> Result<MusicCommandResult, String> {
    transport_result(db, "next").await
}

pub async fn previous_track(db: &Database) -> Result<MusicCommandResult, String> {
    transport_result(db, "previous").await
}

async fn transport_result(db: &Database, operation: &str) -> Result<MusicCommandResult, String> {
    let app_opened = ensure_spotify_open()?;
    if spotify::has_refresh_token() {
        let result = match operation {
            "pause" => spotify::pause(db).await,
            "next" => spotify::next(db).await,
            "previous" => spotify::previous(db).await,
            _ => Err("Operacao de musica desconhecida.".to_string()),
        };
        if result.is_ok() {
            return Ok(MusicCommandResult {
                app_opened,
                playback_started: operation != "pause",
                method_used: "spotify_api".to_string(),
                message: match operation {
                    "pause" => "Pausar solicitado no Spotify.".to_string(),
                    "next" => "Passei para a proxima faixa.".to_string(),
                    "previous" => "Voltei para a faixa anterior.".to_string(),
                    _ => "Comando de musica executado.".to_string(),
                },
                limitation: None,
            });
        }
    }

    let desktop_command = match operation {
        "pause" => "play_pause",
        "next" => "next",
        "previous" => "previous",
        _ => return Err("Operacao de musica desconhecida.".to_string()),
    };
    let desktop_result = system_control::send_media_command(desktop_command);
    Ok(MusicCommandResult {
        app_opened,
        playback_started: desktop_result.is_ok() && operation != "pause",
        method_used: "spotify_desktop_media_key".to_string(),
        message: match (operation, desktop_result.is_ok()) {
            ("pause", true) => "Enviei pausa para o Spotify instalado.".to_string(),
            ("next", true) => "Enviei proxima faixa para o Spotify instalado.".to_string(),
            ("previous", true) => "Enviei faixa anterior para o Spotify instalado.".to_string(),
            _ => "Abri o Spotify, mas nao consegui enviar o controle pelo app instalado.".to_string(),
        },
        limitation: desktop_result.err(),
    })
}

fn ensure_spotify_open() -> Result<bool, String> {
    if !system_control::spotify_installed() {
        return Err("Spotify nao esta instalado neste PC.".to_string());
    }
    system_control::open_spotify_target(None)?;
    Ok(true)
}

fn desktop_search_result(
    app_opened: bool,
    query: &str,
    limitation: Option<String>,
) -> Result<MusicCommandResult, String> {
    let uri = if query.starts_with("spotify:") {
        query.to_string()
    } else {
        format!("spotify:search:{}", query.replace(' ', "%20"))
    };
    let _ = system_control::open_spotify_target(Some(&uri));
    let _ = system_control::focus_spotify_window();
    Ok(MusicCommandResult {
        app_opened,
        playback_started: false,
        method_used: "spotify_desktop_search".to_string(),
        message: format!("Abri o Spotify na busca por {query}, mas nao consegui confirmar a reproducao automatica."),
        limitation: Some(
            limitation.unwrap_or_else(|| "Spotify API nao conectada nesta instalacao.".to_string()),
        ),
    })
}

fn resolve_music_query(db: &Database, query: &str) -> Result<String, String> {
    let normalized = query.trim();
    if normalized.is_empty() {
        return Ok(query.to_string());
    }
    if let Some(favorite) = db.find_music_favorite(normalized)? {
        if let Some(uri) = favorite.uri {
            return Ok(uri);
        }
        if let Some(saved_query) = favorite.query {
            return Ok(saved_query);
        }
    }
    let lower = normalized.to_lowercase();
    for memory in db.list_memories()? {
        if memory.category == "musica"
            && (memory.title.to_lowercase().contains(&lower)
                || memory.content.to_lowercase().contains(&lower))
        {
            return Ok(memory.content);
        }
    }
    Ok(normalized.to_string())
}
