use super::{DATABASE, DeviceCache, MANIFEST, SourceLink, safe_path};
use openbox_domain::Library;
use openbox_storage as storage;
use std::path::Path;

pub(super) fn commit(
    root: &Path,
    pdb: &[u8],
    previous_device: Option<&super::DeviceLibrary>,
    library: &Library,
    sources: &[SourceLink],
) -> Result<(), String> {
    let baseline = previous_device.map(|device| device.database.bytes.as_slice());
    let expected = previous_device
        .filter(|device| device.has_legacy)
        .map(|device| device.database.bytes.as_slice());
    let expected_modern = previous_device.and_then(|device| device.modern.as_ref());
    if super::onelibrary::Snapshot::read(root)?.as_ref() != expected_modern {
        return Err("USB OneLibrary changed. Refresh it before saving.".into());
    }
    if expected_modern
        .is_some_and(|snapshot| snapshot.wal.as_ref().is_some_and(|wal| !wal.is_empty()))
    {
        return Err("Close other DJ software and safely eject the USB before editing it in Openbox. Its OneLibrary write-ahead log is still active.".into());
    }
    let modern = super::onelibrary::create(pdb, baseline, expected_modern)?;
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
        onelibrary_fingerprint: storage::fingerprint_bytes(&modern),
        sources: sources.to_vec(),
    };
    let bytes = serde_json::to_vec_pretty(&cache).map_err(|e| e.to_string())?;
    super::journal::publish(
        root,
        &[
            (DATABASE, pdb),
            (super::onelibrary::DATABASE, &modern),
            (MANIFEST, &bytes),
        ],
        || {
            let current = super::onelibrary::snapshot::read_optional(&safe_path(root, DATABASE)?)?;
            if current.as_deref() != expected {
                return Err("USB library changed. Refresh it before saving.".into());
            }
            if super::onelibrary::Snapshot::read(root)?.as_ref() != expected_modern {
                return Err("USB OneLibrary changed. Refresh it before saving.".into());
            }
            Ok(())
        },
    )?;
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
        let lock = Self { _file: file };
        super::journal::recover(root)?;
        Ok(lock)
    }
}

impl Drop for DeviceWrite {
    fn drop(&mut self) {
        // A concurrent process spawn can inherit the descriptor and outlive this write.
        let _ = self._file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn a_completed_writer_releases_its_lock_even_if_a_descriptor_was_inherited() {
        let root = tempfile::tempdir().unwrap();
        let writer = DeviceWrite::acquire(root.path()).unwrap();
        let inherited = writer._file.try_clone().unwrap();
        drop(writer);
        let _next = DeviceWrite::acquire(root.path())
            .expect("A completed writer must not leave its lock held by inherited descriptors");
        assert!(inherited.metadata().unwrap().is_file());
    }
}
