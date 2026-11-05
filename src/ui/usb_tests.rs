use super::*;

#[test]
fn startup_opens_onelibrary_only_devices_with_their_playlist_tree() {
    let (folder, root) = device_fixture();
    std::fs::remove_file(root.join("PIONEER/rekordbox/export.pdb")).unwrap();
    let before = openbox_test_support::files(&root);

    let app = App::load(folder.path().join("library.json"), Some(root.clone())).unwrap();
    let device = app
        .workspace
        .device
        .as_ref()
        .expect("OneLibrary-only USBs must appear in Devices");
    assert_eq!(device.library.playlists()[0].name, "Warm up");
    assert_eq!(device.library.folders()[0].name, "Sets");
    assert_eq!(device.library.tracks().len(), 1);
    assert_eq!(openbox_test_support::files(&root), before);
}

fn device_fixture() -> (tempfile::TempDir, PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().join("usb");
    std::fs::create_dir(&root).unwrap();
    let audio = folder.path().join("track.wav");
    openbox_test_support::audio(&audio);
    let mut library = Library::default();
    let track = library.import(audio, "Track".into(), "Artist".into());
    let sets = library.create_folder("Sets", None).unwrap();
    let playlist = library.create_playlist_in("Warm up", Some(sets)).unwrap();
    library.add_to_playlist(playlist, track).unwrap();
    usb::export(&library, &[playlist], &root).unwrap();
    (folder, root)
}
