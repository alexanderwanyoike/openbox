use super::{error, key};
use rusqlite::{Connection, params, types::Value};
use std::{
    collections::{HashMap, VecDeque},
    path::Path,
};

type Row = Vec<Value>;

pub(super) fn apply(db: &mut Connection, incoming: &Path, baseline: &Path) -> Result<(), String> {
    db.execute(
        "ATTACH DATABASE ?1 AS incoming KEY ?2",
        params![incoming.to_string_lossy(), key::passphrase()?],
    )
    .map_err(error)?;
    db.execute(
        "ATTACH DATABASE ?1 AS baseline KEY ?2",
        params![baseline.to_string_lossy(), key::passphrase()?],
    )
    .map_err(error)?;
    for selection in [
        "content_id,path,analysisDataFilePath FROM {schema}.content ORDER BY content_id",
        "playlist_id,sequenceNo,name,attribute,playlist_id_parent FROM {schema}.playlist ORDER BY playlist_id",
        "playlist_id,content_id,sequenceNo FROM {schema}.playlist_content ORDER BY playlist_id,sequenceNo,rowid",
    ] {
        let current = rows(
            db,
            &format!("SELECT {}", selection.replace("{schema}", "main")),
        )?;
        let original = rows(
            db,
            &format!("SELECT {}", selection.replace("{schema}", "baseline")),
        )?;
        if current != original {
            return Err("The traditional and OneLibrary records differ. Synchronize this USB in rekordbox before editing it.".into());
        }
    }
    let tx = db.transaction().map_err(error)?;
    let triggers: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type='trigger'",
            [],
            |r| r.get(0),
        )
        .map_err(error)?;
    if triggers != 0 {
        return Err("This OneLibrary contains custom triggers and cannot be edited safely".into());
    }
    let integrity: String = tx
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(error)?;
    if integrity != "ok" {
        return Err(format!("OneLibrary integrity check failed: {integrity}"));
    }
    for (table, id) in [
        ("genre", "genre_id"),
        ("artist", "artist_id"),
        ("album", "album_id"),
        ("label", "label_id"),
        ("key", "key_id"),
        ("image", "image_id"),
        ("content", "content_id"),
    ] {
        let identity = if table == "content" {
            "path,analysisDataFilePath"
        } else if table == "image" {
            "path"
        } else {
            "name"
        };
        let before = rows(&tx, &format!("SELECT {id},{identity} FROM main.{table}"))?;
        let after = rows(
            &tx,
            &format!("SELECT {id},{identity} FROM incoming.{table}"),
        )?;
        let map: HashMap<_, _> = after
            .iter()
            .filter_map(|r| integer(&r[0]).map(|id| (id, &r[1..])))
            .collect();
        for row in &before {
            let id_value = integer(&row[0]).ok_or("Invalid OneLibrary identity")?;
            if map.get(&id_value).copied() != Some(&row[1..]) {
                return Err(format!(
                    "The traditional and OneLibrary {table} records differ. Synchronize this USB in rekordbox before editing it."
                ));
            }
        }
        let columns: Vec<String> = rows(&tx, &format!("PRAGMA incoming.table_info({table})"))?
            .into_iter()
            .map(|r| match r[1].clone() {
                Value::Text(name) => Ok(name),
                _ => Err("Invalid generated schema".to_string()),
            })
            .collect::<Result<_, _>>()?;
        let names = columns.join(",");
        tx.execute(&format!("INSERT INTO main.{table} ({names}) SELECT {names} FROM incoming.{table} AS source WHERE NOT EXISTS (SELECT 1 FROM main.{table} AS target WHERE target.{id}=source.{id})"), []).map_err(error)?;
    }
    playlists(&tx)?;
    tx.execute(
        "UPDATE property SET numberOfContents=(SELECT COUNT(*) FROM content)",
        [],
    )
    .map_err(error)?;
    if !rows(&tx, "PRAGMA foreign_key_check")?.is_empty() {
        return Err("OneLibrary contains invalid references".into());
    }
    tx.commit().map_err(error)?;
    db.execute_batch("DETACH DATABASE incoming; DETACH DATABASE baseline")
        .map_err(error)?;
    Ok(())
}

fn playlists(db: &Connection) -> Result<(), String> {
    let incoming = rows(
        db,
        "SELECT playlist_id,sequenceNo,name,attribute,playlist_id_parent FROM incoming.playlist",
    )?;
    let desired: Vec<_> = incoming.iter().filter_map(|r| integer(&r[0])).collect();
    for row in rows(db, "SELECT playlist_id FROM playlist")? {
        let id = integer(&row[0]).ok_or("Invalid OneLibrary playlist ID")?;
        if !desired.contains(&id) {
            db.execute("DELETE FROM playlist_content WHERE playlist_id=?", [id])
                .map_err(error)?;
            db.execute("DELETE FROM playlist WHERE playlist_id=?", [id])
                .map_err(error)?;
        }
    }
    for row in incoming {
        let id = integer(&row[0]).ok_or("Invalid OneLibrary playlist ID")?;
        let changed = db.execute("UPDATE playlist SET sequenceNo=?2,name=?3,attribute=?4,playlist_id_parent=?5 WHERE playlist_id=?1", rusqlite::params_from_iter(row.iter())).map_err(error)?;
        if changed == 0 {
            db.execute("INSERT INTO playlist(playlist_id,sequenceNo,name,attribute,playlist_id_parent) VALUES (?1,?2,?3,?4,?5)",rusqlite::params_from_iter(row.iter())).map_err(error)?;
        }
        let old = rows(
            db,
            &format!(
                "SELECT rowid,content_id FROM playlist_content WHERE playlist_id={id} ORDER BY sequenceNo,rowid"
            ),
        )?;
        let mut retained: HashMap<i64, VecDeque<i64>> = HashMap::new();
        for row in old {
            retained
                .entry(integer(&row[1]).ok_or("Invalid member ID")?)
                .or_default()
                .push_back(integer(&row[0]).ok_or("Invalid member row")?);
        }
        for row in rows(
            db,
            &format!(
                "SELECT content_id,sequenceNo FROM incoming.playlist_content WHERE playlist_id={id} ORDER BY sequenceNo"
            ),
        )? {
            let content = integer(&row[0]).ok_or("Invalid member ID")?;
            let order = integer(&row[1]).ok_or("Invalid member order")?;
            if let Some(rowid) = retained.get_mut(&content).and_then(VecDeque::pop_front) {
                db.execute(
                    "UPDATE playlist_content SET sequenceNo=? WHERE rowid=?",
                    params![order, rowid],
                )
                .map_err(error)?;
            } else {
                db.execute("INSERT INTO playlist_content(playlist_id,content_id,sequenceNo) VALUES (?,?,?)",params![id,content,order]).map_err(error)?;
            }
        }
        for rowid in retained.values().flatten() {
            db.execute("DELETE FROM playlist_content WHERE rowid=?", [rowid])
                .map_err(error)?;
        }
    }
    Ok(())
}

fn integer(value: &Value) -> Option<i64> {
    if let Value::Integer(id) = value {
        Some(*id)
    } else {
        None
    }
}

fn rows(db: &Connection, sql: &str) -> Result<Vec<Row>, String> {
    let mut statement = db.prepare(sql).map_err(error)?;
    let columns = statement.column_count();
    statement
        .query_map([], |r| (0..columns).map(|i| r.get(i)).collect())
        .map_err(error)?
        .collect::<Result<_, _>>()
        .map_err(error)
}
