use super::{DATABASE, MANIFEST, onelibrary, safe_path};
use onelibrary::snapshot::read_optional;
use openbox_storage as storage;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MARKER: &str = ".openbox/pending.json";
const FILES: [&str; 3] = [DATABASE, onelibrary::DATABASE, MANIFEST];

#[derive(Serialize, Deserialize)]
struct Journal {
    version: u8,
    backup: String,
    files: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    path: String,
    before: Option<String>,
    after: String,
}

pub(super) fn pending(root: &Path) -> Result<bool, String> {
    safe_path(root, MARKER)?
        .try_exists()
        .map_err(|e| e.to_string())
}

pub(super) fn publish(
    root: &Path,
    updates: &[(&str, &[u8])],
    check: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if pending(root)? {
        return Err("Recover the interrupted USB save before publishing another library".into());
    }
    let paths = updates
        .iter()
        .map(|(name, _)| safe_path(root, name))
        .collect::<Result<Vec<_>, _>>()?;
    let staged = updates
        .iter()
        .zip(&paths)
        .map(|((_, bytes), path)| storage::prepare(path, bytes))
        .collect::<Result<Vec<_>, _>>()?;
    let backups = safe_path(root, ".openbox/backups")?;
    std::fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
    let backup = tempfile::Builder::new()
        .prefix("save-")
        .tempdir_in(&backups)
        .map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    for ((name, bytes), path) in updates.iter().zip(&paths) {
        let previous = read_optional(path)?;
        if let Some(previous) = &previous {
            storage::write_atomic(
                &backup
                    .path()
                    .join(path.file_name().ok_or("Invalid device database name")?),
                previous,
            )?;
        }
        files.push(Entry {
            path: (*name).into(),
            before: previous.as_deref().map(storage::fingerprint_bytes),
            after: storage::fingerprint_bytes(bytes),
        });
    }
    sync_directory(backup.path())?;
    let journal = Journal {
        version: 1,
        backup: backup
            .path()
            .file_name()
            .ok_or("Invalid backup name")?
            .to_string_lossy()
            .into(),
        files,
    };
    check()?;
    let _ = backup.keep();
    sync_directory(&backups)?;
    let marker = safe_path(root, MARKER)?;
    storage::write_atomic(
        &marker,
        &serde_json::to_vec(&journal).map_err(|e| e.to_string())?,
    )?;
    sync_directory(marker.parent().ok_or("Missing USB metadata directory")?)?;
    let result = (|| {
        for (file, path) in staged.into_iter().zip(&paths) {
            file.persist(path).map_err(|e| e.to_string())?;
            sync_directory(path.parent().ok_or("Missing device database directory")?)?;
        }
        std::fs::remove_file(&marker).map_err(|e| e.to_string())?;
        sync_directory(marker.parent().ok_or("Missing USB metadata directory")?)
    })();
    if let Err(error) = result {
        return Err(if pending(root)? {
            match recover(root) {
                Ok(()) => format!("USB save failed; the previous libraries were restored: {error}"),
                Err(recovery) => {
                    format!("USB save failed: {error}. Recovery is required: {recovery}")
                }
            }
        } else {
            format!("USB libraries were published, but the final directory flush failed: {error}")
        });
    }
    Ok(())
}

pub(super) fn recover(root: &Path) -> Result<(), String> {
    let marker = safe_path(root, MARKER)?;
    if !marker.try_exists().map_err(|e| e.to_string())? {
        return Ok(());
    }
    let wal_path = safe_path(root, &format!("{}-wal", onelibrary::DATABASE))?;
    if read_optional(&wal_path)?.is_some_and(|wal| !wal.is_empty()) {
        return Err("Close rekordbox and safely eject the USB before recovery. Its OneLibrary write-ahead log contains newer changes.".into());
    }
    if std::fs::metadata(&marker).map_err(|e| e.to_string())?.len() > 65536 {
        return Err("USB recovery journal is too large".into());
    }
    let journal: Journal =
        serde_json::from_slice(&std::fs::read(&marker).map_err(|e| e.to_string())?)
            .map_err(|e| format!("Cannot read USB recovery journal: {e}"))?;
    if journal.version != 1
        || journal.files.len() != FILES.len()
        || !journal.backup.starts_with("save-")
        || !journal
            .backup
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || journal.files.iter().map(|e| e.path.as_str()).ne(FILES)
    {
        return Err("Unsupported USB recovery journal. Automatic recovery was refused.".into());
    }
    let backup = safe_path(root, &format!(".openbox/backups/{}", journal.backup))?;
    let mut restore = Vec::new();
    for entry in &journal.files {
        let path = safe_path(root, &entry.path)?;
        let current = read_optional(&path)?;
        let current_hash = current.as_deref().map(storage::fingerprint_bytes);
        if current_hash != entry.before && current_hash.as_deref() != Some(&entry.after) {
            return Err("USB files changed after an interrupted save. Automatic recovery was refused to preserve those changes.".into());
        }
        let previous_path = safe_backup_path(&backup, &path)?;
        let bytes = read_optional(&previous_path)?;
        if bytes.as_deref().map(storage::fingerprint_bytes) != entry.before {
            return Err(
                "The USB recovery backup is missing or damaged. No files were restored.".into(),
            );
        }
        restore.push((path, bytes));
    }
    for (path, bytes) in restore {
        match bytes {
            Some(bytes) => storage::write_atomic(&path, &bytes)?,
            None => match std::fs::remove_file(&path) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.to_string()),
            },
        }
        sync_directory(path.parent().ok_or("Missing device database directory")?)?;
    }
    std::fs::remove_file(&marker).map_err(|e| e.to_string())?;
    sync_directory(marker.parent().ok_or("Missing USB metadata directory")?)
}

fn safe_backup_path(backup: &Path, file: &Path) -> Result<PathBuf, String> {
    let name = file
        .file_name()
        .ok_or("Invalid backup filename")?
        .to_str()
        .ok_or("Invalid backup filename")?;
    safe_path(backup, name)
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    std::fs::File::open(path)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| format!("Cannot flush USB directory: {e}"))?;
    // Windows does not provide directory flushing through std::fs. Each staged file is flushed.
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
