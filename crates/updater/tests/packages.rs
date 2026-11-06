use openbox_updater::Package;

#[test]
fn release_packages_match_the_native_platform_and_archive_layout() {
    for (target, asset, binary) in [
        (
            "x86_64-unknown-linux-gnu",
            "openbox-x86_64-unknown-linux-gnu.AppImage",
            "openbox",
        ),
        (
            "x86_64-pc-windows-msvc",
            "openbox-x86_64-pc-windows-msvc.zip",
            "openbox.exe",
        ),
        (
            "aarch64-apple-darwin",
            "openbox-aarch64-apple-darwin.tar.gz",
            "Openbox.app/Contents/MacOS/openbox",
        ),
        (
            "x86_64-apple-darwin",
            "openbox-x86_64-apple-darwin.tar.gz",
            "Openbox.app/Contents/MacOS/openbox",
        ),
    ] {
        let package = Package::for_target(target).unwrap();
        assert_eq!(package.asset, asset);
        assert_eq!(package.binary, binary);
    }
    assert!(Package::for_target("aarch64-unknown-linux-gnu").is_err());
}
