use crate::ffi::{CxxLibraryAlbum, CxxLibraryArtist, CxxLibraryPlaylist, CxxTrackMetadata};
use anyhow::Result;
use serde_json::Value;

pub fn parse_search_results(json: &Value) -> Result<Vec<CxxTrackMetadata>> {
    let mut tracks = Vec::new();

    let sections = json
        .pointer("/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")
        .or_else(|| json.pointer("/contents/sectionListRenderer/contents"));

    if let Some(sections_array) = sections.and_then(|v| v.as_array()) {
        for section in sections_array {
            if let Some(items) = section
                .pointer("/musicShelfRenderer/contents")
                .or_else(|| section.pointer("/musicCardShelfRenderer/contents"))
                .and_then(|v| v.as_array())
            {
                for item in items {
                    if let Some(track) = parse_list_item(item) {
                        tracks.push(track);
                    }
                }
            }
        }
    }

    if tracks.is_empty() {
        find_list_items(json, &mut tracks);
    }

    Ok(tracks)
}

fn parse_list_item(item: &Value) -> Option<CxxTrackMetadata> {
    let renderer = item.get("musicResponsiveListItemRenderer")?;

    let video_id = renderer
        .pointer("/playlistItemData/videoId")
        .or_else(|| renderer.pointer("/doubleTapCommand/watchEndpoint/videoId"))
        .or_else(|| renderer.pointer("/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/videoId"))
        .and_then(|v| v.as_str())?
        .to_string();

    let flex_columns = renderer.get("flexColumns")?.as_array()?;

    let title = flex_columns
        .first()?
        .pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Title")
        .to_string();

    let mut artist = "Unknown Artist".to_string();
    let mut album = String::new();
    let mut duration_secs = 0.0;

    if let Some(runs) = flex_columns
        .get(1)
        .and_then(|col| col.pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs"))
        .and_then(|v| v.as_array())
    {
        let mut text_parts = Vec::new();
        for r in runs {
            if let Some(t) = r.get("text").and_then(|v| v.as_str()) {
                let trimmed = t.trim();
                if trimmed != "•" && !trimmed.is_empty() {
                    text_parts.push(trimmed.to_string());
                }
            }
        }

        if !text_parts.is_empty() {
            artist = text_parts[0].clone();
        }
        if text_parts.len() > 1 {
            let last = &text_parts[text_parts.len() - 1];
            if last.contains(':') {
                duration_secs = parse_duration_str(last);
                if text_parts.len() > 2 {
                    album = text_parts[1].clone();
                }
            } else {
                album = text_parts[1].clone();
            }
        }
    }

    if duration_secs == 0.0 {
        duration_secs = renderer
            .pointer("/fixedColumns/0/musicResponsiveListItemFixedColumnRenderer/text/runs/0/text")
            .and_then(Value::as_str)
            .map(parse_duration_str)
            .unwrap_or(0.0);
    }

    let thumbnail_url = renderer
        .pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.last())
        .and_then(|t| t.get("url"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(CxxTrackMetadata {
        id: video_id,
        title,
        artist,
        album,
        duration_secs,
        thumbnail_url,
        lyrics: String::new(),
    })
}

fn find_list_items(val: &Value, tracks: &mut Vec<CxxTrackMetadata>) {
    if let Value::Object(map) = val {
        if map.contains_key("musicResponsiveListItemRenderer") {
            if let Some(track) = parse_list_item(val) {
                if !tracks.iter().any(|t| t.id == track.id) {
                    tracks.push(track);
                }
            }
        } else {
            for v in map.values() {
                find_list_items(v, tracks);
            }
        }
    } else if let Value::Array(arr) = val {
        for v in arr {
            find_list_items(v, tracks);
        }
    }
}

pub fn parse_duration_str(s: &str) -> f64 {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 2 {
        let m: f64 = parts[0].parse().unwrap_or(0.0);
        let s: f64 = parts[1].parse().unwrap_or(0.0);
        m * 60.0 + s
    } else if parts.len() == 3 {
        let h: f64 = parts[0].parse().unwrap_or(0.0);
        let m: f64 = parts[1].parse().unwrap_or(0.0);
        let s: f64 = parts[2].parse().unwrap_or(0.0);
        h * 3600.0 + m * 60.0 + s
    } else {
        0.0
    }
}

fn first_run_text(value: Option<&Value>) -> Option<String> {
    value?
        .pointer("/runs/0/text")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn thumbnail_url(header: &Value) -> String {
    header
        .pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
        .and_then(Value::as_array)
        .and_then(|items| items.last())
        .and_then(|item| item.get("url"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn response_browse_id(json: &Value) -> Option<String> {
    json.pointer("/responseContext/serviceTrackingParams")
        .and_then(Value::as_array)?
        .iter()
        .flat_map(|service| {
            service
                .get("params")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .find(|param| param.get("key").and_then(Value::as_str) == Some("browse_id"))
        .and_then(|param| param.get("value").and_then(Value::as_str))
        .map(str::to_string)
}

fn parse_leading_count(value: &str) -> i32 {
    value
        .trim()
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

fn parse_compact_count(value: &str) -> i64 {
    let normalized = value.trim().replace(',', "");
    let suffix = normalized.chars().last().unwrap_or_default();
    let number = if suffix.is_ascii_alphabetic() {
        normalized[..normalized.len() - suffix.len_utf8()]
            .parse::<f64>()
            .unwrap_or(0.0)
    } else {
        normalized.parse::<f64>().unwrap_or(0.0)
    };
    let multiplier = match suffix.to_ascii_uppercase() {
        'K' => 1_000.0,
        'M' => 1_000_000.0,
        'B' => 1_000_000_000.0,
        _ => 1.0,
    };
    (number * multiplier).round() as i64
}

fn parse_duration_display(value: &str) -> f64 {
    if value.contains(':') {
        return parse_duration_str(value);
    }

    value
        .split(',')
        .filter_map(|part| {
            let mut words = part.split_whitespace();
            let amount = words.next()?.parse::<f64>().ok()?;
            let unit = words.next()?.to_ascii_lowercase();
            let multiplier = if unit.starts_with("hour") {
                3600.0
            } else if unit.starts_with("minute") {
                60.0
            } else if unit.starts_with("second") {
                1.0
            } else {
                return None;
            };
            Some(amount * multiplier)
        })
        .sum()
}

pub fn parse_album_detail(json: &Value) -> Option<CxxLibraryAlbum> {
    let header = json.pointer("/header/musicResponsiveHeaderRenderer")?;
    let title = first_run_text(header.get("title"))?;
    let artist = first_run_text(header.get("straplineTextOne"))
        .or_else(|| first_run_text(header.get("straplineTextTwo")))
        .unwrap_or_default();
    let subtitle_runs = header.pointer("/subtitle/runs").and_then(Value::as_array);
    let year = subtitle_runs
        .into_iter()
        .flatten()
        .filter_map(|run| run.get("text").and_then(Value::as_str))
        .find(|text| text.len() == 4 && text.chars().all(|ch| ch.is_ascii_digit()))
        .unwrap_or_default()
        .to_string();
    let second_subtitle = header.get("secondSubtitle");
    let summary = second_subtitle
        .and_then(|value| value.pointer("/runs/0/text"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let track_count = parse_leading_count(summary);
    let header_duration_secs = second_subtitle
        .and_then(|value| value.pointer("/runs"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|run| run.get("text").and_then(Value::as_str))
        .map(parse_duration_display)
        .find(|duration| *duration > 0.0)
        .unwrap_or(0.0);

    let tracks = json
        .pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|section| section.pointer("/musicShelfRenderer/contents"))
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(parse_list_item)
        .collect::<Vec<_>>();
    let track_duration_secs = tracks.iter().map(|track| track.duration_secs).sum::<f64>();
    let duration_secs = if header_duration_secs > 0.0 {
        header_duration_secs
    } else {
        track_duration_secs
    };

    Some(CxxLibraryAlbum {
        id: response_browse_id(json)?,
        title,
        artist,
        thumbnail_url: thumbnail_url(header),
        year,
        track_count,
        duration_secs,
    })
}

pub fn parse_artist_detail(json: &Value) -> Option<CxxLibraryArtist> {
    let header = json
        .pointer("/header/musicImmersiveHeaderRenderer")
        .or_else(|| json.pointer("/header/musicVisualHeaderRenderer"))?;
    let subscription = header.pointer("/subscriptionButton/subscribeButtonRenderer");
    let id = subscription
        .and_then(|value| value.get("channelId"))
        .and_then(Value::as_str)?
        .to_string();
    let follower_text = subscription
        .and_then(|value| first_run_text(value.get("subscriberCountText")))
        .unwrap_or_default();

    Some(CxxLibraryArtist {
        id,
        name: first_run_text(header.get("title"))?,
        thumbnail_url: thumbnail_url(header),
        follower_count: parse_compact_count(&follower_text),
    })
}

pub fn parse_playlist_detail(json: &Value) -> Option<CxxLibraryPlaylist> {
    let header = json
        .pointer("/header/musicResponsiveHeaderRenderer")
        .or_else(|| json.pointer("/header/musicEditablePlaylistDetailHeaderRenderer"))?;
    let shelf = json
        .pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|section| section.get("musicPlaylistShelfRenderer"));
    let id = shelf
        .and_then(|value| value.get("playlistId"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            response_browse_id(json).map(|value| value.trim_start_matches("VL").to_string())
        })?;
    let second_subtitle = header.get("secondSubtitle");
    let summary = second_subtitle
        .and_then(|value| value.pointer("/runs/0/text"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    Some(CxxLibraryPlaylist {
        id,
        name: first_run_text(header.get("title"))?,
        thumbnail_url: thumbnail_url(header),
        track_count: parse_leading_count(summary),
        is_owner: json
            .pointer("/header/musicEditablePlaylistDetailHeaderRenderer")
            .is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_track_from_a_real_innertube_browse_response() {
        let json: Value = serde_json::from_str(include_str!("fixtures/album.json")).unwrap();
        let tracks = parse_search_results(&json).unwrap();

        assert!(!tracks.is_empty());
        assert_eq!(tracks[0].id, "0m68rsj48X0");
        assert_eq!(tracks[0].title, "The Explanation");
        assert_eq!(tracks[0].artist, "XXXTENTACION");
        assert_eq!(tracks[0].duration_secs, 51.0);
    }

    #[test]
    fn parses_album_metadata_from_a_real_innertube_response() {
        let json: Value = serde_json::from_str(include_str!("fixtures/album.json")).unwrap();
        let album = parse_album_detail(&json).unwrap();

        assert_eq!(album.id, "MPREb_Tr9YVfRfip1");
        assert_eq!(album.title, "17");
        assert_eq!(album.artist, "XXXTENTACION");
        assert_eq!(album.year, "2017");
        assert_eq!(album.track_count, 11);
        assert!(album.duration_secs > 0.0);
        assert!(album.thumbnail_url.starts_with("https://"));
    }

    #[test]
    fn parses_artist_metadata_from_a_real_innertube_response() {
        let json: Value = serde_json::from_str(include_str!("fixtures/artist.json")).unwrap();
        let artist = parse_artist_detail(&json).unwrap();

        assert_eq!(artist.id, "UCJwGWV914kBlV4dKRn7AEFA");
        assert_eq!(artist.name, "Hatsune Miku");
        assert_eq!(artist.follower_count, 4_410_000);
        assert!(artist.thumbnail_url.starts_with("https://"));
    }

    #[test]
    fn parses_playlist_metadata_from_a_real_innertube_response() {
        let json: Value = serde_json::from_str(include_str!("fixtures/playlist.json")).unwrap();
        let playlist = parse_playlist_detail(&json).unwrap();

        assert_eq!(playlist.id, "PLxyTaDz8f5PBc-8kE36gvB-eflhODG2dw");
        assert_eq!(playlist.name, "collaborative");
        assert_eq!(playlist.track_count, 2);
        assert!(!playlist.is_owner);
        assert!(playlist.thumbnail_url.starts_with("https://"));
    }

    #[test]
    fn test_parse_duration_str() {
        assert_eq!(parse_duration_str("0:45"), 45.0);
        assert_eq!(parse_duration_str("3:24"), 204.0);
        assert_eq!(parse_duration_str("1:02:15"), 3735.0);
        assert_eq!(parse_duration_str("invalid"), 0.0);
    }

    #[test]
    fn test_parse_search_results_empty() {
        let empty_json = serde_json::json!({});
        let res = parse_search_results(&empty_json).unwrap();
        assert!(res.is_empty());
    }

    #[test]
    fn test_parse_search_results_with_mock() {
        let mock_json = serde_json::json!({
            "contents": {
                "sectionListRenderer": {
                    "contents": [{
                        "musicShelfRenderer": {
                            "contents": [{
                                "musicResponsiveListItemRenderer": {
                                    "playlistItemData": { "videoId": "abc12345" },
                                    "flexColumns": [
                                        {
                                            "musicResponsiveListItemFlexColumnRenderer": {
                                                "text": { "runs": [{ "text": "Test Song" }] }
                                            }
                                        },
                                        {
                                            "musicResponsiveListItemFlexColumnRenderer": {
                                                "text": { "runs": [
                                                    { "text": "Test Artist" },
                                                    { "text": " • " },
                                                    { "text": "Test Album" },
                                                    { "text": " • " },
                                                    { "text": "3:30" }
                                                ] }
                                            }
                                        }
                                    ],
                                    "thumbnail": {
                                        "musicThumbnailRenderer": {
                                            "thumbnail": {
                                                "thumbnails": [{ "url": "https://example.com/thumb.jpg" }]
                                            }
                                        }
                                    }
                                }
                            }]
                        }
                    }]
                }
            }
        });

        let results = parse_search_results(&mock_json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "abc12345");
        assert_eq!(results[0].title, "Test Song");
        assert_eq!(results[0].artist, "Test Artist");
        assert_eq!(results[0].album, "Test Album");
        assert_eq!(results[0].duration_secs, 210.0);
        assert_eq!(results[0].thumbnail_url, "https://example.com/thumb.jpg");
    }
}
