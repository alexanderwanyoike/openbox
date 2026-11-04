use openbox_updater::newer_stable;

#[test]
fn only_strictly_newer_stable_versions_are_installable() {
    assert!(newer_stable("0.1.0", "v0.2.0").unwrap());
    assert!(!newer_stable("0.2.0", "v0.2.0").unwrap());
    assert!(!newer_stable("0.2.0", "v0.1.9").unwrap());
    assert!(!newer_stable("0.2.0", "v0.3.0-rc.1").unwrap());
    assert!(newer_stable("0.2.0", "latest").is_err());
}
