use openbox_media_tools::Tools;

#[test]
fn launches_media_tools_from_ordered_installation_directories() {
    let root = tempfile::tempdir().unwrap();
    let preferred = root.path().join("preferred");
    let fallback = root.path().join("fallback");
    std::fs::create_dir(&preferred).unwrap();
    std::fs::create_dir(&fallback).unwrap();
    for (directory, name) in [
        (&preferred, "ffmpeg"),
        (&fallback, "ffmpeg"),
        (&fallback, "ffprobe"),
    ] {
        std::fs::copy(
            std::env::current_exe().unwrap(),
            directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
        )
        .unwrap();
    }
    let tools = Tools::in_directories([preferred.clone(), fallback.clone()]);
    assert_eq!(
        tools.ffmpeg().get_program(),
        preferred.join(format!("ffmpeg{}", std::env::consts::EXE_SUFFIX))
    );
    assert_eq!(
        tools.ffprobe().get_program(),
        fallback.join(format!("ffprobe{}", std::env::consts::EXE_SUFFIX))
    );
    for mut command in [tools.ffmpeg(), tools.ffprobe()] {
        let output = command.arg("--list").output().unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("launches_media_tools"));
    }
}
