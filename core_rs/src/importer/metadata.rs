//! Audio metadata reading for the local importer.
//!
//! Melyar ships no tagging library, so the four container families the import
//! centre promises are read here directly: ID3v2 for MP3/AIFF/WAV, Vorbis
//! comments for FLAC/OGG/Opus, and the `ilst` atom tree for MP4/M4A. The
//! readers are deliberately narrow — they take the fields a library needs and
//! ignore the rest — but they are real parsers, not filename guesses.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Picture {
    pub mime: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct AudioMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub duration_secs: f64,
    pub track_no: i32,
    pub disc_no: i32,
    pub year: String,
    pub genre: String,
    pub isrc: String,
    pub picture: Option<Picture>,
    pub container: String,
}

/// Extensions the importer accepts. Anything else is left alone.
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "oga", "opus", "aac", "m4a", "m4b", "aiff", "aif", "aifc",
];

pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

pub fn read_metadata(path: &Path) -> Result<AudioMetadata> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();

    let mut meta = match ext.as_str() {
        "mp3" => read_mp3(path)?,
        "flac" => read_flac(path)?,
        "ogg" | "oga" | "opus" => read_ogg(path)?,
        "m4a" | "m4b" | "mp4" | "aac" => read_mp4(path)?,
        "wav" => read_wav(path)?,
        "aiff" | "aif" | "aifc" => read_aiff(path)?,
        _ => return Err(anyhow!("unsupported audio extension: {ext}")),
    };

    if meta.title.trim().is_empty() {
        meta.title = title_from_filename(path);
    }
    if meta.artist.trim().is_empty() {
        meta.artist = "Unknown artist".to_string();
    }
    if meta.album.trim().is_empty() {
        meta.album = "Unknown album".to_string();
    }
    if meta.container.is_empty() {
        meta.container = ext;
    }
    Ok(meta)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn file_len(path: &Path) -> Result<u64> {
    Ok(std::fs::metadata(path)?.len())
}

fn read_at(file: &mut File, offset: u64, len: usize) -> Result<Vec<u8>> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; len];
    let read = file.read(&mut buf)?;
    buf.truncate(read);
    Ok(buf)
}

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn be16(bytes: &[u8]) -> u16 {
    u16::from_be_bytes([bytes[0], bytes[1]])
}

fn syncsafe(bytes: &[u8]) -> usize {
    ((bytes[0] as usize & 0x7f) << 21)
        | ((bytes[1] as usize & 0x7f) << 14)
        | ((bytes[2] as usize & 0x7f) << 7)
        | (bytes[3] as usize & 0x7f)
}

fn title_from_filename(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled");
    let cleaned = stem.replace('_', " ");
    // "01 - Song" and "01. Song" are common; the number is not the title.
    let trimmed = cleaned.trim();
    let without_number = match trimmed.find(" - ") {
        Some(pos) if trimmed[..pos].trim().chars().all(|c| c.is_ascii_digit()) => {
            &trimmed[pos + 3..]
        }
        _ => match trimmed.find(". ") {
            Some(pos) if trimmed[..pos].trim().chars().all(|c| c.is_ascii_digit()) => {
                &trimmed[pos + 2..]
            }
            _ => trimmed,
        },
    };
    without_number.trim().to_string()
}

/// Save embedded artwork in the cache and return a file URL the web view can
/// display. Identical bytes share one file, so a thousand tracks from one
/// album do not create a thousand copies.
pub fn cache_picture(picture: &Picture) -> Option<String> {
    if picture.data.is_empty() {
        return None;
    }
    let ext = match picture.mime.as_str() {
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "jpg",
    };
    let dir = dirs::cache_dir()?.join("melyar").join("artwork");
    std::fs::create_dir_all(&dir).ok()?;
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in &picture.data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let path: PathBuf = dir.join(format!("{hash:016x}.{ext}"));
    if !path.exists() {
        std::fs::write(&path, &picture.data).ok()?;
    }
    url::Url::from_file_path(&path).ok().map(|u| u.to_string())
}

// ---------------------------------------------------------------------------
// ID3v2 / ID3v1
// ---------------------------------------------------------------------------

fn read_mp3(path: &Path) -> Result<AudioMetadata> {
    let mut file = File::open(path)?;
    let len = file_len(path)?;
    let head = read_at(&mut file, 0, 10)?;
    let mut meta = AudioMetadata {
        container: "mp3".into(),
        ..Default::default()
    };

    let mut audio_start = 0u64;
    if head.len() >= 10 && &head[0..3] == b"ID3" {
        let tag_size = syncsafe(&head[6..10]) + 10;
        let tag = read_at(&mut file, 0, tag_size.min(len as usize))?;
        apply_id3(&tag, &mut meta);
        audio_start = tag_size as u64;
    }

    // ID3v1 sits in the last 128 bytes and is only a fallback.
    if meta.title.is_empty() {
        if len >= 128 {
            let tail = read_at(&mut file, len - 128, 128)?;
            if tail.len() == 128 && &tail[0..3] == b"TAG" {
                apply_id3v1(&tail, &mut meta);
            }
        }
    }

    meta.duration_secs = mp3_duration(&mut file, audio_start, len).unwrap_or(0.0);
    Ok(meta)
}

fn apply_id3v1(tag: &[u8], meta: &mut AudioMetadata) {
    let text = |start: usize, end: usize| -> String {
        let slice = &tag[start..end];
        let end = slice.iter().position(|b| *b == 0).unwrap_or(slice.len());
        String::from_utf8_lossy(&slice[..end]).trim().to_string()
    };
    if meta.title.is_empty() {
        meta.title = text(3, 33);
    }
    if meta.artist.is_empty() {
        meta.artist = text(33, 63);
    }
    if meta.album.is_empty() {
        meta.album = text(63, 93);
    }
    if meta.year.is_empty() {
        meta.year = text(93, 97);
    }
    if tag.len() > 125 && tag[125] == 0 {
        meta.track_no = tag[126] as i32;
    }
}

fn apply_id3(tag: &[u8], meta: &mut AudioMetadata) {
    if tag.len() < 10 {
        return;
    }
    let version = tag[3];
    let flags = tag[5];
    let body = &tag[10..];
    let body = if flags & 0x80 != 0 {
        deunsynchronise(body)
    } else {
        body.to_vec()
    };

    let mut pos = 0usize;
    if flags & 0x40 != 0 {
        if version >= 4 {
            if body.len() >= 4 {
                pos = syncsafe(&body[0..4]);
            }
        } else if body.len() >= 4 {
            pos = 4 + be32(&body[0..4]) as usize;
        }
    }

    let frame_header = if version <= 2 { 6 } else { 10 };
    while pos + frame_header <= body.len() {
        let id_bytes = &body[pos..pos + if version <= 2 { 3 } else { 4 }];
        if id_bytes.iter().all(|b| *b == 0) {
            break;
        }
        let id = String::from_utf8_lossy(id_bytes).to_string();
        let (size, header) = if version <= 2 {
            (
                be32(&[0, body[pos + 3], body[pos + 4], body[pos + 5]]) as usize,
                6,
            )
        } else if version >= 4 {
            (syncsafe(&body[pos + 4..pos + 8]), 10)
        } else {
            (be32(&body[pos + 4..pos + 8]) as usize, 10)
        };
        if header == 0 || pos + header + size > body.len() {
            break;
        }
        let frame = &body[pos + header..pos + header + size];
        apply_id3_frame(&id, frame, meta);
        pos += header + size;
    }
}

fn deunsynchronise(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        out.push(input[i]);
        if input[i] == 0xff && i + 1 < input.len() && input[i + 1] == 0x00 {
            i += 1;
        }
        i += 1;
    }
    out
}

fn apply_id3_frame(id: &str, frame: &[u8], meta: &mut AudioMetadata) {
    match id {
        "TIT2" | "TT2" | "TITLE" => meta.title = decode_text_frame(frame),
        "TPE1" | "TP1" | "ARTIST" => meta.artist = decode_text_frame(frame),
        "TPE2" | "TP2" | "ALBUMARTIST" => meta.album_artist = decode_text_frame(frame),
        "TALB" | "TAL" | "ALBUM" => meta.album = decode_text_frame(frame),
        "TCON" | "TCO" | "GENRE" => meta.genre = decode_text_frame(frame),
        "TSRC" | "TRC" | "ISRC" => meta.isrc = decode_text_frame(frame),
        "TDRC" | "TYER" | "TYE" | "DATE" => {
            let value = decode_text_frame(frame);
            meta.year = value.chars().take(4).collect();
        }
        "TRCK" | "TRK" | "TRACKNUMBER" => {
            let value = decode_text_frame(frame);
            meta.track_no = value
                .split('/')
                .next()
                .and_then(|v| v.trim().parse::<i32>().ok())
                .unwrap_or(0);
        }
        "TPOS" | "TPA" | "DISCNUMBER" => {
            let value = decode_text_frame(frame);
            meta.disc_no = value
                .split('/')
                .next()
                .and_then(|v| v.trim().parse::<i32>().ok())
                .unwrap_or(0);
        }
        "APIC" | "PIC" => {
            if let Some(picture) = decode_apic(frame, id == "PIC") {
                if meta.picture.is_none() {
                    meta.picture = Some(picture);
                }
            }
        }
        _ => {}
    }
}

fn decode_text_frame(frame: &[u8]) -> String {
    if frame.is_empty() {
        return String::new();
    }
    let (text, _) = decode_encoded_string(frame, frame[0]);
    text.trim_matches('\0').trim().to_string()
}

/// Decode a string that starts with an ID3 encoding byte, returning the text
/// and how many bytes were consumed (including the encoding byte).
fn decode_encoded_string(bytes: &[u8], encoding: u8) -> (String, usize) {
    if bytes.is_empty() {
        return (String::new(), 0);
    }
    let body = &bytes[1..];
    match encoding {
        0 => {
            let end = body.iter().position(|b| *b == 0).unwrap_or(body.len());
            (latin1_to_string(&body[..end]), 1 + end)
        }
        1 | 2 => {
            let mut end = body.len();
            let mut i = 0;
            while i + 1 < body.len() {
                if body[i] == 0 && body[i + 1] == 0 {
                    end = i;
                    break;
                }
                i += 2;
            }
            let slice = &body[..end];
            let text = if encoding == 1 {
                if slice.len() >= 2 && slice[0] == 0xff && slice[1] == 0xfe {
                    utf16_lossy(&slice[2..], true)
                } else if slice.len() >= 2 && slice[0] == 0xfe && slice[1] == 0xff {
                    utf16_lossy(&slice[2..], false)
                } else {
                    utf16_lossy(slice, true)
                }
            } else {
                utf16_lossy(slice, false)
            };
            (text, 1 + end + 2)
        }
        _ => {
            let end = body.iter().position(|b| *b == 0).unwrap_or(body.len());
            (String::from_utf8_lossy(&body[..end]).to_string(), 1 + end)
        }
    }
}

fn latin1_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| *b as char).collect()
}

fn utf16_lossy(bytes: &[u8], little_endian: bool) -> String {
    let mut units = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        let unit = if little_endian {
            u16::from_le_bytes([bytes[i], bytes[i + 1]])
        } else {
            u16::from_be_bytes([bytes[i], bytes[i + 1]])
        };
        units.push(unit);
        i += 2;
    }
    String::from_utf16_lossy(&units)
}

fn decode_apic(frame: &[u8], v22: bool) -> Option<Picture> {
    if frame.is_empty() {
        return None;
    }
    let encoding = frame[0];
    let mut pos = 1usize;
    let mime = if v22 {
        // PIC uses a three character image format instead of a mime type.
        if frame.len() < 5 {
            return None;
        }
        let fmt = String::from_utf8_lossy(&frame[1..4]).to_uppercase();
        pos = 4;
        match fmt.as_str() {
            "PNG" => "image/png".to_string(),
            _ => "image/jpeg".to_string(),
        }
    } else {
        let end = frame[pos..].iter().position(|b| *b == 0)? + pos;
        let mime = String::from_utf8_lossy(&frame[pos..end]).to_string();
        pos = end + 1;
        mime
    };
    if pos >= frame.len() {
        return None;
    }
    pos += 1; // picture type
    let (_, consumed) = decode_encoded_string(&frame[pos..], encoding);
    pos += consumed.min(frame.len().saturating_sub(pos));
    if pos >= frame.len() {
        return None;
    }
    let data = frame[pos..].to_vec();
    if data.is_empty() {
        return None;
    }
    Some(Picture { mime, data })
}

fn mp3_duration(file: &mut File, start: u64, len: u64) -> Option<f64> {
    if start >= len {
        return None;
    }
    let head = read_at(file, start, 4096).ok()?;
    let mut offset = 0usize;
    while offset + 4 <= head.len() {
        if head[offset] == 0xff && (head[offset + 1] & 0xe0) == 0xe0 {
            break;
        }
        offset += 1;
    }
    if offset + 4 > head.len() {
        return None;
    }
    let b1 = head[offset + 1];
    let b2 = head[offset + 2];
    let b3 = head[offset + 3];
    let version = (b1 >> 3) & 0x03;
    let layer = (b1 >> 1) & 0x03;
    let bitrate_index = (b2 >> 4) & 0x0f;
    let sample_index = (b2 >> 2) & 0x03;
    let padding = (b2 >> 1) & 0x01;

    let bitrates = match (version, layer) {
        // MPEG1 layer III
        (3, 1) => [
            0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
        ],
        // MPEG1 layer II
        (3, 2) => [
            0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 0,
        ],
        // MPEG1 layer I
        (3, 3) => [
            0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 0,
        ],
        // MPEG2/2.5 layer III
        (2, 1) | (0, 1) => [
            0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
        ],
        // MPEG2/2.5 layer II
        (2, 2) | (0, 2) => [
            0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
        ],
        _ => return None,
    };
    let bitrate = *bitrates.get(bitrate_index as usize)? as f64 * 1000.0;
    let sample_rate = match sample_index {
        0 => 44100.0,
        1 => 48000.0,
        2 => 32000.0,
        _ => return None,
    } / if version == 3 {
        1.0
    } else if version == 2 {
        2.0
    } else {
        4.0
    };
    if bitrate <= 0.0 || sample_rate <= 0.0 {
        return None;
    }

    // A Xing/Info header carries the exact frame count, which makes VBR files
    // accurate instead of merely plausible.
    let side_info = if version == 3 {
        if b3 & 0x01 == 0 {
            17
        } else {
            32
        }
    } else if b3 & 0x01 == 0 {
        9
    } else {
        17
    };
    let xing_offset = offset + 4 + side_info;
    if xing_offset + 12 <= head.len() {
        let tag = &head[xing_offset..xing_offset + 4];
        if tag == b"Xing" || tag == b"Info" {
            let flags = be32(&head[xing_offset + 4..xing_offset + 8]);
            if flags & 0x01 != 0 {
                let frames = be32(&head[xing_offset + 8..xing_offset + 12]) as f64;
                return Some(frames * samples_per_frame(version, layer) / sample_rate);
            }
        }
    }

    let frame_len = if layer == 3 {
        (12.0 * bitrate / sample_rate + padding as f64) * 4.0
    } else {
        samples_per_frame(version, layer) / 8.0 * bitrate / sample_rate + padding as f64
    };
    let first_frame = read_at(file, start + offset as u64, frame_len.ceil() as usize).ok()?;
    let _ = first_frame;
    let audio_bytes = (len - start) as f64;
    Some(audio_bytes * 8.0 / bitrate)
}

fn samples_per_frame(version: u8, layer: u8) -> f64 {
    match (version, layer) {
        (3, 3) => 384.0,
        (3, 2) => 1152.0,
        (3, 1) => 1152.0,
        (2, 2) | (0, 2) => 1152.0,
        _ => 576.0,
    }
}

// ---------------------------------------------------------------------------
// FLAC
// ---------------------------------------------------------------------------

fn read_flac(path: &Path) -> Result<AudioMetadata> {
    let mut file = File::open(path)?;
    let head = read_at(&mut file, 0, 4)?;
    if head != b"fLaC" {
        return Err(anyhow!("not a FLAC file"));
    }
    let mut meta = AudioMetadata {
        container: "flac".into(),
        ..Default::default()
    };
    let len = file_len(path)?;
    let mut pos = 4u64;
    loop {
        if pos + 4 > len {
            break;
        }
        let header = read_at(&mut file, pos, 4)?;
        if header.len() < 4 {
            break;
        }
        let last = header[0] & 0x80 != 0;
        let block_type = header[0] & 0x7f;
        let block_len =
            ((header[1] as usize) << 16) | ((header[2] as usize) << 8) | header[3] as usize;
        pos += 4;
        if pos + block_len as u64 > len {
            break;
        }
        let block = read_at(&mut file, pos, block_len)?;
        match block_type {
            0 => {
                if block.len() >= 18 {
                    let packed = u64::from_be_bytes([
                        block[10], block[11], block[12], block[13], block[14], block[15],
                        block[16], block[17],
                    ]);
                    let sample_rate = (packed >> 44) as f64;
                    let total_samples = (packed & 0x0f_ffff_ffff) as f64;
                    if sample_rate > 0.0 {
                        meta.duration_secs = total_samples / sample_rate;
                    }
                }
            }
            4 => parse_vorbis_comment(&block, &mut meta),
            6 => {
                if let Some(picture) = parse_flac_picture(&block) {
                    meta.picture = Some(picture);
                }
            }
            _ => {}
        }
        pos += block_len as u64;
        if last {
            break;
        }
    }
    Ok(meta)
}

fn parse_flac_picture(block: &[u8]) -> Option<Picture> {
    let mut pos = 0usize;
    let read_u32 = |pos: &mut usize| -> Option<u32> {
        if *pos + 4 > block.len() {
            return None;
        }
        let v = be32(&block[*pos..*pos + 4]);
        *pos += 4;
        Some(v)
    };
    let _type = read_u32(&mut pos)?;
    let mime_len = read_u32(&mut pos)? as usize;
    if pos + mime_len > block.len() {
        return None;
    }
    let mime = String::from_utf8_lossy(&block[pos..pos + mime_len]).to_string();
    pos += mime_len;
    let desc_len = read_u32(&mut pos)? as usize;
    pos += desc_len;
    let _width = read_u32(&mut pos)?;
    let _height = read_u32(&mut pos)?;
    let _depth = read_u32(&mut pos)?;
    let _colors = read_u32(&mut pos)?;
    let data_len = read_u32(&mut pos)? as usize;
    if pos + data_len > block.len() {
        return None;
    }
    Some(Picture {
        mime,
        data: block[pos..pos + data_len].to_vec(),
    })
}

/// A Vorbis comment block: vendor length, vendor, count, then KEY=value pairs.
fn parse_vorbis_comment(block: &[u8], meta: &mut AudioMetadata) {
    if block.len() < 8 {
        return;
    }
    let mut pos = 0usize;
    let vendor_len = u32::from_le_bytes([block[0], block[1], block[2], block[3]]) as usize;
    pos += 4 + vendor_len;
    if pos + 4 > block.len() {
        return;
    }
    let count =
        u32::from_le_bytes([block[pos], block[pos + 1], block[pos + 2], block[pos + 3]]) as usize;
    pos += 4;
    for _ in 0..count {
        if pos + 4 > block.len() {
            break;
        }
        let len = u32::from_le_bytes([block[pos], block[pos + 1], block[pos + 2], block[pos + 3]])
            as usize;
        pos += 4;
        if pos + len > block.len() {
            break;
        }
        let entry = String::from_utf8_lossy(&block[pos..pos + len]).to_string();
        pos += len;
        let Some((key, value)) = entry.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match key.to_uppercase().as_str() {
            "TITLE" => meta.title = value.to_string(),
            "ARTIST" => meta.artist = value.to_string(),
            "ALBUMARTIST" | "ALBUM ARTIST" => meta.album_artist = value.to_string(),
            "ALBUM" => meta.album = value.to_string(),
            "DATE" | "YEAR" => meta.year = value.chars().take(4).collect(),
            "TRACKNUMBER" => meta.track_no = value.parse().unwrap_or(0),
            "DISCNUMBER" => meta.disc_no = value.parse().unwrap_or(0),
            "GENRE" => meta.genre = value.to_string(),
            "ISRC" => meta.isrc = value.to_string(),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// OGG / Opus
// ---------------------------------------------------------------------------

fn read_ogg(path: &Path) -> Result<AudioMetadata> {
    let mut file = File::open(path)?;
    let len = file_len(path)?;
    let mut meta = AudioMetadata {
        container: "ogg".into(),
        ..Default::default()
    };

    let head_len = len.min(512 * 1024) as usize;
    let head = read_at(&mut file, 0, head_len)?;
    let mut sample_rate = 0.0f64;
    let mut is_opus = false;

    if let Some(pos) = find_subslice(&head, b"\x01vorbis") {
        if pos + 16 <= head.len() {
            sample_rate = u32::from_le_bytes([
                head[pos + 12],
                head[pos + 13],
                head[pos + 14],
                head[pos + 15],
            ]) as f64;
        }
    }
    if let Some(pos) = find_subslice(&head, b"OpusHead") {
        is_opus = true;
        // The input sample rate is advisory; Opus always decodes at 48 kHz.
        sample_rate = 48000.0;
        let _ = pos;
    }

    if let Some(pos) = find_subslice(&head, b"\x03vorbis") {
        parse_vorbis_comment(&head[pos + 7..], &mut meta);
    } else if let Some(pos) = find_subslice(&head, b"OpusTags") {
        parse_vorbis_comment(&head[pos + 8..], &mut meta);
    }

    // The granule position of the last page is the track length. Reading the
    // tail keeps this cheap even for hour-long DJ sets.
    let tail_len = len.min(256 * 1024);
    let tail = read_at(&mut file, len - tail_len, tail_len as usize)?;
    let mut last_granule: u64 = 0;
    let mut i = 0usize;
    while i + 14 <= tail.len() {
        if &tail[i..i + 4] == b"OggS" {
            let granule = u64::from_le_bytes([
                tail[i + 6],
                tail[i + 7],
                tail[i + 8],
                tail[i + 9],
                tail[i + 10],
                tail[i + 11],
                tail[i + 12],
                tail[i + 13],
            ]);
            if granule != u64::MAX && granule > last_granule {
                last_granule = granule;
            }
        }
        i += 1;
    }
    if sample_rate <= 0.0 {
        sample_rate = if is_opus { 48000.0 } else { 44100.0 };
    }
    if last_granule > 0 {
        meta.duration_secs = last_granule as f64 / sample_rate;
    }
    Ok(meta)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

// ---------------------------------------------------------------------------
// MP4 / M4A
// ---------------------------------------------------------------------------

struct Atom {
    name: [u8; 4],
    start: u64,
    payload: u64,
    end: u64,
}

fn read_atom_header(file: &mut File, pos: u64, end: u64) -> Option<Atom> {
    if pos + 8 > end {
        return None;
    }
    let head = read_at(file, pos, 16).ok()?;
    if head.len() < 8 {
        return None;
    }
    let mut size = be32(&head[0..4]) as u64;
    let mut name = [0u8; 4];
    name.copy_from_slice(&head[4..8]);
    let mut header = 8u64;
    if size == 1 {
        if head.len() < 16 {
            return None;
        }
        size = u64::from_be_bytes([
            head[8], head[9], head[10], head[11], head[12], head[13], head[14], head[15],
        ]);
        header = 16;
    } else if size == 0 {
        size = end - pos;
    }
    if size < header || pos + size > end {
        return None;
    }
    Some(Atom {
        name,
        start: pos,
        payload: pos + header,
        end: pos + size,
    })
}

fn mp4_children(file: &mut File, start: u64, end: u64) -> Vec<Atom> {
    let mut out = Vec::new();
    let mut pos = start;
    while pos + 8 <= end {
        let Some(atom) = read_atom_header(file, pos, end) else {
            break;
        };
        if atom.name == *b"meta" {
            // `meta` is a full box: skip version and flags before its children.
            out.push(Atom {
                name: atom.name,
                start: atom.start,
                payload: atom.payload + 4,
                end: atom.end,
            });
        } else {
            out.push(atom);
        }
        pos = out.last().map(|a| a.end).unwrap_or(pos + 8);
    }
    out
}

fn read_mp4(path: &Path) -> Result<AudioMetadata> {
    let mut file = File::open(path)?;
    let len = file_len(path)?;
    let mut meta = AudioMetadata {
        container: "m4a".into(),
        ..Default::default()
    };

    let top = mp4_children(&mut file, 0, len);
    let Some(moov) = top.iter().find(|a| &a.name == b"moov") else {
        // Raw ADTS AAC has no MP4 container; fall through to the frame counter.
        meta.duration_secs = adts_duration(path).unwrap_or(0.0);
        return Ok(meta);
    };

    for child in mp4_children(&mut file, moov.payload, moov.end) {
        match &child.name {
            b"mvhd" => {
                let data = read_at(
                    &mut file,
                    child.payload,
                    (child.end - child.payload).min(120) as usize,
                )?;
                if data.len() >= 20 {
                    let version = data[0];
                    if version == 1 && data.len() >= 32 {
                        let timescale = be32(&data[20..24]) as f64;
                        let duration = u64::from_be_bytes([
                            data[24], data[25], data[26], data[27], data[28], data[29], data[30],
                            data[31],
                        ]) as f64;
                        if timescale > 0.0 {
                            meta.duration_secs = duration / timescale;
                        }
                    } else {
                        let timescale = be32(&data[12..16]) as f64;
                        let duration = be32(&data[16..20]) as f64;
                        if timescale > 0.0 {
                            meta.duration_secs = duration / timescale;
                        }
                    }
                }
            }
            b"udta" => {
                for udta_child in mp4_children(&mut file, child.payload, child.end) {
                    if &udta_child.name == b"meta" {
                        for meta_child in
                            mp4_children(&mut file, udta_child.payload, udta_child.end)
                        {
                            if &meta_child.name == b"ilst" {
                                read_ilst(
                                    &mut file,
                                    meta_child.payload,
                                    meta_child.end,
                                    &mut meta,
                                )?;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if meta.duration_secs <= 0.0 {
        meta.duration_secs = adts_duration(path).unwrap_or(0.0);
    }
    Ok(meta)
}

fn read_ilst(file: &mut File, start: u64, end: u64, meta: &mut AudioMetadata) -> Result<()> {
    for item in mp4_children(file, start, end) {
        let name = String::from_utf8_lossy(&item.name).to_string();
        let mut values: Vec<(u32, Vec<u8>)> = Vec::new();
        for data_atom in mp4_children(file, item.payload, item.end) {
            if &data_atom.name != b"data" {
                continue;
            }
            let payload = read_at(
                file,
                data_atom.payload,
                (data_atom.end - data_atom.payload) as usize,
            )?;
            if payload.len() < 8 {
                continue;
            }
            let kind = be32(&payload[0..4]) & 0x00ff_ffff;
            values.push((kind, payload[8..].to_vec()));
        }
        let first_text = || -> String {
            values
                .iter()
                .find(|(kind, _)| *kind == 1 || *kind == 2)
                .map(|(_, data)| String::from_utf8_lossy(data).trim_matches('\0').to_string())
                .unwrap_or_default()
        };
        match name.as_str() {
            "\u{a9}nam" => meta.title = first_text(),
            "\u{a9}ART" => meta.artist = first_text(),
            "aART" => meta.album_artist = first_text(),
            "\u{a9}alb" => meta.album = first_text(),
            "\u{a9}day" => meta.year = first_text().chars().take(4).collect(),
            "\u{a9}gen" => meta.genre = first_text(),
            "trkn" => {
                if let Some((_, data)) = values.first() {
                    if data.len() >= 4 {
                        meta.track_no = be16(&data[2..4]) as i32;
                    }
                }
            }
            "disk" => {
                if let Some((_, data)) = values.first() {
                    if data.len() >= 4 {
                        meta.disc_no = be16(&data[2..4]) as i32;
                    }
                }
            }
            "covr" => {
                if meta.picture.is_none() {
                    if let Some((kind, data)) = values.first() {
                        let mime = if *kind == 14 {
                            "image/png"
                        } else {
                            "image/jpeg"
                        };
                        meta.picture = Some(Picture {
                            mime: mime.to_string(),
                            data: data.clone(),
                        });
                    }
                }
            }
            "----" => {
                // Freeform atom: mean/name/data children. ISRC lives here.
                let mut key = String::new();
                for sub in mp4_children(file, item.payload, item.end) {
                    if &sub.name == b"name" {
                        let data = read_at(file, sub.payload, (sub.end - sub.payload) as usize)?;
                        if data.len() > 4 {
                            key = String::from_utf8_lossy(&data[4..])
                                .trim_matches('\0')
                                .to_string();
                        }
                    }
                }
                if key.eq_ignore_ascii_case("com.apple.iTunes:ISRC") {
                    meta.isrc = first_text();
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn adts_duration(path: &Path) -> Option<f64> {
    let mut file = File::open(path).ok()?;
    let len = file_len(path).ok()?;
    if len < 7 {
        return None;
    }
    let mut pos = 0u64;
    let mut frames = 0u64;
    let mut sample_rate = 44100.0;
    let mut first = true;
    let mut buffer = [0u8; 7];
    while pos + 7 <= len && frames < 2_000_000 {
        file.seek(SeekFrom::Start(pos)).ok()?;
        if file.read_exact(&mut buffer).is_err() {
            break;
        }
        if buffer[0] != 0xff || (buffer[1] & 0xf0) != 0xf0 {
            pos += 1;
            continue;
        }
        let sample_index = (buffer[2] >> 2) & 0x0f;
        const RATES: [f64; 13] = [
            96000.0, 88200.0, 64000.0, 48000.0, 44100.0, 32000.0, 24000.0, 22050.0, 16000.0,
            12000.0, 11025.0, 8000.0, 7350.0,
        ];
        if first {
            sample_rate = *RATES.get(sample_index as usize).unwrap_or(&44100.0);
            first = false;
        }
        let frame_len = (((buffer[3] as u64 & 0x03) << 11)
            | ((buffer[4] as u64) << 3)
            | ((buffer[5] as u64) >> 5))
            .max(7);
        pos += frame_len;
        frames += 1;
    }
    if frames == 0 {
        None
    } else {
        Some(frames as f64 * 1024.0 / sample_rate)
    }
}

// ---------------------------------------------------------------------------
// WAV / AIFF
// ---------------------------------------------------------------------------

fn read_wav(path: &Path) -> Result<AudioMetadata> {
    let mut file = File::open(path)?;
    let len = file_len(path)?;
    let head = read_at(&mut file, 0, 12)?;
    if head.len() < 12 || &head[0..4] != b"RIFF" || &head[8..12] != b"WAVE" {
        return Err(anyhow!("not a WAV file"));
    }
    let mut meta = AudioMetadata {
        container: "wav".into(),
        ..Default::default()
    };
    let mut pos = 12u64;
    while pos + 8 <= len {
        let header = read_at(&mut file, pos, 8)?;
        if header.len() < 8 {
            break;
        }
        let id = &header[0..4];
        let size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as u64;
        let payload = pos + 8;
        if payload + size > len {
            break;
        }
        match id {
            b"fmt " => {
                let data = read_at(&mut file, payload, size.min(40) as usize)?;
                if data.len() >= 12 {
                    let byte_rate = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
                    if byte_rate > 0 {
                        // The data chunk size is read below; store the rate for
                        // the duration calculation.
                        meta.duration_secs = -(byte_rate as f64);
                    }
                }
            }
            b"data" => {
                if meta.duration_secs < 0.0 {
                    meta.duration_secs = size as f64 / -meta.duration_secs;
                }
            }
            b"LIST" => {
                let data = read_at(&mut file, payload, size as usize)?;
                if data.len() >= 4 && &data[0..4] == b"INFO" {
                    parse_riff_info(&data[4..], &mut meta);
                }
            }
            b"id3 " | b"ID3 " => {
                let data = read_at(&mut file, payload, size as usize)?;
                apply_id3(&data, &mut meta);
            }
            _ => {}
        }
        pos = payload + size + (size & 1);
    }
    if meta.duration_secs < 0.0 {
        meta.duration_secs = 0.0;
    }
    Ok(meta)
}

fn parse_riff_info(data: &[u8], meta: &mut AudioMetadata) {
    let mut pos = 0usize;
    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        let size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
            as usize;
        pos += 8;
        if pos + size > data.len() {
            break;
        }
        let value = String::from_utf8_lossy(&data[pos..pos + size])
            .trim_matches('\0')
            .trim()
            .to_string();
        match id {
            b"INAM" => meta.title = value,
            b"IART" => meta.artist = value,
            b"IPRD" => meta.album = value,
            b"ICRD" => meta.year = value.chars().take(4).collect(),
            b"ITRK" | b"IPRT" => meta.track_no = value.parse().unwrap_or(0),
            b"IGNR" => meta.genre = value,
            _ => {}
        }
        pos += size + (size & 1);
    }
}

fn read_aiff(path: &Path) -> Result<AudioMetadata> {
    let mut file = File::open(path)?;
    let len = file_len(path)?;
    let head = read_at(&mut file, 0, 12)?;
    if head.len() < 12 || &head[0..4] != b"FORM" {
        return Err(anyhow!("not an AIFF file"));
    }
    let mut meta = AudioMetadata {
        container: "aiff".into(),
        ..Default::default()
    };
    let mut pos = 12u64;
    while pos + 8 <= len {
        let header = read_at(&mut file, pos, 8)?;
        if header.len() < 8 {
            break;
        }
        let id = &header[0..4];
        let size = be32(&header[4..8]) as u64;
        let payload = pos + 8;
        if payload + size > len {
            break;
        }
        match id {
            b"COMM" => {
                let data = read_at(&mut file, payload, size.min(40) as usize)?;
                if data.len() >= 18 {
                    let frames = be32(&data[2..6]) as f64;
                    let rate = read_extended(&data[8..18]);
                    if rate > 0.0 {
                        meta.duration_secs = frames / rate;
                    }
                }
            }
            b"id3 " | b"ID3 " => {
                let data = read_at(&mut file, payload, size as usize)?;
                apply_id3(&data, &mut meta);
            }
            _ => {}
        }
        pos = payload + size + (size & 1);
    }
    Ok(meta)
}

/// 80-bit IEEE 754 extended precision, as used by AIFF sample rates.
fn read_extended(bytes: &[u8]) -> f64 {
    if bytes.len() < 10 {
        return 0.0;
    }
    let exponent = (((bytes[0] as u16 & 0x7f) << 8) | bytes[1] as u16) as i32;
    let mantissa = u64::from_be_bytes([
        bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9],
    ]);
    if exponent == 0 && mantissa == 0 {
        return 0.0;
    }
    let sign = if bytes[0] & 0x80 != 0 { -1.0 } else { 1.0 };
    sign * (mantissa as f64 / (1u64 << 63) as f64) * 2f64.powi(exponent - 16383)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id3v23_text_frames_are_read() {
        let mut tag = Vec::new();
        tag.extend_from_slice(b"ID3\x03\x00\x00");
        tag.extend_from_slice(&[0, 0, 0, 40]);
        let frame = |id: &[u8], text: &[u8]| {
            let mut f = Vec::new();
            f.extend_from_slice(id);
            f.extend_from_slice(&(text.len() as u32).to_be_bytes());
            f.extend_from_slice(&[0, 0]);
            f.extend_from_slice(text);
            f
        };
        tag.extend(frame(b"TIT2", b"\x00Test title"));
        tag.extend(frame(b"TPE1", b"\x00Test artist"));
        tag.extend(frame(b"TALB", b"\x00Test album"));
        tag.extend(frame(b"TRCK", b"\x003/12"));
        tag.extend_from_slice(&[0; 40]);

        let mut meta = AudioMetadata::default();
        apply_id3(&tag, &mut meta);
        assert_eq!(meta.title, "Test title");
        assert_eq!(meta.artist, "Test artist");
        assert_eq!(meta.album, "Test album");
        assert_eq!(meta.track_no, 3);
    }

    #[test]
    fn id3v24_syncsafe_frame_sizes_are_read() {
        let mut tag = Vec::new();
        tag.extend_from_slice(b"ID3\x04\x00\x00");
        tag.extend_from_slice(&[0, 0, 0, 40]);
        let body = b"\x03A longer title";
        let mut frame = Vec::new();
        frame.extend_from_slice(b"TIT2");
        let size = body.len();
        frame.push(((size >> 21) & 0x7f) as u8);
        frame.push(((size >> 14) & 0x7f) as u8);
        frame.push(((size >> 7) & 0x7f) as u8);
        frame.push((size & 0x7f) as u8);
        frame.extend_from_slice(&[0, 0]);
        frame.extend_from_slice(body);
        tag.extend(frame);
        tag.extend_from_slice(&[0; 40]);

        let mut meta = AudioMetadata::default();
        apply_id3(&tag, &mut meta);
        assert_eq!(meta.title, "A longer title");
    }

    #[test]
    fn vorbis_comments_are_read() {
        let mut block = Vec::new();
        block.extend_from_slice(&4u32.to_le_bytes());
        block.extend_from_slice(b"test");
        block.extend_from_slice(&4u32.to_le_bytes());
        for entry in ["TITLE=Song", "ARTIST=Band", "TRACKNUMBER=7"] {
            block.extend_from_slice(&(entry.len() as u32).to_le_bytes());
            block.extend_from_slice(entry.as_bytes());
        }
        let mut meta = AudioMetadata::default();
        parse_vorbis_comment(&block, &mut meta);
        assert_eq!(meta.title, "Song");
        assert_eq!(meta.artist, "Band");
        assert_eq!(meta.track_no, 7);
    }

    #[test]
    fn ai_ff_extended_rates_are_read() {
        // 44100 in 80-bit extended: 0x400E AC44 0000 0000 0000
        let bytes = [0x40, 0x0e, 0xac, 0x44, 0, 0, 0, 0, 0, 0];
        assert!((read_extended(&bytes) - 44100.0).abs() < 0.5);
    }

    #[test]
    fn filenames_become_titles_without_track_numbers() {
        assert_eq!(
            title_from_filename(Path::new("/music/01 - First Song.mp3")),
            "First Song"
        );
        assert_eq!(
            title_from_filename(Path::new("/music/First_Song.flac")),
            "First Song"
        );
    }
}
