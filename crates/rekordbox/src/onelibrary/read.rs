use super::{Snapshot, connect, error};
use crate::database::Database;
use baken_export::pdb::{self, Export, ExportPlaylist, rows::TrackRow};
use rusqlite::{Connection, Row, types::FromSql};

pub(crate) fn legacy(snapshot: &Snapshot) -> Result<Database, String> {
    let folder = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = folder.path().join("exportLibrary.db");
    snapshot.copy_to(&path)?;
    let db = connect(&path)?;
    let integrity: String = db
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(error)?;
    if integrity != "ok" {
        return Err(format!("OneLibrary integrity check failed: {integrity}"));
    }
    let tracks = query(&db, "SELECT * FROM content ORDER BY content_id", |r| {
        Ok(TrackRow {
            id: value(r, "content_id")?,
            title: value(r, "title")?,
            mix_name: value(r, "subtitle")?,
            tempo: value(r, "bpmx100")?,
            duration_seconds: value(r, "length")?,
            track_number: value(r, "trackNo")?,
            disc_number: value(r, "discNo")?,
            artist_id: value(r, "artist_id_artist")?,
            remixer_id: value(r, "artist_id_remixer")?,
            original_artist_id: value(r, "artist_id_originalArtist")?,
            composer_id: value(r, "artist_id_composer")?,
            album_id: value(r, "album_id")?,
            genre_id: value(r, "genre_id")?,
            label_id: value(r, "label_id")?,
            key_id: value(r, "key_id")?,
            color_id: value(r, "color_id")?,
            artwork_id: value(r, "image_id")?,
            comment: value(r, "djComment")?,
            rating: value(r, "rating")?,
            year: value(r, "releaseYear")?,
            release_date: value(r, "releaseDate")?,
            date_added: value(r, "dateAdded")?,
            file_path: value(r, "path")?,
            filename: value(r, "fileName")?,
            file_size: value(r, "fileSize")?,
            file_type: value(r, "fileType")?,
            bitrate: value(r, "bitrate")?,
            sample_depth: value(r, "bitDepth")?,
            sample_rate: value(r, "samplingRate")?,
            play_count: value(r, "djPlayCount")?,
            analyze_path: value(r, "analysisDataFilePath")?,
            ..Default::default()
        })
    })?;
    let mut playlists = query(&db, "SELECT * FROM playlist ORDER BY playlist_id", |r| {
        Ok(ExportPlaylist {
            id: value(r, "playlist_id")?,
            parent: value(r, "playlist_id_parent")?,
            sort_order: value(r, "sequenceNo")?,
            is_folder: value(r, "attribute")?,
            name: value(r, "name")?,
            track_ids: Vec::new(),
        })
    })?;
    for p in &mut playlists {
        let mut statement = db.prepare("SELECT content_id FROM playlist_content WHERE playlist_id=? ORDER BY sequenceNo,rowid").map_err(error)?;
        p.track_ids = statement
            .query_map([p.id], |r| r.get(0))
            .map_err(error)?
            .collect::<Result<_, _>>()
            .map_err(error)?;
    }
    let names = |table: &str| {
        query(
            &db,
            &format!("SELECT {table}_id,name FROM {table} ORDER BY {table}_id"),
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
    };
    let model = Export {
        tracks,
        playlists,
        artists: names("artist")?,
        genres: names("genre")?,
        labels: names("label")?,
        keys: names("key")?,
        albums: query(
            &db,
            "SELECT album_id,COALESCE(artist_id,0),name FROM album ORDER BY album_id",
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?,
        device_name: "Openbox".into(),
        export_date: baken_export::build::today(),
    };
    let legacy = Database::read(pdb::write(&model))?;
    let images = query(
        &db,
        "SELECT image_id,path FROM image ORDER BY image_id",
        |r| Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?)),
    )?;
    let artwork = images
        .iter()
        .map(|(id, path)| pdb::rows::named(*id, path))
        .collect();
    Database::read(legacy.update(&[(13, artwork, false)])?)
}

fn value<T: FromSql + Default>(row: &Row<'_>, column: &str) -> rusqlite::Result<T> {
    Ok(row.get::<_, Option<T>>(column)?.unwrap_or_default())
}

fn query<T>(
    db: &Connection,
    sql: &str,
    row: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, String> {
    let mut statement = db.prepare(sql).map_err(error)?;
    statement
        .query_map([], row)
        .map_err(error)?
        .collect::<Result<_, _>>()
        .map_err(error)
}
