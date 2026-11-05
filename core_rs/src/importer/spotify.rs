//! Optional Spotify library import.
//!
//! Authorization is OAuth 2.0 Authorization Code with PKCE, the flow Spotify
//! documents for public desktop clients. No client secret is shipped, no
//! password is ever requested, and only read scopes are asked for. The access
//! token is kept in the platform secret store when one is available, with a
//! 0600 file as the documented fallback.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::formats::{ParsedPlaylist, ParsedTrack};

const AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const API_BASE: &str = "https://api.spotify.com/v1";
const SCOPES: &str = "user-library-read playlist-read-private playlist-read-collaborative";
const REDIRECT_PORT: u16 = 43821;
const REDIRECT_PATH: &str = "/callback";
const SECRET_SERVICE: &str = "melyar";
const SECRET_ACCOUNT: &str = "spotify";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Token {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn client_id() -> Option<String> {
    crate::get_core()
        .storage()
        .get_setting("spotify.clientId")
        .ok()
        .flatten()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn redirect_uri() -> String {
    format!("http://127.0.0.1:{REDIRECT_PORT}{REDIRECT_PATH}")
}

pub fn status_json() -> Value {
    let configured = client_id().is_some();
    let token = load_token().unwrap_or_default();
    let account = crate::get_core()
        .storage()
        .get_provider_account("spotify")
        .ok()
        .flatten();
    let connected = !token.access_token.is_empty();
    let account_name = account
        .as_ref()
        .filter(|a| a.connected)
        .map(|a| a.account_name.clone())
        .unwrap_or_default();
    json!({
        "status": if connected { "connected" } else if configured { "ready" } else { "requires_configuration" },
        "configured": configured,
        "connected": connected,
        "account": account_name,
        "redirectUri": redirect_uri(),
        "scopes": SCOPES.split(' ').collect::<Vec<_>>(),
        "reason": if configured {
            Value::Null
        } else {
            json!("A Spotify developer application Client ID is needed. Create one at developer.spotify.com, add http://127.0.0.1:43821/callback as a redirect URI, then paste the Client ID here. No client secret is used.")
        },
    })
}

pub fn set_client_id(id: String) -> bool {
    let id = id.trim().to_string();
    if id.is_empty() || id.chars().any(|c| c.is_whitespace()) {
        super::emit(json!({
            "type": "error",
            "code": "invalid_client_id",
            "message": "The Spotify Client ID is empty or contains spaces.",
        }));
        return false;
    }
    let storage = crate::get_core().storage();
    let ok = storage.set_setting("spotify.clientId", &id).is_ok();
    super::emit(json!({"type": "provider_state", "provider": "spotify", "state": status_json()}));
    ok
}

/// Start the authorization flow. The URL goes to the interface, which opens
/// the user's browser; the loopback listener catches the redirect.
pub fn connect() -> bool {
    let Some(client_id) = client_id() else {
        super::emit(json!({
            "type": "error",
            "code": "requires_configuration",
            "message": "Add a Spotify Client ID before connecting.",
        }));
        return false;
    };

    let verifier = random_string(64);
    let challenge = URL_SAFE_NO_PAD.encode(sha256(verifier.as_bytes()));
    let state = random_string(24);
    let auth_url = format!(
        "{AUTH_URL}?response_type=code&client_id={}&scope={}&redirect_uri={}&state={}&code_challenge_method=S256&code_challenge={}",
        urlencode(&client_id),
        urlencode(SCOPES),
        urlencode(&redirect_uri()),
        urlencode(&state),
        urlencode(&challenge),
    );

    let core = crate::get_core();
    core.runtime().spawn(async move {
        super::emit(json!({
            "type": "spotify_auth_url",
            "url": auth_url,
            "redirectUri": redirect_uri(),
            "scopes": SCOPES.split(' ').collect::<Vec<_>>(),
        }));

        let listener = match TcpListener::bind(("127.0.0.1", REDIRECT_PORT)).await {
            Ok(listener) => listener,
            Err(err) => {
                super::emit(json!({
                    "type": "error",
                    "code": "redirect_port_busy",
                    "message": format!("Could not listen on 127.0.0.1:{REDIRECT_PORT}: {err}. Close the other program using that port and try again."),
                }));
                return;
            }
        };

        let accepted = tokio::time::timeout(Duration::from_secs(180), listener.accept()).await;
        let Ok(Ok((mut socket, _))) = accepted else {
            super::emit(json!({
                "type": "error",
                "code": "authorization_timeout",
                "message": "Spotify authorization was not completed in time. Try again when you are ready.",
            }));
            return;
        };

        let mut buffer = vec![0u8; 16 * 1024];
        let read = socket.read(&mut buffer).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..read]).to_string();
        let first_line = request.lines().next().unwrap_or("");
        let target = first_line.split_whitespace().nth(1).unwrap_or("");
        let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
        let params: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();

        let body = if params.contains_key("error") {
            "<h2>Spotify authorization was cancelled.</h2><p>You can close this tab and return to Melyar.</p>"
        } else {
            "<h2>Melyar is connected to Spotify.</h2><p>You can close this tab and return to Melyar.</p>"
        };
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.shutdown().await;

        if let Some(error) = params.get("error") {
            super::emit(json!({
                "type": "error",
                "code": "authorization_cancelled",
                "message": format!("Spotify authorization was not granted ({error})."),
            }));
            return;
        }
        if params.get("state").map(String::as_str) != Some(state.as_str()) {
            super::emit(json!({
                "type": "error",
                "code": "authorization_state_mismatch",
                "message": "The Spotify redirect did not match this request. Nothing was connected.",
            }));
            return;
        }
        let Some(code) = params.get("code").cloned() else {
            super::emit(json!({
                "type": "error",
                "code": "authorization_cancelled",
                "message": "Spotify did not return an authorization code.",
            }));
            return;
        };

        let client = http_client();
        let token = match exchange_code(&client, &client_id, &code, &verifier).await {
            Ok(token) => token,
            Err(err) => {
                super::emit(json!({
                    "type": "error",
                    "code": "token_exchange_failed",
                    "message": format!("Spotify authorization succeeded, but the token exchange failed: {err}"),
                }));
                return;
            }
        };
        if save_token(&token).is_err() {
            super::emit(json!({
                "type": "error",
                "code": "credential_storage_failed",
                "message": "The Spotify token could not be stored securely, so the connection was not kept.",
            }));
            return;
        }
        let account = fetch_account_name(&client, &token.access_token)
            .await
            .unwrap_or_default();
        let _ = crate::get_core().storage().upsert_provider_account(
            &crate::db::import_store::ProviderAccountRow {
                provider: "spotify".into(),
                account_name: account.clone(),
                connected: true,
                updated_at: now(),
            },
        );
        super::emit(json!({
            "type": "provider_connected",
            "provider": "spotify",
            "account": account,
            "state": status_json(),
        }));
    });
    true
}

pub fn disconnect() -> bool {
    let _ = clear_secret();
    let _ = crate::get_core().storage().upsert_provider_account(
        &crate::db::import_store::ProviderAccountRow {
            provider: "spotify".into(),
            account_name: String::new(),
            connected: false,
            updated_at: now(),
        },
    );
    super::emit(json!({
        "type": "provider_disconnected",
        "provider": "spotify",
        "state": status_json(),
    }));
    true
}

/// Fetch the selected Spotify resources and build an import preview. Nothing
/// is written to the library until the user confirms that preview.
pub fn preview(options_json: String) -> bool {
    let options: Value = serde_json::from_str(&options_json).unwrap_or_else(|_| json!({}));
    let include_tracks = options.get("tracks").and_then(Value::as_bool).unwrap_or(true);
    let include_albums = options.get("albums").and_then(Value::as_bool).unwrap_or(true);
    let include_playlists = options
        .get("playlists")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let playlist_filter: Vec<String> = options
        .get("playlistIds")
        .and_then(Value::as_array)
        .map(|ids| ids.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();

    let Some(client_id) = client_id() else {
        super::emit(json!({
            "type": "error",
            "code": "requires_configuration",
            "message": "Add a Spotify Client ID before importing.",
        }));
        return false;
    };
    let Some(mut token) = load_token() else {
        super::emit(json!({
            "type": "error",
            "code": "not_connected",
            "message": "Connect Spotify before importing your library.",
        }));
        return false;
    };

    super::CANCEL.store(false, std::sync::atomic::Ordering::SeqCst);
    let core = crate::get_core();
    core.runtime().spawn(async move {
        let client = http_client();
        if let Err(err) = ensure_fresh_token(&client, &client_id, &mut token).await {
            super::emit(json!({
                "type": "error",
                "code": "token_expired",
                "message": format!("The Spotify session could not be refreshed: {err}. Reconnect Spotify."),
            }));
            return;
        }

        let mut tracks: Vec<ParsedTrack> = Vec::new();
        let mut seen: std::collections::HashSet<String> = Default::default();
        let mut playlists: Vec<ParsedPlaylist> = Vec::new();
        let mut skipped = 0usize;
        let mut warnings: Vec<String> = Vec::new();

        if include_tracks {
            super::emit_progress("spotify_library", 0, 0, json!({"step": "saved tracks"}));
            match fetch_saved_tracks(&client, &client_id, &mut token).await {
                Ok(items) => {
                    for item in items {
                        if let Some(track) = spotify_track(&item) {
                            if seen.insert(track.source_id.clone()) {
                                tracks.push(track);
                            }
                        } else {
                            skipped += 1;
                        }
                    }
                }
                Err(err) => warnings.push(format!("Saved tracks: {err}")),
            }
        }

        if include_albums {
            super::emit_progress("spotify_library", 0, 0, json!({"step": "saved albums"}));
            match fetch_saved_albums(&client, &client_id, &mut token).await {
                Ok(albums) => {
                    for album in albums {
                        if let Some(items) = album.get("tracks").and_then(|t| t.get("items")) {
                            if let Some(items) = items.as_array() {
                                for item in items {
                                    if let Some(track) = spotify_track(item) {
                                        if seen.insert(track.source_id.clone()) {
                                            tracks.push(track);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(err) => warnings.push(format!("Saved albums: {err}")),
            }
        }

        if include_playlists {
            super::emit_progress("spotify_library", 0, 0, json!({"step": "playlists"}));
            match fetch_playlists(&client, &client_id, &mut token).await {
                Ok(items) => {
                    let total = items.len();
                    for (index, playlist) in items.iter().enumerate() {
                        if super::cancelled() {
                            super::emit(json!({"type": "cancelled", "phase": "spotify"}));
                            return;
                        }
                        let id = playlist.get("id").and_then(Value::as_str).unwrap_or("");
                        let name = playlist.get("name").and_then(Value::as_str).unwrap_or("Playlist");
                        if id.is_empty() {
                            continue;
                        }
                        if !playlist_filter.is_empty() && !playlist_filter.iter().any(|p| p == id) {
                            continue;
                        }
                        match fetch_playlist_tracks(&client, &client_id, &mut token, id).await {
                            Ok(items) => {
                                let mut ids = Vec::new();
                                for item in items {
                                    if let Some(track) = spotify_track(&item) {
                                        if seen.insert(track.source_id.clone()) {
                                            tracks.push(track.clone());
                                        }
                                        ids.push(track.source_id);
                                    } else {
                                        skipped += 1;
                                    }
                                }
                                if !ids.is_empty() {
                                    playlists.push(ParsedPlaylist {
                                        name: name.to_string(),
                                        track_ids: ids,
                                    });
                                }
                            }
                            Err(err) => {
                                warnings.push(format!("Playlist “{name}”: {err}"));
                            }
                        }
                        super::emit_progress(
                            "spotify_library",
                            index + 1,
                            total,
                            json!({"step": "playlists", "tracks": tracks.len()}),
                        );
                    }
                }
                Err(err) => warnings.push(format!("Playlists: {err}")),
            }
        }

        if tracks.is_empty() {
            super::emit(json!({
                "type": "error",
                "code": "no_tracks",
                "message": "Spotify returned no importable tracks for the selected permissions.",
            }));
            return;
        }

        let payload = super::preview_from_tracks(
            "spotify",
            "api",
            "Spotify Library",
            "",
            tracks,
            playlists,
            skipped,
            warnings,
        );
        super::emit(json!({"type": "preview", "preview": payload}));
    });
    true
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .user_agent("Melyar/0.0.2 (desktop library importer)")
        .build()
        .unwrap_or_default()
}

async fn exchange_code(
    client: &reqwest::Client,
    client_id: &str,
    code: &str,
    verifier: &str,
) -> Result<Token> {
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &redirect_uri()),
            ("client_id", client_id),
            ("code_verifier", verifier),
        ])
        .send()
        .await?;
    token_from_response(response).await
}

async fn refresh_token(client: &reqwest::Client, client_id: &str, token: &Token) -> Result<Token> {
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", token.refresh_token.as_str()),
            ("client_id", client_id),
        ])
        .send()
        .await?;
    let mut refreshed = token_from_response(response).await?;
    if refreshed.refresh_token.is_empty() {
        refreshed.refresh_token = token.refresh_token.clone();
    }
    Ok(refreshed)
}

async fn token_from_response(response: reqwest::Response) -> Result<Token> {
    let status = response.status();
    let body: Value = response.json().await.unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        let message = body
            .get("error_description")
            .or_else(|| body.get("error"))
            .and_then(Value::as_str)
            .unwrap_or("Spotify rejected the token request");
        return Err(anyhow!(message.to_string()));
    }
    let access = body
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Spotify returned no access token"))?;
    let expires_in = body.get("expires_in").and_then(Value::as_i64).unwrap_or(3600);
    Ok(Token {
        access_token: access.to_string(),
        refresh_token: body
            .get("refresh_token")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        expires_at: now() + expires_in,
    })
}

async fn ensure_fresh_token(
    client: &reqwest::Client,
    client_id: &str,
    token: &mut Token,
) -> Result<()> {
    if token.access_token.is_empty() {
        return Err(anyhow!("no access token"));
    }
    if token.expires_at > now() + 90 {
        return Ok(());
    }
    if token.refresh_token.is_empty() {
        return Err(anyhow!("the access token expired and no refresh token is stored"));
    }
    *token = refresh_token(client, client_id, token).await?;
    save_token(token)?;
    Ok(())
}

async fn api_get(
    client: &reqwest::Client,
    client_id: &str,
    token: &mut Token,
    url: &str,
) -> Result<Value> {
    for attempt in 0..4 {
        if token.expires_at <= now() + 30 && !token.refresh_token.is_empty() {
            *token = refresh_token(client, client_id, token).await?;
            save_token(token)?;
        }
        let response = client
            .get(url)
            .bearer_auth(&token.access_token)
            .header("Accept", "application/json")
            .send()
            .await?;

        match response.status().as_u16() {
            200 => return Ok(response.json().await.unwrap_or_else(|_| json!({}))),
            401 if attempt == 0 => {
                if token.refresh_token.is_empty() {
                    return Err(anyhow!("Spotify says the access token is invalid; reconnect the account"));
                }
                *token = refresh_token(client, client_id, token).await?;
                save_token(token)?;
                continue;
            }
            429 => {
                let wait = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(2)
                    .min(30);
                super::emit_progress(
                    "spotify_rate_limit",
                    0,
                    0,
                    json!({"retryAfter": wait}),
                );
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            500..=599 if attempt < 3 => {
                tokio::time::sleep(Duration::from_secs(1 + attempt as u64 * 2)).await;
                continue;
            }
            status => {
                let body: Value = response.json().await.unwrap_or_else(|_| json!({}));
                let message = body
                    .get("error")
                    .and_then(|e| e.get("message").or(Some(e)))
                    .and_then(Value::as_str)
                    .unwrap_or("Spotify request failed");
                return Err(anyhow!("HTTP {status}: {message}"));
            }
        }
    }
    Err(anyhow!("Spotify request failed after retries"))
}

async fn paged(
    client: &reqwest::Client,
    client_id: &str,
    token: &mut Token,
    mut url: String,
) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let mut pages = 0;
    while !url.is_empty() && pages < 200 {
        if super::cancelled() {
            break;
        }
        let value = api_get(client, client_id, token, &url).await?;
        if let Some(items) = value.get("items").and_then(Value::as_array) {
            out.extend(items.iter().cloned());
        }
        url = value
            .get("next")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        pages += 1;
        if pages % 3 == 0 {
            super::emit_progress("spotify_fetch", out.len(), 0, json!({}));
        }
    }
    Ok(out)
}

async fn fetch_account_name(client: &reqwest::Client, access_token: &str) -> Result<String> {
    let value: Value = client
        .get(format!("{API_BASE}/me"))
        .bearer_auth(access_token)
        .send()
        .await?
        .json()
        .await?;
    Ok(value
        .get("display_name")
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("Spotify account")
        .to_string())
}

async fn fetch_saved_tracks(
    client: &reqwest::Client,
    client_id: &str,
    token: &mut Token,
) -> Result<Vec<Value>> {
    paged(
        client,
        client_id,
        token,
        format!("{API_BASE}/me/tracks?limit=50&offset=0"),
    )
    .await
}

async fn fetch_saved_albums(
    client: &reqwest::Client,
    client_id: &str,
    token: &mut Token,
) -> Result<Vec<Value>> {
    let items = paged(
        client,
        client_id,
        token,
        format!("{API_BASE}/me/albums?limit=50&offset=0"),
    )
    .await?;
    Ok(items
        .into_iter()
        .filter_map(|item| item.get("album").cloned())
        .collect())
}

async fn fetch_playlists(
    client: &reqwest::Client,
    client_id: &str,
    token: &mut Token,
) -> Result<Vec<Value>> {
    paged(
        client,
        client_id,
        token,
        format!("{API_BASE}/me/playlists?limit=50&offset=0"),
    )
    .await
}

async fn fetch_playlist_tracks(
    client: &reqwest::Client,
    client_id: &str,
    token: &mut Token,
    playlist_id: &str,
) -> Result<Vec<Value>> {
    paged(
        client,
        client_id,
        token,
        format!(
            "{API_BASE}/playlists/{}/tracks?limit=100&offset=0&additional_types=track",
            urlencode(playlist_id)
        ),
    )
    .await
}

fn spotify_track(value: &Value) -> Option<ParsedTrack> {
    let track = value.get("track").unwrap_or(value);
    if track.is_null() {
        return None;
    }
    if track.get("type").and_then(Value::as_str) == Some("episode") {
        return None;
    }
    let id = track.get("id").and_then(Value::as_str)?;
    if id.is_empty() {
        return None;
    }
    let title = track.get("name").and_then(Value::as_str)?.to_string();
    let artist = track
        .get("artists")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|a| a.get("name").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown artist".into());
    let album_object = track.get("album");
    let album = album_object
        .and_then(|a| a.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown album")
        .to_string();
    let thumbnail = album_object
        .and_then(|a| a.get("images"))
        .and_then(Value::as_array)
        .and_then(|images| images.first())
        .and_then(|i| i.get("url"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let duration_secs = track
        .get("duration_ms")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        / 1000.0;
    let isrc = track
        .get("external_ids")
        .and_then(|e| e.get("isrc"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let external_url = track
        .get("external_urls")
        .and_then(|e| e.get("spotify"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Some(ParsedTrack {
        source: "spotify".into(),
        source_id: id.to_string(),
        title,
        artist,
        album,
        album_artist: album_object
            .and_then(|a| a.get("artists"))
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(|a| a.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        duration_secs,
        track_no: track
            .get("track_number")
            .and_then(Value::as_u64)
            .unwrap_or(0) as i32,
        disc_no: track
            .get("disc_number")
            .and_then(Value::as_u64)
            .unwrap_or(0) as i32,
        year: String::new(),
        isrc,
        external_url,
        thumbnail,
        file_path: String::new(),
    })
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

fn credentials_path() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("melyar").join("credentials").join("spotify.json"))
}

fn secret_tool_available() -> bool {
    std::process::Command::new("secret-tool")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn load_token() -> Option<Token> {
    if secret_tool_available() {
        if let Ok(output) = std::process::Command::new("secret-tool")
            .args(["lookup", "service", SECRET_SERVICE, "provider", SECRET_ACCOUNT])
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                if let Ok(token) = serde_json::from_slice::<Token>(&output.stdout) {
                    return Some(token);
                }
            }
        }
    }
    let path = credentials_path()?;
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_token(token: &Token) -> Result<()> {
    let json = serde_json::to_vec(token)?;
    if secret_tool_available() {
        use std::process::{Command, Stdio};
        let mut child = Command::new("secret-tool")
            .args([
                "store",
                "--label=Melyar Spotify",
                "service",
                SECRET_SERVICE,
                "provider",
                SECRET_ACCOUNT,
            ])
            .stdin(Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&json)?;
        }
        if child.wait()?.success() {
            return Ok(());
        }
    }
    // Fallback: a file only the user can read. This is the documented
    // behaviour when no secret service is running.
    let path = credentials_path().ok_or_else(|| anyhow!("no data directory"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(&json)?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&path, &json)?;
    }
    Ok(())
}

fn clear_secret() -> Result<()> {
    if secret_tool_available() {
        let _ = std::process::Command::new("secret-tool")
            .args(["clear", "service", SECRET_SERVICE, "provider", SECRET_ACCOUNT])
            .output();
    }
    if let Some(path) = credentials_path() {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn urlencode(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}

/// SHA-256, needed for the PKCE code challenge. Kept local so the importer
/// does not pull a crypto dependency into the audio core.
fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (input.len() as u64) * 8;
    let mut data = input.to_vec();
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in data.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks(4).take(16).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, value) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_standard_vectors() {
        let digest = sha256(b"abc");
        assert_eq!(
            hex::encode(digest),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let empty = sha256(b"");
        assert_eq!(
            hex::encode(empty),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn pkce_challenge_is_base64url() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = URL_SAFE_NO_PAD.encode(sha256(verifier.as_bytes()));
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn spotify_track_keeps_isrc_and_order_fields() {
        let value = json!({
            "id": "abc",
            "name": "Song",
            "artists": [{"name": "Artist"}],
            "album": {"name": "Album", "images": [{"url": "https://img"}], "artists": [{"name": "Artist"}]},
            "duration_ms": 213000,
            "external_ids": {"isrc": "GB123"},
            "external_urls": {"spotify": "https://open.spotify.com/track/abc"},
            "track_number": 3,
            "disc_number": 1
        });
        let track = spotify_track(&value).unwrap();
        assert_eq!(track.title, "Song");
        assert_eq!(track.artist, "Artist");
        assert!((track.duration_secs - 213.0).abs() < 0.01);
        assert_eq!(track.isrc, "GB123");
        assert_eq!(track.track_no, 3);
    }
}
