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
    let verifier = random_string(64);
    let challenge = base64_url(sha256(verifier.as_bytes()));
    let auth_url = format!(
        "{AUTH_URL}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&access_type=offline&prompt=consent&code_challenge_method=S256&code_challenge={}",
        urlencode(&cid),
        urlencode(&redirect_uri()),
        urlencode(SCOPES),
        urlencode(&state),
        urlencode(&challenge),
    );

    let storage_clone = storage.clone();
    let runtime_clone = runtime.clone();
    let state_clone = state.clone();
    let verifier_clone = verifier.clone();
    let client_secret = setting(&storage, "google.clientSecret").unwrap_or_default();

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
        let redirect = redirect_uri();
        let mut params: Vec<(String, String)> = vec![
            ("grant_type".to_string(), "authorization_code".to_string()),
            ("code".to_string(), code.clone()),
            ("redirect_uri".to_string(), redirect.clone()),
            ("client_id".to_string(), cid.clone()),
            ("code_verifier".to_string(), verifier_clone.clone()),
        ];
        if !client_secret.is_empty() {
            params.push(("client_secret".to_string(), client_secret.clone()));
        }
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

pub fn import(
    storage: Arc<DbStorage>,
    runtime: &Arc<tokio::runtime::Runtime>,
    _innertube: &crate::innertube::client::InnerTubeClient,
    options_json: &str,
) -> bool {
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

            // Build track list and write them into the import store so they
            // show up in the Playlists view and stay playable (real video ids).
            let mut track_ids: Vec<String> = Vec::new();
            use crate::db::import_store::ImportItemRow;
            for it in &items {
                let snippet = it.get("snippet").cloned().unwrap_or(Value::Null);
                let title = snippet.get("title").and_then(Value::as_str).unwrap_or("").to_string();
                let artist = snippet.get("videoOwnerChannelTitle").and_then(Value::as_str).unwrap_or("").to_string();
                let thumb = snippet.get("thumbnails").and_then(|t| t.get("default")).and_then(|d| d.get("url")).and_then(Value::as_str).unwrap_or("").to_string();
                let video_id = snippet.get("resourceId").and_then(|r| r.get("videoId")).and_then(Value::as_str).unwrap_or("").to_string();
                if video_id.is_empty() {
                    continue;
                }
                let item_id = format!("google:{}", video_id);
                let row = ImportItemRow {
                    item_id: item_id.clone(),
                    track_id: video_id.clone(),
                    source: "google".into(),
                    source_id: video_id.clone(),
                    title: title.clone(),
                    artist: artist.clone(),
                    album: pl_title.clone(),
                    album_artist: artist.clone(),
                    duration_secs: 0.0,
                    thumbnail: thumb.clone(),
                    isrc: String::new(),
                    external_url: String::new(),
                    file_path: String::new(),
                    track_no: 0,
                    disc_no: 0,
                    year: String::new(),
                    availability: "provider".into(),
                    match_status: "exact".into(),
                    match_confidence: 100,
                    file_size: 0,
                    file_mtime: 0,
                    imported_at: now_ts(),
                };
                let _ = storage_clone.upsert_import_item(&row);
                track_ids.push(item_id);
                total_tracks += 1;
            }

            // Create a library playlist entry in the import store.
            use crate::db::import_store::ImportPlaylistRow;
            let playlist_id = format!("google:{}", pl_id);
            let row = ImportPlaylistRow {
                playlist_id: playlist_id.clone(),
                source: "google".into(),
                name: pl_title.clone(),
                thumbnail: pl_thumb.clone(),
                track_count: track_ids.len() as i64,
                imported_at: now_ts(),
            };
            let _ = storage_clone.upsert_import_playlist(&row);
            let _ = storage_clone.replace_import_playlist_tracks(&playlist_id, &track_ids);
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

fn sha256(input: &[u8]) -> [u8; 32] {
    // Reuse the same implementation as accounts/spotify (duplicated to keep
    // this module self-contained).
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
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
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
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

fn base64_url(input: [u8; 32]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input)
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn random_string(len: usize) -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

fn urlencode(input: &str) -> String {
    // Minimal percent-encoding for query params.
    let mut out = String::new();
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
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
