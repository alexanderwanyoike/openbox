use super::{DATABASE, DeviceCache, MANIFEST, SourceLink, safe_path};
use openbox_domain::Library;
use openbox_storage as storage;
use std::path::Path;

pub(super) fn commit(
    root: &Path,
    pdb: &[u8],
    expected: Option<&[u8]>,
    library: &Library,
    sources: &[SourceLink],
) -> Result<(), String> {
    let mut library = library.clone();
    for track in library.tracks().to_vec() {
        let relative = track
            .path
            .strip_prefix(root)
            .map_err(|_| "Song is outside this USB library")?;
        library.set_path(track.id, relative.into());
    }
    let cache = DeviceCache {
        library,
        database_fingerprint: storage::fingerprint_bytes(pdb),
        sources: sources.to_vec(),
    };
    let bytes = serde_json::to_vec_pretty(&cache).map_err(|e| e.to_string())?;
    let database_path = safe_path(root, DATABASE)?;
    let manifest_path = safe_path(root, MANIFEST)?;
    let previous = if database_path.exists() {
        Some(std::fs::read(&database_path).map_err(|e| e.to_string())?)
    } else {
        None
    };
    if previous.as_deref() != expected {
        return Err("USB library changed. Refresh it before saving.".into());
    }
    let staged_database = storage::prepare(&database_path, pdb)?;
    let staged_manifest = storage::prepare(&manifest_path, &bytes)?;
    if manifest_path.is_dir() {
        return Err("Device manifest is a directory".into());
    }
    if let Some(bytes) = &previous {
        let backups = safe_path(root, ".openbox/backups")?;
        std::fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
        let backup = tempfile::Builder::new()
            .prefix("save-")
            .tempdir_in(&backups)
            .map_err(|e| e.to_string())?;
        storage::write_atomic(&backup.path().join("export.pdb"), bytes)?;
        std::fs::File::open(backup.path())
            .and_then(|d| d.sync_all())
            .map_err(|e| format!("Could not sync USB backup: {e}"))?;
        let _ = backup.keep();
        std::fs::File::open(backups)
            .and_then(|d| d.sync_all())
            .map_err(|e| format!("Could not sync backup directory: {e}"))?;
    }
    let current = match std::fs::read(&database_path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    if current.as_deref() != expected {
        return Err("USB library changed. Refresh it before saving.".into());
    }
    staged_database
        .persist(&database_path)
        .map_err(|e| e.to_string())?;
    if let Err(error) = staged_manifest.persist(&manifest_path) {
        // Roll back a normal cache-write failure; a crash is detected by the cache fingerprint.
        let rollback = match previous {
            Some(bytes) => storage::write_atomic(&database_path, &bytes),
            None => std::fs::remove_file(&database_path).map_err(|e| e.to_string()),
        };
        return Err(match rollback {
            Ok(()) => format!("Device save failed; the previous library was restored: {error}"),
            Err(rollback) => {
                format!("Device save failed: {error}. Could not restore database: {rollback}")
            }
        });
    }
    Ok(())
}

pub(super) struct DeviceWrite {
    _file: std::fs::File,
}

impl DeviceWrite {
    pub(super) fn acquire(root: &Path) -> Result<Self, String> {
        let path = safe_path(root, ".openbox/write.lock")?;
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let file = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| e.to_string())?;
        file.try_lock()
            .map_err(|e| format!("Cannot start USB write. Another write may be running: {e}"))?;
        // Keep one lock inode. Unlinking it could allow writers to lock different files.
        Ok(Self { _file: file })
    }
}
