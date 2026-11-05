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

#[test]
fn startup_offers_recovery_for_interrupted_saves_without_modifying_the_device() {
    use iced_graphics::core::{
        Rectangle,
        widget::{Id, Operation},
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    struct RecoveryButton(bool);
    impl Operation for RecoveryButton {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }
        fn container(&mut self, id: Option<&Id>, _: Rectangle) {
            self.0 |= id.is_some_and(|id| *id == Id::new("usb-recovery"));
        }
    }
    let (folder, root) = device_fixture();
    let backup = root.join(".openbox/backups/save-desktop-test");
    std::fs::create_dir_all(&backup).unwrap();
    let files: Vec<_> = [
        "PIONEER/rekordbox/export.pdb",
        "PIONEER/rekordbox/exportLibrary.db",
        ".openbox/device.json",
    ]
    .into_iter()
    .map(|path| {
        let bytes = std::fs::read(root.join(path)).unwrap();
        std::fs::write(
            backup.join(std::path::Path::new(path).file_name().unwrap()),
            &bytes,
        )
        .unwrap();
        let hash = openbox_storage::fingerprint_bytes(&bytes);
        serde_json::json!({"path":path,"before":hash,"after":hash})
    })
    .collect();
    std::fs::write(
        root.join(".openbox/pending.json"),
        serde_json::to_vec(
            &serde_json::json!({"version":1,"backup":"save-desktop-test","files":files}),
        )
        .unwrap(),
    )
    .unwrap();
    let before = openbox_test_support::files(&root);
    let app =
        App::load(folder.path().join("library.json"), Some(root.clone())).unwrap_or_else(|error| {
            panic!("Interrupted USB saves must allow the desktop to start: {error}")
        });
    assert!(app.workspace.device.is_none());
    assert!(app.status.contains("Recover USB"), "{}", app.status);
    let mut renderer = super::rendering::new_renderer();
    let mut ui = UserInterface::build(
        app.view(),
        iced::Size::new(1440.0, 900.0),
        Cache::new(),
        &mut renderer,
    );
    let mut recovery = RecoveryButton(false);
    ui.operate(&renderer, &mut recovery);
    assert!(recovery.0, "The device panel must offer a recovery action");
    assert_eq!(openbox_test_support::files(&root), before);
}

#[test]
fn usb_write_completion_reconciles_recovery_availability_and_device_views() {
    let completions: [fn(Result<DeviceLibrary, String>) -> Message; 2] =
        [Message::Exported, Message::UsbUpdated];
    for complete in completions {
        let (folder, root) = device_fixture();
        let mut app = App::load(folder.path().join("library.json"), Some(root.clone())).unwrap();
        let healthy = app.workspace.device.clone().unwrap();
        std::fs::write(root.join(".openbox/pending.json"), b"{}").unwrap();
        let _ = app.update(complete(Err("Interrupted USB write".into())));
        assert!(
            app.usb_recovery,
            "A failed write must expose its pending recovery"
        );
        assert!(
            app.workspace.device.is_none(),
            "An interrupted device view must not remain editable"
        );
        std::fs::remove_file(root.join(".openbox/pending.json")).unwrap();
        let _ = app.update(complete(Ok(healthy)));
        assert!(
            !app.usb_recovery,
            "A successful export must remove the recovery action"
        );
        assert!(app.workspace.device.is_some());
    }
    let (folder, root) = device_fixture();
    let mut app = App::load(folder.path().join("library.json"), Some(root.clone())).unwrap();
    app.target = Target::Usb;
    let id = app.workspace.device.as_ref().unwrap().library.playlists()[0].id;
    std::fs::write(root.join(".openbox/pending.json"), b"{}").unwrap();
    assert!(!app.edit(Edit::Rename(id, "Rename".into()), "Renamed"));
    assert!(
        app.usb_recovery,
        "Synchronous playlist edits must also expose pending recovery"
    );
    assert!(app.workspace.device.is_none());
}
