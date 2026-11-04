//! Spotify account binding.
//!
//! Reuses Spotify's OAuth 2.0 Authorization Code + PKCE flow. After
//! connecting, the user's saved tracks and playlists are fetched and written
//! into Melyar's local library (favorites + library playlists).

use crate::accounts::{emit, set_setting, setting};
use crate::db::storage::DbStorage;
use crate::ffi::CxxTrackMetadata;
use crate::ffi::CxxLibraryPlaylist;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpListener;

const AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const API_BASE: &str = "https://api.spotify.com/v1";
const SCOPES: &str = "user-library-read playlist-read-private playlist-read-collaborative";
const REDIRECT_PORT: u16 = 43821;
const REDIRECT_PATH: &str = "/callback";
const CLIENT_ID_KEY: &str = "spotify.clientId";
const TOKEN_KEY: &str = "spotify.token";

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
    let user = setting(storage, "spotify.user").unwrap_or_default();
    crate::accounts::AccountStatus {
        id: "spotify",
        name: "Spotify",
        connected,
        user,
        requires_config: client_id(storage).is_none(),
        config_hint: "Create a Spotify app at developer.spotify.com, add http://127.0.0.1:43821/callback as a redirect URI, then paste the Client ID here. No client secret is used (PKCE).".to_string(),
    }
}

pub fn connect(storage: Arc<DbStorage>, runtime: &Arc<tokio::runtime::Runtime>) -> bool {
    let Some(cid) = client_id(&storage) else {
        emit(json!({
            "type": "error",
            "code": "requires_configuration",
            "message": "Add a Spotify Client ID before connecting.",
        }));
        return false;
    };

    // PKCE.
    let verifier = random_string(64);
    let challenge = base64_url(sha256(verifier.as_bytes()));
    let state = random_string(24);

    let auth_url = format!(
        "{AUTH_URL}?response_type=code&client_id={}&scope={}&redirect_uri={}&state={}&code_challenge_method=S256&code_challenge={}",
        urlencode(&cid),
        urlencode(SCOPES),
        urlencode(&redirect_uri()),
        urlencode(&state),
        urlencode(&challenge),
    );

    let storage_clone = storage.clone();
    let state_clone = state.clone();
    let verifier_clone = verifier.clone();
    let runtime_clone = runtime.clone();

    runtime.spawn(async move {
        emit(json!({
            "type": "account_auth_url",
            "provider": "spotify",
            "url": auth_url,
            "redirectUri": redirect_uri(),
        }));

        let listener = match TcpListener::bind(("127.0.0.1", REDIRECT_PORT)).await {
            Ok(l) => l,
            Err(err) => {
                emit(json!({"type":"error","code":"redirect_port_busy","message":format!("Could not listen on 127.0.0.1:{REDIRECT_PORT}: {err}.")}));
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

        // Exchange code.
        let client = reqwest::Client::new();
        let ru = redirect_uri();
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", ru.as_str()),
            ("client_id", cid.as_str()),
            ("code_verifier", verifier_clone.as_str()),
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

        // Fetch account name.
        if let Ok(name) = fetch_account_name(&client, &token.access_token).await {
            set_setting(&storage_clone, "spotify.user", &name);
        }

        emit(json!({"type":"provider_connected","provider":"spotify"}));
    });

    true
}

pub fn disconnect(storage: &DbStorage) -> bool {
    clear_token(&storage);
    let _ = storage.delete_setting("spotify.user");
    emit(json!({"type":"provider_disconnected","provider":"spotify"}));
    true
}

pub fn import(storage: Arc<DbStorage>, runtime: &Arc<tokio::runtime::Runtime>, options_json: &str) -> bool {
    let Some(token) = load_token(&storage) else {
        emit(json!({"type":"error","code":"not_connected","message":"Connect Spotify first."}));
        return false;
    };
    let storage_clone = storage.clone();
    let runtime_clone = runtime.clone();
    let _options = options_json.to_string();

    runtime.spawn(async move {
        emit(json!({"type":"progress","phase":"spotify_library","current":0,"total":0,"done":false,"detail":{"step":"playlists"}}));

        let client = reqwest::Client::new();
        let access_token = token.access_token.clone();
        let cid = client_id(&storage_clone).unwrap_or_default();

        // Refresh token if needed (best effort).
        let mut token = token.clone();

        // Fetch playlists.
        let playlists = match fetch_playlists(&client, &cid, &mut token).await {
            Ok(v) => v,
            Err(err) => {
                emit(json!({"type":"error","code":"playlists_failed","message":err.to_string()}));
                return;
            }
        };
        save_token(&storage_clone, &token);

        let total_pl = playlists.len();
        let mut imported_pl = 0usize;
        let mut total_tracks = 0usize;

        for (idx, pl) in playlists.iter().enumerate() {
            let pl_id = pl.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let pl_name = pl.get("name").and_then(Value::as_str).unwrap_or("Playlist").to_string();
            let pl_thumb = pl.get("images").and_then(Value::as_array).and_then(|a| a.first()).and_then(|i| i.get("url")).and_then(Value::as_str).unwrap_or("").to_string();

            emit(json!({"type":"progress","phase":"spotify_library","current":idx+1,"total":total_pl,"done":false,"detail":{"step":"playlist","name":pl_name}}));

            let tracks = match fetch_playlist_tracks(&client, &cid, &mut token, &pl_id).await {
                Ok(v) => v,
                Err(_) => continue,
            };

            for t in &tracks {
                let track = match spotify_track(t) {
                    Some(x) => x,
                    None => continue,
                };
                let meta = CxxTrackMetadata {
                    id: track.track_id.clone(),
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                    album: track.album.clone(),
                    duration_secs: track.duration as f64,
                    thumbnail_url: track.thumbnail.clone(),
                    lyrics: String::new(),
                };
                let _ = storage_clone.add_favorite(&meta);
                total_tracks += 1;
            }

            let lp = CxxLibraryPlaylist {
                id: format!("sp:{}", pl_id),
                name: pl_name,
                thumbnail_url: pl_thumb,
                track_count: tracks.len() as i32,
                is_owner: false,
            };
            let _ = storage_clone.add_library_playlist(&lp);
            imported_pl += 1;
        }

        // Saved tracks.
        if let Ok(saved) = fetch_saved_tracks(&client, &cid, &mut token).await {
            save_token(&storage_clone, &token);
            for t in &saved {
                if let Some(track) = spotify_track(t) {
                    let meta = CxxTrackMetadata {
                        id: track.track_id,
                        title: track.title,
                        artist: track.artist,
                        album: track.album,
                        duration_secs: track.duration as f64,
                        thumbnail_url: track.thumbnail,
                        lyrics: String::new(),
                    };
                    let _ = storage_clone.add_favorite(&meta);
                    total_tracks += 1;
                }
            }
        }

        emit(json!({
            "type": "done",
            "phase": "spotify_library",
            "done": true,
            "summary": {
                "playlists": imported_pl,
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
    let mut out = String::new();
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

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


fn base64_url(input: [u8; 32]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input)
}

async fn wait_for_callback(listener: &TcpListener) -> Result<(String, String)> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (mut socket, _) = listener.accept().await.map_err(|e| anyhow!(e))?;
    let mut buf = vec![0u8; 4096];
    let n = socket.read(&mut buf).await.map_err(|e| anyhow!(e))?;
    let req = String::from_utf8_lossy(&buf[..n]);
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
    let body = "<html><body>You can close this window.</body></html>";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = socket.write_all(resp.as_bytes()).await;
    Ok((code, state))
}

async fn fetch_account_name(client: &reqwest::Client, access_token: &str) -> Result<String> {
    let resp = client
        .get(format!("{}/me", API_BASE))
        .bearer_auth(access_token)
        .send()
        .await?;
    let body: Value = resp.json().await?;
    Ok(body.get("display_name").and_then(Value::as_str).unwrap_or("").to_string())
}

async fn ensure_fresh_token(client: &reqwest::Client, client_id: &str, token: &mut Token) -> Result<()> {
    if token.expires_at <= now() + 30 {
        // refresh
        if token.refresh_token.is_empty() {
            return Err(anyhow!("no refresh token"));
        }
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", token.refresh_token.as_str()),
            ("client_id", client_id),
        ];
        let resp = client.post(TOKEN_URL).form(&params).send().await?;
        let body: Value = resp.json().await?;
        if let Some(at) = body.get("access_token").and_then(Value::as_str) {
            token.access_token = at.to_string();
        }
        if let Some(ei) = body.get("expires_in").and_then(Value::as_i64) {
            token.expires_at = now() + ei;
        }
        if let Some(rt) = body.get("refresh_token").and_then(Value::as_str) {
            if !rt.is_empty() {
                token.refresh_token = rt.to_string();
            }
        }
    }
    Ok(())
}

async fn api_get(client: &reqwest::Client, client_id: &str, token: &mut Token, url: &str) -> Result<Value> {
    ensure_fresh_token(client, client_id, token).await?;
    let resp = client.get(url).bearer_auth(&token.access_token).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("spotify api {} failed: {}", url, resp.status()));
    }
    Ok(resp.json().await?)
}

async fn paged(client: &reqwest::Client, client_id: &str, token: &mut Token, mut url: String) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    loop {
        let body = api_get(client, client_id, token, &url).await?;
        if let Some(items) = body.get("items").and_then(Value::as_array) {
            out.extend(items.clone());
        }
        match body.get("next").and_then(Value::as_str) {
            Some(next) if !next.is_empty() => url = next.to_string(),
            _ => break,
        }
    }
    Ok(out)
}

async fn fetch_playlists(client: &reqwest::Client, client_id: &str, token: &mut Token) -> Result<Vec<Value>> {
    paged(client, client_id, token, format!("{API_BASE}/me/playlists?limit=50&offset=0")).await
}

async fn fetch_playlist_tracks(client: &reqwest::Client, client_id: &str, token: &mut Token, playlist_id: &str) -> Result<Vec<Value>> {
    paged(client, client_id, token, format!("{API_BASE}/playlists/{}/tracks?limit=100&offset=0&additional_types=track", urlencode(playlist_id))).await
}

async fn fetch_saved_tracks(client: &reqwest::Client, client_id: &str, token: &mut Token) -> Result<Vec<Value>> {
    paged(client, client_id, token, format!("{API_BASE}/me/tracks?limit=50&offset=0")).await
}

fn spotify_track(value: &Value) -> Option<SpotifyTrack> {
    let track = value.get("track").unwrap_or(value);
    if track.is_null() {
        return None;
    }
    let id = track.get("id").and_then(Value::as_str)?.to_string();
    let title = track.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let artist = track
        .get("artists")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(|a| a.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let album = track.get("album").and_then(|a| a.get("name")).and_then(Value::as_str).unwrap_or("").to_string();
    let duration = track.get("duration_ms").and_then(Value::as_i64).unwrap_or(0) as i64 / 1000;
    let thumbnail = track
        .get("album")
        .and_then(|a| a.get("images"))
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(|i| i.get("url"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Some(SpotifyTrack {
        track_id: format!("spotify:{}", id),
        title,
        artist,
        album,
        duration,
        thumbnail,
    })
}

struct SpotifyTrack {
    track_id: String,
    title: String,
    artist: String,
    album: String,
    duration: i64,
    thumbnail: String,
}
