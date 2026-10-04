mod artwork;
mod tags;

pub use artwork::{cache_cover, device_covers};
pub use tags::write_mp3;

use lofty::{file::TaggedFileExt, picture::PictureType, prelude::*, tag::ItemKey};
use openbox_domain::{Track, TrackDetails};
use std::{path::Path, process::Command};

pub fn album_artist(path: &Path) -> String {
    lofty::probe::Probe::open(path)
        .ok()
        .and_then(|p| {
            p.options(lofty::config::ParseOptions::new().read_properties(false))
                .read()
                .ok()
        })
        .and_then(|f| {
            f.primary_tag()
                .or_else(|| f.first_tag())
                .and_then(|t| t.get_string(ItemKey::AlbumArtist))
                .map(str::to_string)
        })
        .unwrap_or_default()
}

pub fn read(path: &Path, artwork: &Path) -> Result<Track, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_type:format=duration:format_tags",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .map_err(|e| format!("Could not run ffprobe: {e}"))?;
    if !output.status.success() {
        return Err("Audio file could not be read".into());
    }
    let info: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    if info["streams"].as_array().is_none_or(|a| a.is_empty()) {
        return Err("No audio stream".into());
    }
    let tagged = lofty::read_from_path(path).ok();
    let primary = tagged
        .as_ref()
        .and_then(|f| f.primary_tag().or_else(|| f.first_tag()));
    let text = |key: ItemKey, aliases: &[&str]| -> String {
        primary
            .and_then(|tag| tag.get_string(key))
            .map(str::to_string)
            .or_else(|| {
                info["format"]["tags"].as_object().and_then(|tags| {
                    tags.iter()
                        .find(|(name, _)| aliases.iter().any(|a| name.eq_ignore_ascii_case(a)))
                        .and_then(|(_, value)| value.as_str())
                        .map(str::to_string)
                })
            })
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    let number = |value: String| {
        value
            .split(['/', '-'])
            .next()
            .unwrap_or("")
            .parse::<u32>()
            .unwrap_or(0)
    };
    let title = text(ItemKey::TrackTitle, &["title"]);
    let artist = text(ItemKey::TrackArtist, &["artist"]);
    let stem = path
        .file_stem()
        .and_then(|p| p.to_str())
        .unwrap_or("Untitled");
    let (fallback_artist, fallback_title) = stem.split_once(" - ").unwrap_or(("", stem));
    let cover = tagged.as_ref().and_then(|f| {
        let mut pictures = f.tags().iter().flat_map(|tag| tag.pictures());
        pictures
            .clone()
            .find(|p| p.pic_type() == PictureType::CoverFront)
            .or_else(|| pictures.next())
    });
    let artwork = cover.map(|p| cache_cover(p.data(), artwork)).transpose()?;
    Ok(Track {
        id: 0,
        path: path.into(),
        title: if title.is_empty() {
            fallback_title.into()
        } else {
            title
        },
        artist: if artist.is_empty() {
            fallback_artist.into()
        } else {
            artist
        },
        details: TrackDetails {
            album: text(ItemKey::AlbumTitle, &["album"]),
            album_artist: text(ItemKey::AlbumArtist, &["album_artist"]),
            genre: text(ItemKey::Genre, &["genre"]),
            composer: text(ItemKey::Composer, &["composer"]),
            comment: text(ItemKey::Comment, &["comment", "description"]),
            key: text(ItemKey::InitialKey, &["initialkey", "initial_key", "key"]),
            bpm: primary
                .and_then(|t| {
                    t.get_string(ItemKey::Bpm)
                        .or_else(|| t.get_string(ItemKey::IntegerBpm))
                })
                .map(str::to_string)
                .unwrap_or_else(|| text(ItemKey::Bpm, &["bpm", "tbpm"]))
                .parse::<f64>()
                .ok()
                .filter(|b| b.is_finite() && *b > 0.0 && *b < 655.35),
            year: number(text(ItemKey::RecordingDate, &["date", "year"])),
            track_number: primary
                .and_then(|t| t.track())
                .unwrap_or_else(|| number(text(ItemKey::TrackNumber, &["track"]))),
            disc_number: primary
                .and_then(|t| t.disk())
                .unwrap_or_else(|| number(text(ItemKey::DiscNumber, &["disc"]))),
            duration_seconds: info["format"]["duration"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0)
                .round() as u32,
            artwork,
            ..Default::default()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lofty::{
        config::WriteOptions,
        picture::{Picture, PictureType},
        tag::{ItemKey, Tag, TagType},
    };

    #[test]
    fn imports_id3_metadata_and_front_cover_without_changing_the_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("untagged.mp3");
        openbox_test_support::mp3(&path);
        let mut tag = Tag::new(TagType::Id3v2);
        tag.set_title("Tagged title".into());
        tag.set_artist("Tagged artist".into());
        tag.set_album("Album".into());
        tag.set_genre("House".into());
        tag.insert_text(ItemKey::RecordingDate, "2024".into());
        tag.set_track(7);
        tag.insert_text(ItemKey::AlbumArtist, "Various artists".into());
        tag.insert_text(ItemKey::Composer, "Composer".into());
        tag.insert_text(ItemKey::Comment, "Keep me".into());
        tag.insert_text(ItemKey::IntegerBpm, "120".into());
        tag.insert_text(ItemKey::InitialKey, "Am".into());
        let mut data = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(600, 400, image::Rgb([210, 50, 100]))
            .write_to(&mut data, image::ImageFormat::Png)
            .unwrap();
        let mut picture =
            Picture::from_reader(&mut std::io::Cursor::new(data.into_inner())).unwrap();
        picture.set_pic_type(PictureType::CoverFront);
        tag.push_picture(picture);
        tag.save_to_path(&path, WriteOptions::default()).unwrap();
        let source = std::fs::read(&path).unwrap();
        let track = read(&path, &dir.path().join("artwork")).unwrap();
        assert_eq!(track.title, "Tagged title");
        assert_eq!(track.artist, "Tagged artist");
        assert_eq!(track.details.album_artist, "Various artists");
        assert_eq!(track.details.composer, "Composer");
        assert_eq!(track.details.year, 2024);
        assert_eq!(track.details.track_number, 7);
        assert_eq!(track.details.comment, "Keep me");
        assert_eq!(track.details.bpm, Some(120.0));
        assert_eq!(track.details.key, "Am");
        let cover = image::open(track.details.artwork.as_ref().unwrap()).unwrap();
        assert_eq!((cover.width(), cover.height()), (500, 333));
        assert_eq!(std::fs::read(path).unwrap(), source);
    }
}
