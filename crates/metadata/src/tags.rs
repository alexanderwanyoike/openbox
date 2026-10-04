use lofty::{
    config::WriteOptions,
    file::TaggedFileExt,
    picture::{Picture, PictureType},
    prelude::*,
    tag::{ItemKey, Tag, TagType},
};
use openbox_domain::Track;
use std::path::Path;

pub fn write_mp3(track: &Track, output: &Path) -> Result<(), String> {
    let file = lofty::read_from_path(output).map_err(|e| e.to_string())?;
    let mut tag = file
        .primary_tag()
        .cloned()
        .unwrap_or_else(|| Tag::new(TagType::Id3v2));
    tag.set_title(track.title.clone());
    tag.set_artist(track.artist.clone());
    tag.set_album(track.details.album.clone());
    tag.set_genre(track.details.genre.clone());
    for (key, value) in [
        (ItemKey::AlbumArtist, track.details.album_artist.clone()),
        (ItemKey::Composer, track.details.composer.clone()),
        (ItemKey::Comment, track.details.comment.clone()),
        (ItemKey::InitialKey, track.details.key.clone()),
        (
            ItemKey::IntegerBpm,
            track
                .details
                .bpm
                .map(|b| format!("{b:.0}"))
                .unwrap_or_default(),
        ),
    ] {
        if value.is_empty() {
            tag.remove_key(key);
        } else {
            tag.insert_text(key, value);
        }
    }
    if track.details.year > 0 {
        tag.insert_text(ItemKey::RecordingDate, track.details.year.to_string());
    }
    if track.details.track_number > 0 {
        tag.set_track(track.details.track_number);
    }
    if track.details.disc_number > 0 {
        tag.set_disk(track.details.disc_number);
    }
    if let Some(path) = &track.details.artwork {
        let bytes = std::fs::read(path).map_err(|e| format!("Cannot read cover: {e}"))?;
        let mut picture =
            Picture::from_reader(&mut std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
        picture.set_pic_type(PictureType::CoverFront);
        tag.set_picture(0, picture);
    }
    tag.save_to_path(output, WriteOptions::default().use_id3v23(true))
        .map_err(|e| e.to_string())
}
