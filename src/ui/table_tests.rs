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
