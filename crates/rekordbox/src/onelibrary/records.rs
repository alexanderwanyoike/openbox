use super::super::database::{Database, half, string, word};
use super::error;
use baken_export::pdb::fixed::{CATEGORY_ROWS, COLORS, COLUMNS, SORT_ROWS};
use rusqlite::{Connection, params, params_from_iter, types::Value};

pub(super) fn insert(db: &Connection, legacy: &Database) -> Result<(), String> {
    for row in legacy.rows(0)? {
        row.get(..94).ok_or("Truncated device track")?;
        let text = |i: usize| string(row, half(row, 0x5e + i * 2)? as usize).map(Value::Text);
        let number = |offset| word(row, offset).map(|v| Value::Integer(v.into()));
        let small = |offset| half(row, offset).map(|v| Value::Integer(v.into()));
        let id = |offset| {
            word(row, offset).map(|v| {
                if v == 0 {
                    Value::Null
                } else {
                    Value::Integer(v.into())
                }
            })
        };
        let values = [
            ("content_id", number(72)?),
            ("title", text(17)?),
            ("titleForSearch", Value::Null),
            ("subtitle", text(12)?),
            ("bpmx100", number(56)?),
            ("length", small(84)?),
            ("trackNo", number(52)?),
            ("discNo", small(76)?),
            ("artist_id_artist", id(68)?),
            ("artist_id_remixer", id(44)?),
            ("artist_id_originalArtist", id(36)?),
            ("artist_id_composer", id(12)?),
            ("artist_id_lyricist", 0.into()),
            ("album_id", id(64)?),
            ("genre_id", id(60)?),
            ("label_id", id(40)?),
            ("key_id", id(32)?),
            ("color_id", i64::from(row[88]).into()),
            ("image_id", id(28)?),
            ("djComment", text(16)?),
            ("rating", i64::from(row[89]).into()),
            ("releaseYear", small(80)?),
            ("releaseDate", text(11)?),
            ("dateCreated", text(10)?),
            ("dateAdded", text(10)?),
            ("path", text(20)?),
            ("fileName", text(19)?),
            ("fileSize", number(16)?),
            ("fileType", small(90)?),
            ("bitrate", number(48)?),
            ("bitDepth", small(82)?),
            ("samplingRate", number(8)?),
            ("isrc", Value::Text(String::new())),
            ("djPlayCount", small(78)?),
            ("isHotCueAutoLoadOn", 1.into()),
            ("isKuvoDeliverStatusOn", 1.into()),
            ("kuvoDeliveryComment", Value::Text(String::new())),
            ("masterDbId", number(24)?),
            ("masterContentId", number(72)?),
            ("analysisDataFilePath", text(14)?),
            ("analysedBits", small(86)?),
            ("contentLink", number(4)?),
            ("hasModified", 0.into()),
            ("cueUpdateCount", Value::Null),
            ("analysisDataUpdateCount", Value::Null),
            ("informationUpdateCount", Value::Null),
        ];
        db.execute(
            &format!(
                "INSERT INTO content ({}) VALUES ({})",
                values
                    .iter()
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
                    .join(","),
                vec!["?"; values.len()].join(",")
            ),
            params_from_iter(values.into_iter().map(|(_, value)| value)),
        )
        .map_err(error)?;
    }
    for (kind, table, offset) in [
        (1, "genre", 4),
        (4, "label", 4),
        (5, "key", 8),
        (13, "image", 4),
    ] {
        for row in legacy.rows(kind)? {
            db.execute(
                &format!("INSERT INTO {table} VALUES (?,?)"),
                params![word(row, 0)?, string(row, offset)?],
            )
            .map_err(error)?;
        }
    }
    for row in legacy.rows(2)? {
        let offset = if half(row, 0)? == 0x64 {
            half(row, 10)? as usize
        } else {
            *row.get(9).ok_or("Truncated artist")? as usize
        };
        db.execute(
            "INSERT INTO artist VALUES (?,?,NULL)",
            params![word(row, 4)?, string(row, offset)?],
        )
        .map_err(error)?;
    }
    for row in legacy.rows(3)? {
        db.execute(
            "INSERT INTO album VALUES (?,?,?,NULL,0,NULL)",
            params![
                word(row, 12)?,
                string(row, *row.get(21).ok_or("Truncated album")? as usize)?,
                word(row, 8)?
            ],
        )
        .map_err(error)?;
    }
    for row in legacy.rows(7)? {
        db.execute(
            "INSERT INTO playlist VALUES (?,?,?,NULL,?,?)",
            params![
                word(row, 12)?,
                word(row, 8)?,
                string(row, 20)?,
                word(row, 16)?,
                word(row, 0)?
            ],
        )
        .map_err(error)?;
    }
    for row in legacy.rows(8)? {
        db.execute(
            "INSERT INTO playlist_content VALUES (?,?,?)",
            params![word(row, 8)?, word(row, 4)?, word(row, 0)?],
        )
        .map_err(error)?;
    }
    for (id, name, _) in COLORS {
        db.execute("INSERT INTO color VALUES (?,?)", params![id, name])
            .map_err(error)?;
    }
    for (id, kind, name) in COLUMNS {
        db.execute(
            "INSERT INTO menuItem VALUES (?,?,?)",
            params![id, kind, name],
        )
        .map_err(error)?;
    }
    for r in CATEGORY_ROWS {
        db.execute(
            "INSERT INTO category VALUES (?,?,?,?)",
            params![r[2], r[0], r[6], r[5] != 1],
        )
        .map_err(error)?;
    }
    for r in SORT_ROWS {
        db.execute(
            "INSERT INTO sort VALUES (?,?,?,?,?)",
            params![r[2], r[0], r[5], r[4] == 0, r[6]],
        )
        .map_err(error)?;
    }
    db.execute(
        "INSERT INTO property VALUES (?,'1000',?,?,0,0)",
        params![
            "Openbox",
            legacy.rows(0)?.len() as i64,
            baken_export::build::today()
        ],
    )
    .map_err(error)?;
    Ok(())
}
