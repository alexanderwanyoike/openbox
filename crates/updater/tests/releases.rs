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
    use std::{io::{Read, Write}, net::TcpListener};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        let n = stream.read(&mut request).unwrap();
        assert!(String::from_utf8_lossy(&request[..n]).starts_with("GET /repos/alexanderwanyoike/openbox/releases/latest "));
        let body = r#"{"tag_name":"v0.2.0","created_at":"2026-11-04T00:00:00Z","name":"0.2.0","assets":[{"name":"openbox-x86_64-unknown-linux-gnu.tar.gz","url":"https://github.com/alexanderwanyoike/openbox/releases/download/v0.2.0/openbox-x86_64-unknown-linux-gnu.tar.gz","digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}]}"#;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
    });
    let client = openbox_updater::Client::with_api_base(&base);
    let release = client.check("0.1.0").unwrap().unwrap();
    assert_eq!(release.version, "0.2.0");
    server.join().unwrap();
}
