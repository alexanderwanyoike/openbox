use std::{io::Write, path::Path};

pub fn rekordbox_usb(root: &Path) {
    use baken_export::pdb::{self, Export, ExportPlaylist, rows::TrackRow};
    let audio_path = root.join("Contents/Original/track.wav");
    std::fs::create_dir_all(audio_path.parent().unwrap()).unwrap();
    audio(&audio_path);
    let analysis = root.join("PIONEER/USBANLZ/Original/ANLZ0000.DAT");
    std::fs::create_dir_all(analysis.parent().unwrap()).unwrap();
    std::fs::write(&analysis, b"original cues and beatgrid").unwrap();
    let database = root.join("PIONEER/rekordbox/export.pdb");
    std::fs::create_dir_all(database.parent().unwrap()).unwrap();
    std::fs::write(
        database,
        pdb::write(&Export {
            tracks: vec![TrackRow {
                id: 42,
                title: "Original track".into(),
                artist_id: 17,
                file_path: "/Contents/Original/track.wav".into(),
                analyze_path: "/PIONEER/USBANLZ/Original/ANLZ0000.DAT".into(),
                rating: 5,
                play_count: 91,
                comment: "Keep this comment".into(),
                tempo: 12345,
                duration_seconds: 2,
                file_type: pdb::rows::FILE_TYPE_WAV,
                ..Default::default()
            }],
            artists: vec![(17, "Original artist".into())],
            playlists: vec![
                ExportPlaylist {
                    id: 80,
                    parent: 0,
                    sort_order: 0,
                    is_folder: true,
                    name: "Sets".into(),
                    track_ids: vec![],
                },
                ExportPlaylist {
                    id: 23,
                    parent: 80,
                    sort_order: 0,
                    is_folder: false,
                    name: "Original set".into(),
                    track_ids: vec![42],
                },
            ],
            export_date: "2026-01-01".into(),
            device_name: "Original USB".into(),
            ..Default::default()
        }),
    )
    .unwrap();
    std::fs::write(
        root.join("PIONEER/rekordbox/exportExt.pdb"),
        b"original extended metadata",
    )
    .unwrap();
}

pub fn files(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn visit(
        root: &Path,
        path: &Path,
        result: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().into(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = std::collections::BTreeMap::new();
    visit(root, root, &mut result);
    result
}

pub fn audio(path: &Path) {
    let samples = 44_100_u32 * 2;
    let data_len = samples * 4;
    let mut file = std::fs::File::create(path).unwrap();
    for bytes in [
        b"RIFF".to_vec(),
        (36 + data_len).to_le_bytes().to_vec(),
        b"WAVEfmt ".to_vec(),
        16_u32.to_le_bytes().to_vec(),
        1_u16.to_le_bytes().to_vec(),
        2_u16.to_le_bytes().to_vec(),
        44_100_u32.to_le_bytes().to_vec(),
        176_400_u32.to_le_bytes().to_vec(),
        4_u16.to_le_bytes().to_vec(),
        16_u16.to_le_bytes().to_vec(),
        b"data".to_vec(),
        data_len.to_le_bytes().to_vec(),
    ] {
        file.write_all(&bytes).unwrap();
    }
    for i in 0..samples {
        let sample = ((i as f64 * 440.0 * std::f64::consts::TAU / 44_100.0).sin() * 4000.0) as i16;
        file.write_all(&sample.to_le_bytes()).unwrap();
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
}

pub fn rhythm(path: &Path) {
    let status = std::process::Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-f", "lavfi", "-i",
            "aevalsrc=0.25*sin(2*PI*130.8128*t)+0.15*sin(2*PI*164.8138*t)+0.15*sin(2*PI*195.9977*t)+0.12*sin(2*PI*261.6256*t)+0.7*exp(-60*mod(t\\,0.5))*sin(2*PI*70*t):s=22050:d=32",
            "-c:a", "pcm_s16le"])
        .arg(path).status().unwrap();
    assert!(status.success());
}

pub fn mp3(path: &Path) {
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=2",
            "-c:a",
            "libmp3lame",
        ])
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
}
