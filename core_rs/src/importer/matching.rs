//! Conservative matching between an imported resource and the library.
//!
//! The rules are ordered by the strength of the identifier: ISRC first, then a
//! provider id, then names, then duration. An uncertain result is reported as
//! uncertain — the importer never merges an ambiguous recording on its own.

use serde::{Deserialize, Serialize};

use super::formats::ParsedTrack;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Candidate {
    /// The id that can be played, or an imported item id.
    pub track_id: String,
    pub item_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: f64,
    pub thumbnail: String,
    pub source: String,
    pub isrc: String,
    pub availability: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchResult {
    /// exact | high | possible | ambiguous | not_found
    pub state: String,
    pub confidence: i32,
    pub candidate: Option<Candidate>,
    pub alternatives: Vec<Candidate>,
}

impl Default for MatchResult {
    fn default() -> Self {
        Self {
            state: "not_found".into(),
            confidence: 0,
            candidate: None,
            alternatives: Vec::new(),
        }
    }
}

pub fn match_track(track: &ParsedTrack, candidates: &[Candidate]) -> MatchResult {
    let mut scored: Vec<(i32, &Candidate)> = candidates
        .iter()
        .map(|candidate| (score(track, candidate), candidate))
        .filter(|(score, _)| *score >= 40)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    if scored.is_empty() {
        return MatchResult::default();
    }

    let best_score = scored[0].0;
    let best = scored[0].1.clone();
    let runner_up = scored.get(1).map(|(score, _)| *score).unwrap_or(0);
    let alternatives: Vec<Candidate> = scored
        .iter()
        .skip(1)
        .take(4)
        .map(|(_, candidate)| (*candidate).clone())
        .collect();

    // Two candidates almost as good as each other are an ambiguous answer,
    // not permission to pick one.
    let (state, confidence) = if best_score >= 98 {
        ("exact", best_score)
    } else if best_score >= 82 {
        if runner_up >= best_score - 4 {
            ("ambiguous", best_score)
        } else {
            ("high", best_score)
        }
    } else if best_score >= 58 {
        if runner_up >= best_score - 4 {
            ("ambiguous", best_score)
        } else {
            ("possible", best_score)
        }
    } else {
        ("not_found", best_score)
    };

    MatchResult {
        state: state.into(),
        confidence,
        candidate: Some(best),
        alternatives: if state == "ambiguous" {
            alternatives
        } else {
            alternatives
        },
    }
}

pub fn score(track: &ParsedTrack, candidate: &Candidate) -> i32 {
    if !track.isrc.trim().is_empty()
        && !candidate.isrc.trim().is_empty()
        && normalise(&track.isrc) == normalise(&candidate.isrc)
    {
        return 100;
    }

    // A provider id that survived into the library is the strongest possible
    // local identity after ISRC.
    if !track.source_id.trim().is_empty()
        && (track.source_id == candidate.track_id || track.source_id == candidate.item_id)
    {
        return 100;
    }

    let title_a = normalise(&track.title);
    let title_b = normalise(&candidate.title);
    if title_a.is_empty() || title_b.is_empty() {
        return 0;
    }

    let artist_a = normalise(&track.artist);
    let artist_b = normalise(&candidate.artist);
    let album_a = normalise(&track.album);
    let album_b = normalise(&candidate.album);

    let mut score = if title_a == title_b {
        55
    } else if title_a.contains(&title_b) || title_b.contains(&title_a) {
        30
    } else {
        let similarity = token_similarity(&title_a, &title_b);
        if similarity >= 0.85 {
            40
        } else if similarity >= 0.65 {
            22
        } else {
            return 0;
        }
    };

    if !artist_a.is_empty() && !artist_b.is_empty() {
        if artist_a == artist_b {
            score += 30;
        } else if artist_a.contains(&artist_b) || artist_b.contains(&artist_a) {
            score += 18;
        } else if token_similarity(&artist_a, &artist_b) >= 0.7 {
            score += 10;
        } else {
            // A different artist is a different recording. Token overlap on a
            // collaboration still counts, but a plain mismatch does not.
            score -= 25;
        }
    } else {
        score += 5;
    }

    if !album_a.is_empty() && !album_b.is_empty() {
        if album_a == album_b {
            score += 8;
        } else if album_a.contains(&album_b) || album_b.contains(&album_a) {
            score += 4;
        }
    }

    if track.duration_secs > 1.0 && candidate.duration_secs > 1.0 {
        let delta = (track.duration_secs - candidate.duration_secs).abs();
        if delta <= 2.0 {
            score += 7;
        } else if delta <= 5.0 {
            score += 3;
        } else if delta > 15.0 {
            // The same names with a wildly different length are usually a
            // live version, a remix, or a different edit.
            score -= 12;
        }
    }

    score.clamp(0, 100)
}

/// Lower case, punctuation removed, whitespace collapsed. Version markers are
/// deliberately kept: "(live)" is a different recording from the studio cut.
pub fn normalise(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if c.is_whitespace() || c == '-' || c == '_' || c == '/' {
            out.push(' ');
        }
        // Apostrophes vanish so "don't" matches "dont".
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn token_similarity(a: &str, b: &str) -> f64 {
    let a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(&b).count() as f64;
    let union = a.union(&b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(title: &str, artist: &str, duration: f64) -> ParsedTrack {
        ParsedTrack {
            title: title.into(),
            artist: artist.into(),
            duration_secs: duration,
            ..Default::default()
        }
    }

    fn candidate(title: &str, artist: &str, duration: f64) -> Candidate {
        Candidate {
            track_id: "yt:1".into(),
            title: title.into(),
            artist: artist.into(),
            duration_secs: duration,
            ..Default::default()
        }
    }

    #[test]
    fn exact_names_are_high_confidence() {
        let result = match_track(
            &track("Bohemian Rhapsody", "Queen", 354.0),
            &[candidate("Bohemian Rhapsody", "Queen", 355.0)],
        );
        assert!(matches!(result.state.as_str(), "exact" | "high"));
        assert!(result.confidence >= 82);
    }

    #[test]
    fn a_different_artist_is_not_matched() {
        let result = match_track(
            &track("Bohemian Rhapsody", "Queen", 354.0),
            &[candidate(
                "Bohemian Rhapsody",
                "The Chemical Brothers",
                354.0,
            )],
        );
        assert_eq!(result.state, "not_found");
    }

    #[test]
    fn isrc_wins_over_names() {
        let mut imported = track("Different title", "Different artist", 0.0);
        imported.isrc = "GBUM71029604".into();
        let mut library = candidate("Bohemian Rhapsody", "Queen", 0.0);
        library.isrc = "gbum71029604".into();
        let result = match_track(&imported, &[library]);
        assert_eq!(result.state, "exact");
        assert_eq!(result.confidence, 100);
    }

    #[test]
    fn two_close_candidates_are_ambiguous() {
        let result = match_track(
            &track("Song", "Artist", 200.0),
            &[
                candidate("Song", "Artist", 201.0),
                candidate("Song", "Artist", 200.5),
            ],
        );
        assert_eq!(result.state, "ambiguous");
        assert!(!result.alternatives.is_empty());
    }

    #[test]
    fn remix_is_not_collapsed_into_original() {
        let result = match_track(
            &track("Song (Club Remix)", "Artist", 380.0),
            &[candidate("Song", "Artist", 200.0)],
        );
        assert_ne!(result.state, "exact");
    }

    #[test]
    fn normalisation_keeps_version_markers() {
        assert_eq!(normalise("Song (Live)"), "song live");
        assert_eq!(normalise("Don't  Stop"), "dont stop");
    }
}
