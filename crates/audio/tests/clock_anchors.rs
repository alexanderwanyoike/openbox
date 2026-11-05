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

#[test]
fn starting_a_synced_follower_does_not_inherit_the_stopped_masters_clock() {
    let mut mixer = pair();
    mixer.control(DeckId::A, DeckControl::PlayPause);
    mixer.sync_to(DeckId::B, DeckId::A).unwrap();
    mixer.control(DeckId::A, DeckControl::PlayPause);
    let pressed_at = std::time::Instant::now();
    mixer.control(DeckId::B, DeckControl::PlayPause);
    let follower = mixer.snapshot().decks[1];
    assert!(follower.playing);
    assert!(
        follower.sampled_at >= pressed_at,
        "Play inherited time accumulated while the master was stopped"
    );
}

#[test]
fn phase_alignment_transfers_the_clock_of_the_playing_master() {
    let mut mixer = pair();
    mixer.control(DeckId::A, DeckControl::Seek(2.1));
    mixer.control(DeckId::B, DeckControl::Seek(4.31));
    mixer.control(DeckId::A, DeckControl::PlayPause);
    mixer.control(DeckId::B, DeckControl::PlayPause);
    let before = mixer.snapshot();
    mixer.sync_to(DeckId::B, DeckId::A).unwrap();
    let after = mixer.snapshot().decks[1];
    assert!((after.position - 4.1).abs() < 0.0001);
    assert_ne!(after.position, before.decks[1].position);
    assert_eq!(after.sampled_at, before.decks[0].sampled_at);
}
