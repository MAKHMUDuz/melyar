use super::error::InnerTubeError;
use reqwest::Client;
use serde::Deserialize;
use std::sync::OnceLock;

/// A lyrics lookup, or the reason the track has none.
type Result<T> = std::result::Result<T, InnerTubeError>;

/// One shared client for every lyrics request, with a deadline.
///
/// Each call used to build its own client, which sets no total timeout, and the
/// C++ side runs this from the shared QtConcurrent pool. A server that accepted
/// the connection and then went quiet parked a pool thread for good; a handful
/// of those exhaust the pool and search, playback and library loading all stop
/// together. The InnerTube client has carried a 15 s deadline all along — this
/// one now has one too.
pub fn lyrics_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default()
    })
}

#[derive(Deserialize, Debug)]
pub struct LrclibResponse {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub track_name: Option<String>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
}

pub async fn fetch_lyrics(
    client: &Client,
    track_title: &str,
    artist: &str,
    duration_secs: Option<f64>,
) -> Result<String> {
    // 1. Try direct exact match with duration
    let mut get_url = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}",
        urlencoding::encode(track_title),
        urlencoding::encode(artist)
    );
    if let Some(dur) = duration_secs {
        if dur > 0.0 {
            get_url.push_str(&format!("&duration={}", dur.round() as i64));
        }
    }

    if let Ok(resp) = client.get(&get_url).send().await {
        if resp.status().is_success() {
            if let Ok(data) = resp.json::<LrclibResponse>().await {
                if let Some(synced) = data.synced_lyrics {
                    if !synced.trim().is_empty() {
                        return Ok(synced);
                    }
                }
                if let Some(plain) = data.plain_lyrics {
                    if !plain.trim().is_empty() {
                        return Ok(plain);
                    }
                }
            }
        }
    }

    // 2. Fallback search endpoint
    let query = format!("{} {}", track_title, artist);
    let search_url = format!(
        "https://lrclib.net/api/search?q={}",
        urlencoding::encode(&query)
    );

    let resp = client.get(&search_url).send().await?;
    if resp.status().is_success() {
        let items: Vec<LrclibResponse> = resp.json().await.unwrap_or_default();
        for item in items {
            if let Some(synced) = item.synced_lyrics {
                if !synced.trim().is_empty() {
                    return Ok(synced);
                }
            }
            if let Some(plain) = item.plain_lyrics {
                if !plain.trim().is_empty() {
                    return Ok(plain);
                }
            }
        }
    }

    Err(InnerTubeError::NoLyrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lrclib_live() {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();
        let res = fetch_lyrics(&client, "Let It Happen", "Tame Impala", Some(468.0)).await;
        match res {
            Ok(lrc) => println!(
                "LYRICS OK len={} preview={:?}",
                lrc.len(),
                lrc.get(..lrc.len().min(80)).unwrap_or_default()
            ),
            Err(e) => println!("LYRICS ERR: {e:#}"),
        }
    }
}
