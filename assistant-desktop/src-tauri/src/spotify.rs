use crate::models::{LogEntry, SpotifyPlaylist, SpotifyTrack};
use crate::secrets;
use crate::storage::Database;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use rand::RngCore;
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use url::Url;
use uuid::Uuid;

#[derive(Clone)]
pub struct PendingSpotifyAuth {
    pub client_id: String,
    pub redirect_uri: String,
    pub verifier: String,
    pub state: String,
}

pub struct SpotifyAuthState {
    pending: Mutex<Option<PendingSpotifyAuth>>,
}

impl SpotifyAuthState {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(None),
        }
    }

    pub fn set_pending(&self, pending: PendingSpotifyAuth) -> Result<(), String> {
        *self.pending.lock().map_err(|err| err.to_string())? = Some(pending);
        Ok(())
    }

    pub fn take_pending(&self) -> Result<Option<PendingSpotifyAuth>, String> {
        Ok(self.pending.lock().map_err(|err| err.to_string())?.take())
    }
}

#[allow(dead_code)]
pub fn begin_auth(
    state: &SpotifyAuthState,
    client_id: String,
    redirect_uri: String,
) -> Result<String, String> {
    let (pending, url) = build_auth_request(client_id, redirect_uri)?;
    state.set_pending(pending)?;
    let _ = open::that(&url);
    Ok(url)
}

pub fn begin_auth_with_callback(
    db: Arc<Database>,
    state: &SpotifyAuthState,
    client_id: String,
    redirect_uri: String,
) -> Result<String, String> {
    let (pending, url) = build_auth_request(client_id, redirect_uri)?;
    state.set_pending(pending.clone())?;
    match start_callback_listener(db.clone(), pending) {
        Ok(()) => add_log(&db, "spotify", "Aguardando retorno OAuth local do Spotify.", "info"),
        Err(error) => add_log(
            &db,
            "spotify",
            &format!("Callback automatico indisponivel; finalize com URL manual: {error}"),
            "warn",
        ),
    }
    let _ = open::that(&url);
    Ok(url)
}

fn build_auth_request(client_id: String, redirect_uri: String) -> Result<(PendingSpotifyAuth, String), String> {
    if client_id.trim().is_empty() {
        return Err("Informe o Spotify Client ID.".to_string());
    }

    let verifier = random_url_token(64);
    let challenge = code_challenge(&verifier);
    let csrf_state = random_url_token(24);
    let pending = PendingSpotifyAuth {
        client_id: client_id.clone(),
        redirect_uri: redirect_uri.clone(),
        verifier,
        state: csrf_state.clone(),
    };

    let scopes = [
        "playlist-read-private",
        "playlist-read-collaborative",
        "user-read-playback-state",
        "user-read-currently-playing",
        "user-modify-playback-state",
        "user-library-read",
    ]
    .join(" ");

    let url = format!(
        "https://accounts.spotify.com/authorize?response_type=code&client_id={}&scope={}&redirect_uri={}&state={}&code_challenge_method=S256&code_challenge={}",
        urlencoding::encode(&client_id),
        urlencoding::encode(&scopes),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&csrf_state),
        urlencoding::encode(&challenge)
    );

    Ok((pending, url))
}

pub async fn finish_auth(
    db: &Database,
    state: &SpotifyAuthState,
    callback_url: String,
) -> Result<bool, String> {
    let pending = state
        .take_pending()?
        .ok_or("Nenhum login Spotify em andamento.")?;
    let parsed = Url::parse(&callback_url).map_err(|err| format!("URL de retorno invalida: {err}"))?;
    let code = parsed
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.to_string())
        .ok_or("Codigo OAuth ausente na URL de retorno.")?;
    let returned_state = parsed
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .ok_or("State OAuth ausente na URL de retorno.")?;

    if returned_state != pending.state {
        return Err("State OAuth nao confere. Login cancelado por seguranca.".to_string());
    }

    let value = exchange_auth_code(&pending, &code).await?;
    store_tokens(db, value)?;
    Ok(true)
}

fn start_callback_listener(db: Arc<Database>, pending: PendingSpotifyAuth) -> Result<(), String> {
    let parsed = Url::parse(&pending.redirect_uri)
        .map_err(|err| format!("Redirect URI Spotify invalida: {err}"))?;
    let host = parsed.host_str().ok_or("Redirect URI Spotify sem host.")?;
    if host != "127.0.0.1" && host != "localhost" {
        return Err("callback automatico exige redirect_uri em 127.0.0.1 ou localhost.".to_string());
    }
    let bind_host = if host == "localhost" { "127.0.0.1" } else { host };
    let port = parsed
        .port_or_known_default()
        .ok_or("Redirect URI Spotify sem porta.")?;
    let expected_path = parsed.path().to_string();
    let listener = TcpListener::bind(format!("{bind_host}:{port}"))
        .map_err(|err| format!("Porta local do Spotify indisponivel: {err}"))?;
    listener.set_nonblocking(true).map_err(|err| err.to_string())?;

    std::thread::spawn(move || {
        if let Err(error) = handle_callback(listener, expected_path, pending, db.clone()) {
            add_log(&db, "spotify", &format!("Falha no callback Spotify: {error}"), "error");
        }
    });

    Ok(())
}

fn handle_callback(
    listener: TcpListener,
    expected_path: String,
    pending: PendingSpotifyAuth,
    db: Arc<Database>,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(180);
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(err) => return Err(err.to_string()),
        }
    };

    let mut buffer = [0_u8; 4096];
    let read = stream.read(&mut buffer).map_err(|err| err.to_string())?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let first_line = request.lines().next().unwrap_or_default();
    let target = first_line
        .split_whitespace()
        .nth(1)
        .ok_or("Retorno Spotify invalido.")?;
    let parsed = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|err| format!("URL de retorno Spotify invalida: {err}"))?;

    if parsed.path() != expected_path {
        write_callback_response(&mut stream, false)?;
        return Err("Caminho de retorno Spotify invalido.".to_string());
    }

    if let Some(error) = parsed
        .query_pairs()
        .find(|(key, _)| key == "error")
        .map(|(_, value)| value.to_string())
    {
        write_callback_response(&mut stream, false)?;
        return Err(format!("Spotify recusou login: {error}"));
    }

    let code = parsed
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.to_string())
        .ok_or("Codigo Spotify ausente.")?;
    let returned_state = parsed
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .ok_or("State Spotify ausente.")?;

    if returned_state != pending.state {
        write_callback_response(&mut stream, false)?;
        return Err("State OAuth nao confere. Login Spotify cancelado por seguranca.".to_string());
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| err.to_string())?;
    let value = runtime.block_on(exchange_auth_code(&pending, &code))?;
    store_tokens(&db, value)?;
    add_log(&db, "spotify", "Spotify conectado com OAuth PKCE.", "info");
    write_callback_response(&mut stream, true)?;
    Ok(())
}

fn write_callback_response(stream: &mut std::net::TcpStream, ok: bool) -> Result<(), String> {
    let (title, body) = if ok {
        ("PC Control AI conectado ao Spotify", "Voce ja pode fechar esta janela.")
    } else {
        ("Falha ao conectar Spotify", "Volte ao aplicativo e tente novamente.")
    };
    let html = format!(
        "<html><body style='font-family:Segoe UI;background:#101827;color:#f8fafc'><h2>{title}</h2><p>{body}</p></body></html>"
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        html.len(),
        html
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|err| err.to_string())
}

async fn exchange_auth_code(pending: &PendingSpotifyAuth, code: &str) -> Result<Value, String> {
    let client = Client::new();
    let response = client
        .post("https://accounts.spotify.com/api/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", pending.redirect_uri.as_str()),
            ("client_id", pending.client_id.as_str()),
            ("code_verifier", pending.verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|err| err.to_string())?;
    let status = response.status();
    let value: Value = response.json().await.map_err(|err| err.to_string())?;
    if !status.is_success() {
        return Err(value.to_string());
    }
    Ok(value)
}

pub async fn current_track(db: &Database) -> Result<Option<SpotifyTrack>, String> {
    let value = spotify_request(db, reqwest::Method::GET, "https://api.spotify.com/v1/me/player/currently-playing", None).await?;
    if value.is_null() {
        return Ok(None);
    }
    let item = value.get("item").cloned().unwrap_or(Value::Null);
    if item.is_null() {
        return Ok(None);
    }
    Ok(Some(track_from_value(&item, value.get("is_playing").and_then(Value::as_bool))))
}

pub async fn list_playlists(db: &Database) -> Result<Vec<SpotifyPlaylist>, String> {
    let value = spotify_request(db, reqwest::Method::GET, "https://api.spotify.com/v1/me/playlists?limit=30", None).await?;
    Ok(value
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(playlist_from_value)
        .collect())
}

pub async fn search_playlists(db: &Database, query: String) -> Result<Vec<SpotifyPlaylist>, String> {
    let url = format!(
        "https://api.spotify.com/v1/search?type=playlist&limit=20&q={}",
        urlencoding::encode(&query)
    );
    let value = spotify_request(db, reqwest::Method::GET, &url, None).await?;
    Ok(value
        .pointer("/playlists/items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(playlist_from_value)
        .collect())
}

pub async fn search_tracks(db: &Database, query: String) -> Result<Vec<SpotifyTrack>, String> {
    let url = format!(
        "https://api.spotify.com/v1/search?type=track&limit=10&q={}",
        urlencoding::encode(&query)
    );
    let value = spotify_request(db, reqwest::Method::GET, &url, None).await?;
    Ok(value
        .pointer("/tracks/items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|track| track_from_value(track, None))
        .collect())
}

pub async fn liked_tracks(db: &Database) -> Result<Vec<SpotifyTrack>, String> {
    let value = spotify_request(db, reqwest::Method::GET, "https://api.spotify.com/v1/me/tracks?limit=50", None).await?;
    Ok(value
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|item| item.get("track"))
        .map(|track| track_from_value(track, None))
        .collect())
}

pub async fn play_liked_tracks(db: &Database) -> Result<bool, String> {
    let tracks = liked_tracks(db).await?;
    let uris = tracks
        .into_iter()
        .filter_map(|track| track.uri)
        .take(50)
        .collect::<Vec<_>>();
    if uris.is_empty() {
        return Err("Nenhuma musica curtida encontrada no Spotify.".to_string());
    }
    spotify_request(
        db,
        reqwest::Method::PUT,
        "https://api.spotify.com/v1/me/player/play",
        Some(json!({ "uris": uris })),
    )
    .await?;
    Ok(true)
}

pub async fn play_uri(db: &Database, uri: String) -> Result<bool, String> {
    let body = if uri.starts_with("spotify:track:") {
        json!({ "uris": [uri] })
    } else {
        json!({ "context_uri": uri })
    };
    spotify_request(db, reqwest::Method::PUT, "https://api.spotify.com/v1/me/player/play", Some(body)).await?;
    Ok(true)
}

pub async fn pause(db: &Database) -> Result<bool, String> {
    spotify_request(db, reqwest::Method::PUT, "https://api.spotify.com/v1/me/player/pause", None).await?;
    Ok(true)
}

pub async fn next(db: &Database) -> Result<bool, String> {
    spotify_request(db, reqwest::Method::POST, "https://api.spotify.com/v1/me/player/next", None).await?;
    Ok(true)
}

pub async fn previous(db: &Database) -> Result<bool, String> {
    spotify_request(db, reqwest::Method::POST, "https://api.spotify.com/v1/me/player/previous", None).await?;
    Ok(true)
}

pub async fn set_volume(db: &Database, volume: u8) -> Result<bool, String> {
    let value = volume.min(100);
    let url = format!("https://api.spotify.com/v1/me/player/volume?volume_percent={value}");
    spotify_request(db, reqwest::Method::PUT, &url, None).await?;
    Ok(true)
}

pub fn has_refresh_token() -> bool {
    secrets::has_secret("spotify_refresh_token")
}

async fn spotify_request(
    db: &Database,
    method: reqwest::Method,
    url: &str,
    body: Option<Value>,
) -> Result<Value, String> {
    let token = ensure_access_token(db).await?;
    let client = Client::new();
    let mut request = client.request(method, url).bearer_auth(token);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.map_err(|err| err.to_string())?;
    if response.status().as_u16() == 204 {
        return Ok(Value::Null);
    }
    let status = response.status();
    let text = response.text().await.map_err(|err| err.to_string())?;
    if !status.is_success() {
        return Err(text);
    }
    if text.trim().is_empty() {
        Ok(Value::Null)
    } else {
        serde_json::from_str(&text).map_err(|err| err.to_string())
    }
}

async fn ensure_access_token(db: &Database) -> Result<String, String> {
    let expires_at = db
        .get_setting("spotify_expires_at")?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    if expires_at > Utc::now().timestamp() + 60 {
        if let Some(token) = secrets::get_secret("spotify_access_token") {
            return Ok(token);
        }
    }
    refresh_access_token(db).await
}

async fn refresh_access_token(db: &Database) -> Result<String, String> {
    let settings = db.get_settings()?;
    let refresh_token = secrets::get_secret("spotify_refresh_token")
        .ok_or("Spotify ainda nao esta conectado.")?;
    let client = Client::new();
    let response = client
        .post("https://accounts.spotify.com/api/token")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", settings.spotify_client_id.as_str()),
        ])
        .send()
        .await
        .map_err(|err| err.to_string())?;
    let status = response.status();
    let value: Value = response.json().await.map_err(|err| err.to_string())?;
    if !status.is_success() {
        return Err(value.to_string());
    }
    store_tokens(db, value.clone())?;
    secrets::get_secret("spotify_access_token").ok_or("Token Spotify nao encontrado.".to_string())
}

fn store_tokens(db: &Database, value: Value) -> Result<(), String> {
    if let Some(access) = value.get("access_token").and_then(Value::as_str) {
        secrets::set_secret("spotify_access_token", access)?;
    }
    if let Some(refresh) = value.get("refresh_token").and_then(Value::as_str) {
        secrets::set_secret("spotify_refresh_token", refresh)?;
    }
    let expires_in = value.get("expires_in").and_then(Value::as_i64).unwrap_or(3600);
    let expires_at = Utc::now().timestamp() + expires_in;
    db.set_setting("spotify_expires_at", &expires_at.to_string())?;
    Ok(())
}

fn add_log(db: &Database, module: &str, message: &str, level: &str) {
    let _ = db.add_log(&LogEntry {
        id: Uuid::new_v4().to_string(),
        level: level.to_string(),
        module: module.to_string(),
        message: message.to_string(),
        created_at: Utc::now().to_rfc3339(),
    });
}

fn track_from_value(item: &Value, is_playing: Option<bool>) -> SpotifyTrack {
    SpotifyTrack {
        id: item.get("id").and_then(Value::as_str).unwrap_or("").to_string(),
        name: item.get("name").and_then(Value::as_str).unwrap_or("Spotify").to_string(),
        artist: item
            .get("artists")
            .and_then(Value::as_array)
            .and_then(|artists| artists.first())
            .and_then(|artist| artist.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        album: item
            .pointer("/album/name")
            .and_then(Value::as_str)
            .map(str::to_string),
        image_url: item
            .pointer("/album/images/0/url")
            .and_then(Value::as_str)
            .map(str::to_string),
        uri: item.get("uri").and_then(Value::as_str).map(str::to_string),
        is_playing,
    }
}

fn playlist_from_value(item: &Value) -> SpotifyPlaylist {
    SpotifyPlaylist {
        id: item.get("id").and_then(Value::as_str).unwrap_or("").to_string(),
        name: item.get("name").and_then(Value::as_str).unwrap_or("Playlist").to_string(),
        description: item
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string),
        image_url: item
            .pointer("/images/0/url")
            .and_then(Value::as_str)
            .map(str::to_string),
        tracks_total: item
            .pointer("/tracks/total")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        uri: item.get("uri").and_then(Value::as_str).unwrap_or("").to_string(),
    }
}

fn random_url_token(byte_len: usize) -> String {
    let mut bytes = vec![0_u8; byte_len];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}
