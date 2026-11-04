use super::*;

#[test]
fn long_metadata_does_not_paint_over_bpm_key_or_duration() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("library.json");
    let render = |genre: &str| {
        std::fs::write(&path, serde_json::json!({"tracks":[{"id":1,"path":"track.wav","title":"Track", "artist":"Artist", "details":{"genre":genre,"bpm":124.6,"key":"Fm","duration_seconds":792}}],"playlists":[]}).to_string()).unwrap();
        let app = App::load(path.clone(), None).unwrap();
        super::test_render::render(&app, iced::mouse::Cursor::Unavailable)
    };
    let empty = render("");
    let long = render("Electronic / Techno / Dub Techno / Ambient / Deep House / Disco / House");
    for y in 220..255 {
        for x in 1300..1425 {
            assert_eq!(empty.pixel(x,y), long.pixel(x,y), "Metadata escaped its column at {x},{y}");
        }
    }
}

#[test]
fn every_waveform_palette_follows_the_application_mode() {
    use super::appearance::{Mode, WaveformTheme};
    let temp = tempfile::tempdir().unwrap();
    let mut app = App::load(temp.path().join("library.json"), None).unwrap();
    app.preview.visible = true;
    for mode in Mode::ALL {
        for waveform_theme in WaveformTheme::ALL {
            app.preferences.mode = mode;
            app.preferences.waveform_theme = waveform_theme;
            let pixels = super::test_render::render(&app, iced::mouse::Cursor::Unavailable);
            let pixel = pixels.pixel(400, 260).unwrap();
            let brightness = (f32::from(pixel.red()) + f32::from(pixel.green()) + f32::from(pixel.blue())) / (3.0 * 255.0);
            assert!(match mode { Mode::Light => brightness > 0.8, Mode::Dark => brightness < 0.15 }, "{mode:?}/{waveform_theme:?} has the wrong waveform surface: {pixel:?}");
        }
    }
}

#[test]
fn decoder_diagnostics_do_not_replace_the_default_error_notice() {
    let temp = tempfile::tempdir().unwrap();
    let render = |diagnostics: &str| {
        let mut app = App::load(temp.path().join("library.json"), None).unwrap();
        app.preview.visible = true;
        let id = app.workspace.library.import("missing.mp3".into(), "Missing track".into(), String::new());
        let track = app.workspace.library.track(id).unwrap().clone();
        let _ = app.preview.load(openbox_audio::DeckId::A, track);
        let generation = app.preview.decks[0].generation;
        let _ = app.preview.update(super::preview::Event::Loaded(openbox_audio::DeckId::A, generation, Err(diagnostics.into())));
        super::test_render::render(&app, iced::mouse::Cursor::Unavailable)
    };
    let a = render("Cannot preview this file: [in#0 @ 0x12345] Error opening input");
    let b = render("Cannot preview this file: [in#0 @ 0x98765] Different decoder output");
    assert!(a.data() == b.data(), "Raw decoder output leaked into the default error presentation");
}
