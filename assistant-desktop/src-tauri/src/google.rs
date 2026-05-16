use crate::models::{AccountStatus, GoogleProfile};
use crate::storage::Database;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use rand::RngCore;
use reqwest::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";
const GOOGLE_REDIRECT_URI: &str = "http://127.0.0.1:43880/google/callback";
const GOOGLE_SCOPES: &str = "openid email profile";

pub async fn login(db: &Database) -> Result<GoogleProfile, String> {
    let client_id = google_client_id()?;
    let verifier = random_url_token(64);
    let challenge = code_challenge(&verifier);
    let csrf_state = random_url_token(24);

    let auth_url = format!(
        "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=online&prompt=select_account",
        GOOGLE_AUTH_URL,
        urlencoding::encode(&client_id),
        urlencoding::encode(GOOGLE_REDIRECT_URI),
        urlencoding::encode(GOOGLE_SCOPES),
        urlencoding::encode(&csrf_state),
        urlencoding::encode(&challenge),
    );

    let receiver = start_callback_listener()?;
    open::that(&auth_url).map_err(|err| format!("Falha ao abrir navegador: {err}"))?;
    let callback = receiver
        .recv_timeout(Duration::from_secs(120))
        .map_err(|_| "Tempo esgotado aguardando o login Google.".to_string())??;

    if callback.state != csrf_state {
        return Err("State OAuth nao confere. Login cancelado por seguranca.".to_string());
    }

    let token = exchange_code(&client_id, &verifier, &callback.code).await?;
    let access_token = token
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or("Google nao retornou access_token.")?;
    let profile = fetch_profile(access_token).await?;
    db.save_google_profile(&profile)?;
    Ok(profile)
}

pub fn logout(db: &Database) -> Result<bool, String> {
    db.clear_google_profile()?;
    Ok(true)
}

pub fn account_status(db: &Database) -> Result<AccountStatus, String> {
    let profile = db.get_google_profile()?;
    Ok(AccountStatus {
        google_connected: profile.is_some(),
        profile,
    })
}

fn google_client_id() -> Result<String, String> {
    option_env!("GOOGLE_OAUTH_CLIENT_ID")
        .map(str::to_string)
        .or_else(|| std::env::var("GOOGLE_OAUTH_CLIENT_ID").ok())
        .or_else(|| std::env::var("GOOGLE_CLIENT_ID").ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or(
            "Google OAuth Client ID do aplicativo nao foi configurado no build. Defina GOOGLE_OAUTH_CLIENT_ID."
                .to_string(),
        )
}

struct OAuthCallback {
    code: String,
    state: String,
}

fn start_callback_listener() -> Result<mpsc::Receiver<Result<OAuthCallback, String>>, String> {
    let listener = TcpListener::bind("127.0.0.1:43880")
        .map_err(|err| format!("Porta local do login Google indisponivel: {err}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| err.to_string())?;
    let (sender, receiver) = mpsc::channel();

    std::thread::spawn(move || {
        let result = handle_callback(listener);
        let _ = sender.send(result);
    });

    Ok(receiver)
}

fn handle_callback(listener: TcpListener) -> Result<OAuthCallback, String> {
    let deadline = Instant::now() + Duration::from_secs(125);
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
        .ok_or("Retorno Google invalido.")?;
    let parsed = url::Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|err| format!("URL de retorno Google invalida: {err}"))?;

    if parsed.path() != "/google/callback" {
        write_callback_response(&mut stream, false)?;
        return Err("Caminho de retorno Google invalido.".to_string());
    }

    let error = parsed
        .query_pairs()
        .find(|(key, _)| key == "error")
        .map(|(_, value)| value.to_string());
    if let Some(error) = error {
        write_callback_response(&mut stream, false)?;
        return Err(format!("Google recusou login: {error}"));
    }

    let code = parsed
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.to_string())
        .ok_or("Codigo Google ausente.")?;
    let state = parsed
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .ok_or("State Google ausente.")?;

    write_callback_response(&mut stream, true)?;
    Ok(OAuthCallback { code, state })
}

fn write_callback_response(stream: &mut std::net::TcpStream, ok: bool) -> Result<(), String> {
    let (title, body) = if ok {
        ("PC Control AI conectado ao Google", "Voce ja pode fechar esta janela.")
    } else {
        ("Falha ao conectar Google", "Volte ao aplicativo e tente novamente.")
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

async fn exchange_code(client_id: &str, verifier: &str, code: &str) -> Result<Value, String> {
    let client = Client::new();
    let response = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", GOOGLE_REDIRECT_URI),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|err| err.to_string())?;
    let status = response.status();
    let value: Value = response.json().await.map_err(|err| err.to_string())?;
    if !status.is_success() {
        return Err(value
            .get("error_description")
            .and_then(Value::as_str)
            .unwrap_or("Google retornou erro ao trocar o codigo OAuth.")
            .to_string());
    }
    Ok(value)
}

async fn fetch_profile(access_token: &str) -> Result<GoogleProfile, String> {
    let client = Client::new();
    let response = client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    let status = response.status();
    let value: Value = response.json().await.map_err(|err| err.to_string())?;
    if !status.is_success() {
        return Err("Falha ao buscar perfil Google.".to_string());
    }

    Ok(GoogleProfile {
        email: value
            .get("email")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        name: value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Conta Google")
            .to_string(),
        picture: value
            .get("picture")
            .and_then(Value::as_str)
            .filter(|picture| !picture.trim().is_empty())
            .map(str::to_string),
        last_verified_at: Utc::now().to_rfc3339(),
    })
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
