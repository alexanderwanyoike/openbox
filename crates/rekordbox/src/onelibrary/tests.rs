use crate::tests::collection;
use crate::{DATABASE, MANIFEST, export, onelibrary, open, recover, save_playlists};
use openbox_storage as storage;
use std::path::Path;

#[test]
fn export_publishes_an_encrypted_onelibrary_alongside_the_legacy_database() {
    let music = tempfile::tempdir().unwrap();
    let usb = tempfile::tempdir().unwrap();
    let (library, playlist, _) = collection(music.path());
    export(&library, &[playlist], usb.path()).unwrap();

    let modern = std::fs::read(usb.path().join("PIONEER/rekordbox/exportLibrary.db"))
        .expect("An export must support OneLibrary players as well as legacy players");
    assert!(!modern.starts_with(b"SQLite format 3"));
    assert!(modern.len() >= 4096);
    assert_eq!(modern.len() % 4096, 0);
    assert_eq!(open(usb.path()).unwrap().library.playlists().len(), 1);
}

#[test]
fn onelibrary_playlist_edits_preserve_cues_history_and_unknown_record_fields() {
    let music = tempfile::tempdir().unwrap();
    let usb = tempfile::tempdir().unwrap();
    let (library, a, b) = collection(music.path());
    export(&library, &[a, b], usb.path()).unwrap();
    let path = usb.path().join(onelibrary::DATABASE);
    {
        let db = onelibrary::connect(&path).unwrap();
        db.execute_batch(
            "ALTER TABLE content ADD COLUMN future TEXT DEFAULT 'retained';
             ALTER TABLE playlist_content ADD COLUMN future TEXT DEFAULT 'membership';
             UPDATE content SET djPlayCount=27, cueUpdateCount=8;
             INSERT INTO cue(cue_id,content_id,inUsec) SELECT 47,content_id,123456 FROM content;
             INSERT INTO history VALUES(91,0,'Last night',0,0);
             INSERT INTO history_content SELECT 91,content_id,1 FROM content;
             CREATE TABLE future_table(data BLOB);
             INSERT INTO future_table VALUES(X'00FF3142');",
        )
        .unwrap();
    }
    let mut device = open(usb.path()).unwrap();
    let id = device
        .library
        .playlists()
        .iter()
        .find(|p| p.name == "Opening")
        .unwrap()
        .id;
    device.library.rename_playlist(id, "Warm up").unwrap();
    save_playlists(&device).unwrap();

    let db = onelibrary::connect(&path).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT name FROM playlist WHERE playlist_id=?",
            [id as i64],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "Warm up"
    );
    assert_eq!(
        db.query_row("SELECT name FROM playlist WHERE name='Closing'", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        "Closing"
    );
    assert_eq!(
        db.query_row(
            "SELECT djPlayCount,cueUpdateCount,future FROM content",
            [],
            |r| Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?
            ))
        )
        .unwrap(),
        (27, 8, "retained".into())
    );
    assert_eq!(
        db.query_row("SELECT inUsec FROM cue WHERE cue_id=47", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        123456
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM history_content", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT hex(data) FROM future_table", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "00FF3142"
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM playlist_content WHERE future='membership'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn a_playlist_present_only_in_onelibrary_is_never_silently_removed() {
    let music = tempfile::tempdir().unwrap();
    let usb = tempfile::tempdir().unwrap();
    let (library, playlist, _) = collection(music.path());
    export(&library, &[playlist], usb.path()).unwrap();
    {
        let db = onelibrary::connect(&usb.path().join(onelibrary::DATABASE)).unwrap();
        db.execute(
            "INSERT INTO playlist VALUES (999,1,'Modern only',NULL,0,0)",
            [],
        )
        .unwrap();
    }
    let mut device = open(usb.path()).unwrap();
    let id = device.library.playlists()[0].id;
    device.library.rename_playlist(id, "Renamed").unwrap();
    let before = openbox_test_support::files(usb.path());
    let error = save_playlists(&device)
        .expect_err("Different device libraries must be reconciled rather than overwritten");
    assert!(error.contains("differ"), "{error}");
    assert_eq!(openbox_test_support::files(usb.path()), before);
}

#[test]
fn saving_refuses_an_active_onelibrary_wal_without_modifying_any_files() {
    let music = tempfile::tempdir().unwrap();
    let usb = tempfile::tempdir().unwrap();
    let (library, playlist, _) = collection(music.path());
    export(&library, &[playlist], usb.path()).unwrap();
    let db = onelibrary::connect(&usb.path().join(onelibrary::DATABASE)).unwrap();
    db.execute_batch("PRAGMA wal_autocheckpoint=0; UPDATE content SET djPlayCount=31")
        .unwrap();
    assert!(
        std::fs::metadata(usb.path().join(format!("{}-wal", onelibrary::DATABASE)))
            .unwrap()
            .len()
            > 0
    );
    let device = open(usb.path()).unwrap();
    let before = openbox_test_support::files(usb.path());
    let error = save_playlists(&device)
        .expect_err("An active SQLite writer must not have its database replaced");
    assert!(error.contains("Close rekordbox"), "{error}");
    assert_eq!(openbox_test_support::files(usb.path()), before);
    assert_eq!(
        db.query_row("SELECT djPlayCount FROM content", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        31
    );
}

#[test]
fn a_onelibrary_only_usb_opens_without_writing_and_can_receive_playlists() {
    let music = tempfile::tempdir().unwrap();
    let usb = tempfile::tempdir().unwrap();
    let (library, a, b) = collection(music.path());
    let exported = export(&library, &[a], usb.path()).unwrap();
    std::fs::remove_file(usb.path().join(DATABASE)).unwrap();
    let before = openbox_test_support::files(usb.path());
    let device = open(usb.path()).expect("OneLibrary devices must not require export.pdb");
    assert_eq!(device.library.tracks(), exported.library.tracks());
    assert_eq!(device.library.playlists(), exported.library.playlists());
    assert_eq!(openbox_test_support::files(usb.path()), before);

    let updated = export(&library, &[b], usb.path()).unwrap();
    assert_eq!(updated.library.tracks().len(), 1);
    assert_eq!(updated.library.playlists().len(), 2);
    let db = onelibrary::connect(&usb.path().join(onelibrary::DATABASE)).unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM playlist", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert!(usb.path().join(DATABASE).is_file());
}

#[test]
fn interrupted_dual_database_publication_is_read_only_until_a_write_restores_the_pair() {
    let music = tempfile::tempdir().unwrap();
    let usb = tempfile::tempdir().unwrap();
    let (library, playlist, _) = collection(music.path());
    let device = export(&library, &[playlist], usb.path()).unwrap();
    let before = openbox_test_support::files(usb.path());
    let backup = usb.path().join(".openbox/backups/save-interrupted");
    std::fs::create_dir_all(&backup).unwrap();
    let mut journal_files = Vec::new();
    for path in [DATABASE, onelibrary::DATABASE, MANIFEST] {
        let bytes = std::fs::read(usb.path().join(path)).unwrap();
        std::fs::write(backup.join(Path::new(path).file_name().unwrap()), &bytes).unwrap();
        let hash = storage::fingerprint_bytes(&bytes);
        journal_files.push(serde_json::json!({"path":path,"before":hash,"after":hash}));
    }
    let mut updated = device.library.clone();
    updated
        .rename_playlist(updated.playlists()[0].id, "Interrupted rename")
        .unwrap();
    let published = device.database.with_playlists(&updated).unwrap();
    journal_files[0]["after"] = storage::fingerprint_bytes(&published).into();
    std::fs::write(usb.path().join(DATABASE), &published).unwrap();
    std::fs::write(
        usb.path().join(".openbox/pending.json"),
        serde_json::to_vec(&serde_json::json!({
            "version":1,"backup":"save-interrupted","files":journal_files
        }))
        .unwrap(),
    )
    .unwrap();
    let interrupted = openbox_test_support::files(usb.path());
    assert!(
        open(usb.path()).is_err(),
        "A half-published library must require recovery"
    );
    assert_eq!(
        openbox_test_support::files(usb.path()),
        interrupted,
        "Opening must never modify USB files"
    );

    let recovered = export(&library, &[playlist], usb.path()).unwrap();
    assert_eq!(recovered.library.playlists(), device.library.playlists());
    assert_eq!(recovered.library.tracks(), device.library.tracks());
    assert!(!usb.path().join(".openbox/pending.json").exists());
    for (path, bytes) in before {
        if path.starts_with("Contents") || path.starts_with("PIONEER/USBANLZ") {
            assert_eq!(std::fs::read(usb.path().join(path)).unwrap(), bytes);
        }
    }
}

#[test]
fn interrupted_save_recovery_refuses_committed_wal_updates_without_modifying_files() {
    let music = tempfile::tempdir().unwrap();
    let usb = tempfile::tempdir().unwrap();
    let (library, playlist, _) = collection(music.path());
    export(&library, &[playlist], usb.path()).unwrap();
    let backup = usb.path().join(".openbox/backups/save-wal-conflict");
    std::fs::create_dir_all(&backup).unwrap();
    let mut files = Vec::new();
    for path in [DATABASE, onelibrary::DATABASE, MANIFEST] {
        let bytes = std::fs::read(usb.path().join(path)).unwrap();
        std::fs::write(backup.join(Path::new(path).file_name().unwrap()), &bytes).unwrap();
        let hash = storage::fingerprint_bytes(&bytes);
        files.push(serde_json::json!({"path":path,"before":hash,"after":hash}));
    }
    std::fs::write(
        usb.path().join(".openbox/pending.json"),
        serde_json::to_vec(&serde_json::json!({
            "version":1,"backup":"save-wal-conflict","files":files
        }))
        .unwrap(),
    )
    .unwrap();
    let db = onelibrary::connect(&usb.path().join(onelibrary::DATABASE)).unwrap();
    db.execute_batch("PRAGMA wal_autocheckpoint=0; UPDATE content SET djPlayCount=35")
        .unwrap();
    let before = openbox_test_support::files(usb.path());
    let error = recover(usb.path())
        .err()
        .expect("Recovery must preserve edits committed after an interrupted save");
    assert!(error.contains("Close rekordbox"), "{error}");
    assert_eq!(openbox_test_support::files(usb.path()), before);
    assert_eq!(
        db.query_row("SELECT djPlayCount FROM content", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        35
    );
}
