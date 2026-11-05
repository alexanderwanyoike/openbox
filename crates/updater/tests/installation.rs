#![cfg(target_os = "linux")]

use std::{fs, process::Command};

#[test]
fn an_appimage_updates_the_outer_file_instead_of_its_read_only_mount() {
    let directory = tempfile::tempdir().unwrap();
    let appdir = directory.path().join("Openbox.AppDir");
    fs::create_dir_all(appdir.join("usr/bin")).unwrap();
    let inner = appdir.join("usr/bin/openbox");
    fs::copy(std::env::current_exe().unwrap(), &inner).unwrap();
    let outer = if let Some(tools) = std::env::var_os("OPENBOX_APPIMAGE_TOOLS") {
        let licenses = directory.path().join("licenses");
        fs::create_dir(&licenses).unwrap();
        let output = Command::new("python3")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../scripts/package-native.py"
            ))
            .arg("--binary")
            .arg(&inner)
            .args([
                "--target",
                "x86_64-unknown-linux-gnu",
                "--version",
                "0.1.1",
                "--licenses",
            ])
            .arg(licenses)
            .arg("--output")
            .arg(directory.path())
            .arg("--appimage-tools")
            .arg(tools)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        directory
            .path()
            .join("openbox-x86_64-unknown-linux-gnu.AppImage")
    } else {
        let outer = directory.path().join("Openbox.AppImage");
        fs::write(&outer, b"\x7fELF\0\0\0\0AI\x02").unwrap();
        outer
    };
    let result = directory.path().join("resolved");
    let program = if std::env::var_os("OPENBOX_APPIMAGE_TOOLS").is_some() {
        &outer
    } else {
        &inner
    };
    let output = Command::new(program)
        .env("APPIMAGE_EXTRACT_AND_RUN", "1")
        .args(["--ignored", "--exact", "report_installation_path"])
        .env("APPDIR", &appdir)
        .env("APPIMAGE", &outer)
        .env("OPENBOX_TEST_RESOLVED_PATH", &result)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(fs::read_to_string(result).unwrap(), outer.to_str().unwrap());
}

#[test]
#[ignore]
fn report_installation_path() {
    let path = openbox_updater::installation_path().unwrap();
    fs::write(
        std::env::var_os("OPENBOX_TEST_RESOLVED_PATH").unwrap(),
        path.to_str().unwrap(),
    )
    .unwrap();
}

#[test]
fn restarting_an_update_clears_the_old_runtime_environment() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("updated-openbox");
    fs::write(&script, b"#!/bin/sh\nprintf '%s|%s|%s' \"$APPIMAGE\" \"$APPDIR\" \"$APPIMAGE_EXTRACT_AND_RUN\" > \"$1\"\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let marker = directory.path().join("restarted");
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "restart_probe"])
        .env("APPDIR", "/old/read-only-mount")
        .env("APPIMAGE", "/old/Openbox.AppImage")
        .env("OPENBOX_TEST_RESTART_EXECUTABLE", &script)
        .env("OPENBOX_TEST_RESTART_MARKER", &marker)
        .output()
        .unwrap();
    assert!(output.status.success());
    for _ in 0..100 {
        if marker.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(marker).unwrap(), "||1");
}

#[test]
#[ignore]
fn restart_probe() {
    let executable = std::env::var_os("OPENBOX_TEST_RESTART_EXECUTABLE").unwrap();
    let marker = std::env::var_os("OPENBOX_TEST_RESTART_MARKER").unwrap();
    openbox_updater::restart(std::path::Path::new(&executable), [marker]).unwrap();
}
