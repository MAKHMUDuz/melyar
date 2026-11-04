//! Google / YouTube account binding.
//!
//! Uses Google OAuth 2.0 (loopback redirect) with the `youtube.readonly`
//! scope, then the YouTube Data API v3 to read the user's playlists and
//! playlist items. There is no official YouTube Music API, so the YouTube
//! Data API is the supported surface; playlist items map to video ids that
//! Melyar resolves through its existing YouTube source.

use crate::accounts::{emit, set_setting, setting};
use crate::db::storage::DbStorage;
use crate::ffi::CxxTrackMetadata;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const API_BASE: &str = "https://www.googleapis.com/youtube/v3";
const SCOPES: &str = "https://www.googleapis.com/auth/youtube.readonly";
const REDIRECT_PORT: u16 = 43822;
const REDIRECT_PATH: &str = "/callback";
const CLIENT_ID_KEY: &str = "google.clientId";
const TOKEN_KEY: &str = "google.token";

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct Token {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn redirect_uri() -> String {
    format!("http://127.0.0.1:{REDIRECT_PORT}{REDIRECT_PATH}")
}

fn client_id(storage: &DbStorage) -> Option<String> {
    setting(storage, CLIENT_ID_KEY)
}

fn load_token(storage: &DbStorage) -> Option<Token> {
    let raw = storage.get_setting(TOKEN_KEY).ok().flatten()?;
    serde_json::from_str(&raw).ok()
}

fn save_token(storage: &DbStorage, token: &Token) {
    if let Ok(s) = serde_json::to_string(token) {
        set_setting(storage, TOKEN_KEY, &s);
    }
}

fn clear_token(storage: &DbStorage) {
    let _ = storage.delete_setting(TOKEN_KEY);
}

pub fn status(storage: &DbStorage) -> crate::accounts::AccountStatus {
    let connected = load_token(storage).is_some();
    let user = if connected {
        // Best effort: store the channel title when importing.
        setting(storage, "google.user").unwrap_or_default()
    } else {
        String::new()
    };
    crate::accounts::AccountStatus {
        id: "google",
        name: "Google",
        connected,
        user,
        requires_config: client_id(storage).is_none(),
        config_hint: "Create an OAuth 2.0 Client ID (Desktop app type) in Google Cloud Console, enable the YouTube Data API v3, then paste the Client ID here. Scope: youtube.readonly.".to_string(),
    }
}

pub fn connect(storage: Arc<DbStorage>, runtime: &Arc<tokio::runtime::Runtime>) -> bool {
    let Some(cid) = client_id(&storage) else {
        emit(json!({
            "type": "error",
            "code": "requires_configuration",
            "message": "Add a Google OAuth Client ID before connecting.",
        }));
        return false;
    };

    let state = random_string(24);
    let auth_url = format!(
        "{AUTH_URL}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&access_type=offline&prompt=consent",
        urlencode(&cid),
        urlencode(&redirect_uri()),
        urlencode(SCOPES),
        urlencode(&state),
    );

    let storage_clone = storage.clone();
    let runtime_clone = runtime.clone();
    let state_clone = state.clone();

    runtime.spawn(async move {
        emit(json!({
            "type": "account_auth_url",
            "provider": "google",
            "url": auth_url,
            "redirectUri": redirect_uri(),
        }));

        // Bind loopback listener.
        let listener = match TcpListener::bind(("127.0.0.1", REDIRECT_PORT)).await {
            Ok(l) => l,
            Err(err) => {
                emit(json!({
                    "type": "error",
                    "code": "redirect_port_busy",
                    "message": format!("Could not listen on 127.0.0.1:{REDIRECT_PORT}: {err}."),
                }));
                return;
            }
        };

        // Wait for callback.
        let (code, returned_state) = match wait_for_callback(&listener).await {
            Ok(v) => v,
            Err(err) => {
                emit(json!({"type":"error","code":"callback_failed","message":err.to_string()}));
                return;
            }
        };

        if returned_state != state_clone {
            emit(json!({"type":"error","code":"state_mismatch","message":"OAuth state mismatch."}));
            return;
        }

        // Exchange code for token.
        let client = reqwest::Client::new();
        let params = [
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri()),
            ("client_id", &cid),
        ];
        let resp = match client.post(TOKEN_URL).form(&params).send().await {
            Ok(r) => r,
            Err(err) => {
                emit(json!({"type":"error","code":"token_exchange_failed","message":err.to_string()}));
                return;
            }
        };
        let body: Value = match resp.json().await {
            Ok(v) => v,
            Err(err) => {
                emit(json!({"type":"error","code":"token_parse_failed","message":err.to_string()}));
                return;
            }
        };
        let access_token = body.get("access_token").and_then(Value::as_str).unwrap_or("").to_string();
        let refresh_token = body.get("refresh_token").and_then(Value::as_str).unwrap_or("").to_string();
        let expires_in = body.get("expires_in").and_then(Value::as_i64).unwrap_or(3600);
        if access_token.is_empty() {
            emit(json!({"type":"error","code":"no_access_token","message":body.to_string()}));
            return;
        }
        let token = Token {
            access_token,
            refresh_token,
            expires_at: now() + expires_in,
        };
        save_token(&storage_clone, &token);

        // Fetch channel title for display.
        if let Ok(name) = fetch_channel_title(&client, &token.access_token).await {
            set_setting(&storage_clone, "google.user", &name);
        }

        emit(json!({"type":"provider_connected","provider":"google"}));
    });

    true
}

pub fn disconnect(storage: &DbStorage) -> bool {
    clear_token(storage);
    let _ = storage.delete_setting("google.user");
    emit(json!({"type":"provider_disconnected","provider":"google"}));
    true
}

pub fn import(storage: Arc<DbStorage>, runtime: &Arc<tokio::runtime::Runtime>, _innertube: &crate::innertube::client::InnerTubeClient, options_json: &str) -> bool {
    let Some(token) = load_token(&storage) else {
        emit(json!({"type":"error","code":"not_connected","message":"Connect Google first."}));
        return false;
    };
    let storage_clone = storage.clone();
    let runtime_clone = runtime.clone();
    let _options = options_json.to_string();

    runtime.spawn(async move {
        emit(json!({"type":"progress","phase":"google_library","current":0,"total":0,"done":false,"detail":{"step":"playlists"}}));

        let client = reqwest::Client::new();
        let access_token = token.access_token.clone();

        // Fetch playlists.
        let mut playlists: Vec<Value> = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut url = format!("{}/playlists?part=snippet&mine=true&maxResults=50", API_BASE);
            if let Some(pt) = &page_token {
                url.push_str(&format!("&pageToken={}", pt));
            }
            let resp = match client.get(&url).bearer_auth(&access_token).send().await {
                Ok(r) => r,
                Err(err) => {
                    emit(json!({"type":"error","code":"playlists_failed","message":err.to_string()}));
                    return;
                }
            };
            let body: Value = match resp.json().await {
                Ok(v) => v,
                Err(err) => {
                    emit(json!({"type":"error","code":"playlists_parse_failed","message":err.to_string()}));
                    return;
                }
            };
            if let Some(items) = body.get("items").and_then(Value::as_array) {
                playlists.extend(items.clone());
            }
            page_token = body.get("nextPageToken").and_then(Value::as_str).map(|s| s.to_string());
            if page_token.is_none() {
                break;
            }
        }

        let total_playlists = playlists.len();
        let mut imported_playlists = 0usize;
        let mut total_tracks = 0usize;

        for (idx, pl) in playlists.iter().enumerate() {
            let pl_id = pl.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let pl_title = pl.get("snippet").and_then(|s| s.get("title")).and_then(Value::as_str).unwrap_or("Playlist").to_string();
            let pl_thumb = pl.get("snippet").and_then(|s| s.get("thumbnails")).and_then(|t| t.get("default")).and_then(|d| d.get("url")).and_then(Value::as_str).unwrap_or("").to_string();

            emit(json!({"type":"progress","phase":"google_library","current":idx+1,"total":total_playlists,"done":false,"detail":{"step":"playlist","name":pl_title}}));

            // Fetch items.
            let mut items: Vec<Value> = Vec::new();
            let mut page_token: Option<String> = None;
            loop {
                let mut url = format!("{}/playlistItems?part=snippet&playlistId={}&maxResults=50", API_BASE, pl_id);
                if let Some(pt) = &page_token {
                    url.push_str(&format!("&pageToken={}", pt));
                }
                let resp = match client.get(&url).bearer_auth(&access_token).send().await {
                    Ok(r) => r,
                    Err(_) => break,
                };
                let body: Value = match resp.json().await {
                    Ok(v) => v,
                    Err(_) => break,
                };
                if let Some(arr) = body.get("items").and_then(Value::as_array) {
                    items.extend(arr.clone());
                }
                page_token = body.get("nextPageToken").and_then(Value::as_str).map(|s| s.to_string());
                if page_token.is_none() {
                    break;
                }
            }

            // Build track list and add to library playlist.
            let mut track_ids: Vec<String> = Vec::new();
            for it in &items {
                let snippet = it.get("snippet").cloned().unwrap_or(Value::Null);
                let title = snippet.get("title").and_then(Value::as_str).unwrap_or("").to_string();
                let artist = snippet.get("videoOwnerChannelTitle").and_then(Value::as_str).unwrap_or("").to_string();
                let thumb = snippet.get("thumbnails").and_then(|t| t.get("default")).and_then(|d| d.get("url")).and_then(Value::as_str).unwrap_or("").to_string();
                let video_id = snippet.get("resourceId").and_then(|r| r.get("videoId")).and_then(Value::as_str).unwrap_or("").to_string();
                if video_id.is_empty() {
                    continue;
                }
                let track = CxxTrackMetadata {
                    id: video_id.clone(),
                    title,
                    artist,
                    album: pl_title.clone(),
                    duration_secs: 0.0,
                    thumbnail_url: thumb,
                    lyrics: String::new(),
                };
                let _ = storage.add_favorite(&track);
                track_ids.push(video_id);
                total_tracks += 1;
            }

            // Create a library playlist entry.
            use crate::ffi::CxxLibraryPlaylist;
            let lp = CxxLibraryPlaylist {
                id: format!("yt:{}", pl_id),
                name: pl_title,
                thumbnail_url: pl_thumb,
                track_count: track_ids.len() as i32,
                is_owner: false,
            };
            let _ = storage.add_library_playlist(&lp);
            imported_playlists += 1;
        }

        emit(json!({
            "type": "done",
            "phase": "google_library",
            "done": true,
            "summary": {
                "playlists": imported_playlists,
                "tracks": total_tracks,
            },
        }));
    });

    true
}

// ---- helpers ----

fn random_string(len: usize) -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..len).map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char).collect()
}

fn urlencode(input: &str) -> String {
    // Minimal percent-encoding for query params.
    let mut out = String::new();
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

async fn wait_for_callback(listener: &TcpListener) -> Result<(String, String)> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (mut socket, _) = listener.accept().await.map_err(|e| anyhow!(e))?;
    let mut buf = vec![0u8; 4096];
    let n = socket.read(&mut buf).await.map_err(|e| anyhow!(e))?;
    let req = String::from_utf8_lossy(&buf[..n]);
    // Parse "GET /callback?code=...&state=... HTTP/1.1"
    let mut code = String::new();
    let mut state = String::new();
    if let Some(line) = req.lines().next() {
        if let Some(path) = line.split_whitespace().nth(1) {
            if let Some(q) = path.split('?').nth(1) {
                for pair in q.split('&') {
                    let mut kv = pair.splitn(2, '=');
                    let k = kv.next().unwrap_or("");
                    let v = kv.next().unwrap_or("");
                    let v = urlencoding::decode(v).unwrap_or_default().to_string();
                    match k {
                        "code" => code = v,
                        "state" => state = v,
                        _ => {}
                    }
                }
            }
        }
    }
    // Respond.
    let body = "<html><body>You can close this window.</body></html>";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = socket.write_all(resp.as_bytes()).await;
    Ok((code, state))
}

async fn fetch_channel_title(client: &reqwest::Client, access_token: &str) -> Result<String> {
    let resp = client
        .get("https://www.googleapis.com/youtube/v3/channels?part=snippet&mine=true")
        .bearer_auth(access_token)
        .send()
        .await?;
    let body: Value = resp.json().await?;
    let title = body
        .get("items")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(|i| i.get("snippet"))
        .and_then(|s| s.get("title"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Ok(title)
}
