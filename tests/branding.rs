#[cfg(feature = "desktop")]
#[test]
fn application_icon_decodes_for_the_native_window() {
    assert!(openbox::branding::window_icon().is_ok());
}
