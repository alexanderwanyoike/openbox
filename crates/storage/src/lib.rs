use openbox_domain::Library;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

pub fn fingerprint(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub fn fingerprint_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn save(path: &Path, library: &Library) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(library).map_err(|e| e.to_string())?;
    write_atomic(path, &data)
}

pub fn load(path: &Path) -> Result<Library, String> {
    match std::fs::read(path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| format!("Cannot read library: {e}"))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Library::default()),
        Err(error) => Err(error.to_string()),
    }
}

pub fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    prepare(path, data)?
        .persist(path)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn prepare(path: &Path, data: &[u8]) -> Result<tempfile::NamedTempFile, String> {
    let parent = path.parent().ok_or("File has no parent directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(data).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_flat_libraries_load_with_playlists_at_the_root() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("library.json");
        std::fs::write(
            &file,
            r#"{"tracks":[],"playlists":[{"id":7,"name":"Opening","track_ids":[]}]}"#,
        )
        .unwrap();
        let library = load(&file).unwrap();
        assert!(library.folders().is_empty());
        assert_eq!(library.playlist(7).unwrap().parent, None);
        assert_eq!(library.children(None)[0].id(), 7);
    }

    #[test]
    fn restarting_restores_tracks_and_playlist_order() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("library.json");
        let mut library = Library::default();
        let a = library.import("/music/a.wav".into(), "A".into(), "Artist".into());
        let b = library.import("/music/b.wav".into(), "B".into(), "Artist".into());
        let playlist = library.create_playlist("Set").unwrap();
        library.add_to_playlist(playlist, b).unwrap();
        library.add_to_playlist(playlist, a).unwrap();
        save(&file, &library).unwrap();

        let restored = load(&file).unwrap();
        assert_eq!(restored.tracks(), library.tracks());
        assert_eq!(restored.playlists(), library.playlists());
    }
}
