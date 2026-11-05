use openbox_audio::{BeatGrid, Clip, DeckControl, DeckId, Mixer};

fn pair() -> Mixer {
    let mut mixer = Mixer::default();
    for id in [DeckId::A, DeckId::B] {
        mixer.load(id, Clip::from_pcm(vec![0; 20_000], 1000).unwrap());
        mixer.set_beat_grid(
            id,
            BeatGrid::new(120.0, (0..20).map(|n| n as f64 * 0.5).collect()).unwrap(),
        );
    }
    mixer
}

#[test]
fn tempo_only_sync_preserves_the_playing_decks_audio_clock() {
    let mut mixer = pair();
    mixer.control(DeckId::B, DeckControl::PlayPause);
    let before = mixer.snapshot().decks[1];
    mixer.sync_to(DeckId::B, DeckId::A).unwrap();
    let after = mixer.snapshot().decks[1];
    assert_eq!(after.position, before.position);
    assert_eq!(
        after.sampled_at, before.sampled_at,
        "Tempo-only sync borrowed a paused masters stale clock"
    );
}
