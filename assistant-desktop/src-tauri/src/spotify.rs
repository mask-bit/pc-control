use crate::models::{SpotifyPlaylist, SpotifyTrack};
use crate::secrets;
use crate::storage::Database;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use rand::RngCore;
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use url::Url;

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

pub fn begin_auth(
    state: &SpotifyAuthState,
    client_id: String,
    redirect_uri: String,
) -> Result<String, String> {
    if client_id.trim().is_empty() {
        return Err("Informe o Spotify Client ID.".to_string());
    }

    let verifier = random_url_token(64);
    let challenge = code_challenge(&verifier);
    let csrf_state = random_url_token(24);
    state.set_pending(PendingSpotifyAuth {
        client_id: client_id.clone(),
        redirect_uri: redirect_uri.clone(),
        verifier,
        state: csrf_state.clone(),
    })?;

    let scopes = [
        "playlist-read-private",
        "playlist-read-collaborative",
        "user-read-playback-state",
        "user-read-currently-playing",
        "user-modify-playback-state",
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

    let _ = open::that(&url);
    Ok(url)
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

    let client = Client::new();
    let response = client
        .post("https://accounts.spotify.com/api/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
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

    store_tokens(db, value)?;
    Ok(true)
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
        .unwrap_or(&Vec::new())
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
        .unwrap_or(&Vec::new())
        .iter()
        .map(playlist_from_value)
        .collect())
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
