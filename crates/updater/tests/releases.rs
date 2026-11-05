use openbox_updater::newer_stable;

#[test]
fn only_strictly_newer_stable_versions_are_installable() {
    assert!(newer_stable("0.1.0", "v0.2.0").unwrap());
    assert!(!newer_stable("0.2.0", "v0.2.0").unwrap());
    assert!(!newer_stable("0.2.0", "v0.1.9").unwrap());
    assert!(!newer_stable("0.2.0", "v0.3.0-rc.1").unwrap());
    assert!(newer_stable("0.2.0", "latest").is_err());
}

#[test]
fn a_github_check_discovers_the_published_version_without_downloading() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        let n = stream.read(&mut request).unwrap();
        assert!(
            String::from_utf8_lossy(&request[..n])
                .starts_with("GET /repos/alexanderwanyoike/openbox/releases/latest ")
        );
        let asset = openbox_updater::Package::current().unwrap().asset;
        let body = serde_json::to_string(&serde_json::json!({"tag_name":"v0.2.0", "created_at":"2026-11-04T00:00:00Z", "name":"0.2.0", "assets":[{"name":asset,"url":format!("https://github.com/alexanderwanyoike/openbox/releases/download/v0.2.0/{asset}"),"digest":format!("sha256:{}", "a".repeat(64))}]})).unwrap();
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
    });
    let client = openbox_updater::Client::with_api_base(&base);
    let release = client.check("0.1.0").unwrap().unwrap();
    assert_eq!(release.version, "0.2.0");
    server.join().unwrap();
}

#[test]
fn verified_download_replaces_a_disposable_install_and_restart_preserves_arguments() {
    use std::{fs, process::Command};
    let temp = tempfile::tempdir().unwrap();
    let package = openbox_updater::Package::current().unwrap();
    let archive = temp.path().join(package.asset);
    let staged = temp.path().join("staged");
    fs::create_dir_all(staged.join(package.binary).parent().unwrap()).unwrap();
    let marker = temp.path().join("restarted");
    let executable = format!("openbox{}", std::env::consts::EXE_SUFFIX);
    let source = temp.path().join("fixture.rs");
    fs::write(&source, format!(
        "fn main() {{ if std::env::args().nth(1).as_deref() == Some(\"--version\") {{ println!(\"openbox 0.2.0\"); }} else {{ std::fs::write({marker:?}, std::env::args().skip(1).collect::<Vec<_>>().join(\"\\n\") + \"\\n\").unwrap(); }} }}"
    )).unwrap();
    assert!(
        Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(staged.join(package.binary))
            .status()
            .unwrap()
            .success()
    );
    if cfg!(target_os = "macos") {
        let resources = staged.join("Openbox.app/Contents/Resources");
        fs::create_dir_all(&resources).unwrap();
        fs::write(resources.join("updated.txt"), "new resource").unwrap();
    }
    #[cfg(not(windows))]
    assert!(
        Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .arg("-C")
            .arg(&staged)
            .arg(if cfg!(target_os = "macos") {
                "Openbox.app"
            } else {
                &executable
            })
            .status()
            .unwrap()
            .success()
    );
    #[cfg(windows)]
    {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        zip.start_file(&executable, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&fs::read(staged.join(package.binary)).unwrap())
            .unwrap();
        zip.finish().unwrap();
    }
    let bytes = fs::read(&archive).unwrap();
    use sha2::{Digest, Sha256};
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let (base, server) = release_server(bytes, digest.clone());
    let installed = if cfg!(target_os = "macos") {
        temp.path()
            .join("installed/Openbox.app/Contents/MacOS/openbox")
    } else {
        temp.path()
            .join(format!("installed-openbox{}", std::env::consts::EXE_SUFFIX))
    };
    fs::create_dir_all(installed.parent().unwrap()).unwrap();
    fs::write(&installed, "old application").unwrap();
    if cfg!(target_os = "macos") {
        fs::write(
            installed.ancestors().nth(3).unwrap().join("obsolete.txt"),
            "old resource",
        )
        .unwrap();
    }
    let client = openbox_updater::Client::with_api_base(&base);
    let release = client.check("0.1.0").unwrap().unwrap();
    client
        .install("0.1.0", &release, &installed, |_, _| {})
        .unwrap();
    assert!(
        Command::new(&installed)
            .arg("--version")
            .output()
            .unwrap()
            .stdout
            .starts_with(b"openbox 0.2.0")
    );
    if cfg!(target_os = "macos") {
        let bundle = installed.ancestors().nth(3).unwrap();
        assert!(!bundle.join("obsolete.txt").exists());
        assert_eq!(
            fs::read_to_string(bundle.join("Contents/Resources/updated.txt")).unwrap(),
            "new resource"
        );
    }
    openbox_updater::restart(&installed, ["--library", "a library.json"]).unwrap();
    for _ in 0..100 {
        if marker.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        fs::read_to_string(marker).unwrap(),
        "--library\na library.json\n"
    );
    server.join().unwrap();
}

fn release_server(bytes: Vec<u8>, digest: String) -> (String, std::thread::JoinHandle<()>) {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let root = base.clone();
    let thread = std::thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let n = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..n]);
            let body = if request.starts_with("GET /asset ") {
                bytes.clone()
            } else {
                serde_json::to_vec(&serde_json::json!({ "tag_name":"v0.2.0", "created_at":"2026-11-04T00:00:00Z", "assets":[{"name":openbox_updater::Package::current().unwrap().asset,"url":format!("{root}/asset"),"digest":format!("sha256:{digest}")}] })).unwrap()
            };
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    (base, thread)
}

#[test]
fn corrupt_downloads_leave_the_installed_application_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let installed = if cfg!(target_os = "macos") {
        temp.path().join("Openbox.app/Contents/MacOS/openbox")
    } else {
        temp.path().join("installed-openbox")
    };
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    std::fs::write(&installed, b"working application").unwrap();
    let (base, server) = release_server(b"damaged download".to_vec(), "a".repeat(64));
    let client = openbox_updater::Client::with_api_base(&base);
    let release = client.check("0.1.0").unwrap().unwrap();
    assert!(
        client
            .install("0.1.0", &release, &installed, |_, _| {})
            .is_err()
    );
    assert_eq!(std::fs::read(&installed).unwrap(), b"working application");
    server.join().unwrap();
}
