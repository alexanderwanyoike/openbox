mod key;
mod records;

use super::{database::Database, safe_path};
use rusqlite::Connection;
use std::path::Path;

pub(super) const DATABASE: &str = "PIONEER/rekordbox/exportLibrary.db";

pub(super) fn create(root: &Path, legacy: &[u8]) -> Result<Vec<u8>, String> {
    if safe_path(root, DATABASE)?.exists() {
        return Err(
            "OneLibrary updates are not available yet. No device records were changed.".into(),
        );
    }
    let folder = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = folder.path().join("exportLibrary.db");
    let mut db = connect(&path)?;
    db.pragma_update(None, "journal_mode", "WAL")
        .map_err(error)?;
    db.execute_batch(include_str!("schema.sql"))
        .map_err(error)?;
    let tx = db.transaction().map_err(error)?;
    records::insert(&tx, &Database::read(legacy.to_vec())?)?;
    tx.commit().map_err(error)?;
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .map_err(error)?;
    db.close().map_err(|(_, e)| error(e))?;
    std::fs::read(path).map_err(|e| e.to_string())
}

pub(super) fn connect(path: &Path) -> Result<Connection, String> {
    let db = Connection::open(path).map_err(error)?;
    db.pragma_update(None, "key", key::passphrase()?)
        .map_err(error)?;
    let version: String = db
        .query_row("PRAGMA cipher_version", [], |r| r.get(0))
        .map_err(error)?;
    if version.is_empty() {
        return Err("OneLibrary requires SQLCipher support".into());
    }
    Ok(db)
}

fn error(error: rusqlite::Error) -> String {
    format!("Cannot process OneLibrary: {error}")
}
