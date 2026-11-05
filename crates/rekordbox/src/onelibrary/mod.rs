mod key;
mod merge;
mod read;
mod records;
pub(super) use read::legacy;
pub(super) mod snapshot;
pub(crate) use snapshot::Snapshot;

use super::database::Database;
use rusqlite::Connection;
use std::path::Path;

pub(super) const DATABASE: &str = "PIONEER/rekordbox/exportLibrary.db";

pub(super) fn create(
    legacy: &[u8],
    baseline: Option<&[u8]>,
    previous: Option<&Snapshot>,
) -> Result<Vec<u8>, String> {
    let folder = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = folder.path().join("incoming.db");
    fresh(&path, legacy)?;
    if let Some(previous) = previous {
        let baseline = baseline.ok_or("OneLibrary has no corresponding traditional library")?;
        let baseline_path = folder.path().join("baseline.db");
        fresh(&baseline_path, baseline)?;
        let existing = folder.path().join("existing.db");
        previous.copy_to(&existing)?;
        let mut db = connect(&existing)?;
        merge::apply(&mut db, &path, &baseline_path)?;
        finish(db)?;
        std::fs::read(existing).map_err(|e| e.to_string())
    } else {
        std::fs::read(path).map_err(|e| e.to_string())
    }
}

fn fresh(path: &Path, legacy: &[u8]) -> Result<(), String> {
    let mut db = connect(path)?;
    db.pragma_update(None, "journal_mode", "WAL")
        .map_err(error)?;
    db.execute_batch(include_str!("schema.sql"))
        .map_err(error)?;
    let tx = db.transaction().map_err(error)?;
    records::insert(&tx, &Database::read(legacy.to_vec())?)?;
    tx.commit().map_err(error)?;
    finish(db)
}

fn finish(db: Connection) -> Result<(), String> {
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .map_err(error)?;
    db.close().map_err(|(_, e)| error(e))
}

pub(super) fn connect(path: &Path) -> Result<Connection, String> {
    let db = Connection::open(path).map_err(error)?;
    db.pragma_update(None, "key", key::passphrase()?)
        .map_err(error)?;
    db.pragma_update(None, "cipher_compatibility", 4)
        .map_err(error)?;
    db.pragma_update(None, "trusted_schema", false)
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
