use openbox_audio::{BeatGrid, Clip, DeckControl, DeckId, Mixer};

fn deck(mixer: &mut Mixer, id: DeckId, period: f64, reported_bpm: f64) {
    mixer.load(id, Clip::from_pcm(vec![1000; 1000 * 2 * 60], 1000).unwrap());
    mixer.set_beat_grid(
        id,
        BeatGrid::new(reported_bpm, (0..100).map(|n| n as f64 * period).collect()).unwrap(),
    );
}

#[test]
fn sync_matches_the_playing_beat_period_instead_of_the_track_average() {
    let mut mixer = Mixer::default();
    deck(&mut mixer, DeckId::A, 0.5, 120.0);
    deck(&mut mixer, DeckId::B, 0.515, 120.0);
    mixer.control(DeckId::A, DeckControl::Seek(4.13));
    mixer.control(DeckId::B, DeckControl::Seek(8.81));
    mixer.control(DeckId::A, DeckControl::PlayPause);
    mixer.control(DeckId::B, DeckControl::PlayPause);
    mixer.sync_to(DeckId::B, DeckId::A).unwrap();
    let mut output = [0.0; 128];
    for _ in 0..312 {
        mixer.render(&mut output, 1000, 2);
        let [a, b] = mixer.snapshot().decks;
        let error = (a.position / 0.5 - b.position / 0.515 + 0.5).rem_euclid(1.0) - 0.5;
        assert!(
            error.abs() < 0.02,
            "Synced beats drifted by {error:.4} beats"
        );
    }
}

#[test]
fn syncing_a_stopped_deck_preserves_its_cue_until_playback_starts() {
    let mut mixer = Mixer::default();
    deck(&mut mixer, DeckId::A, 0.5, 120.0);
    deck(&mut mixer, DeckId::B, 0.48, 125.0);
    mixer.control(DeckId::A, DeckControl::Seek(4.13));
    mixer.control(DeckId::A, DeckControl::PlayPause);
    mixer.control(DeckId::B, DeckControl::Seek(8.81));
    mixer.control(DeckId::B, DeckControl::SetCue);
    let before = mixer.snapshot().decks[1];
    mixer.sync_to(DeckId::B, DeckId::A).unwrap();
    let after = mixer.snapshot().decks[1];
    assert_eq!(
        after.position, before.position,
        "Sync moved a stopped deck away from its cue"
    );
    assert_eq!(after.cue, before.cue);
    assert!(
        !after.sync_phase,
        "A stopped deck must report tempo sync while awaiting Play"
    );
    mixer.control(DeckId::B, DeckControl::PlayPause);
    let [a, b] = mixer.snapshot().decks;
    let error = (a.position / 0.5 - b.position / 0.48 + 0.5).rem_euclid(1.0) - 0.5;
    assert!(
        error.abs() < 0.001,
        "Starting the follower did not align the beats"
    );
}

#[test]
fn sync_keeps_the_last_known_tempo_after_the_detected_grid_ends() {
    let mut mixer = Mixer::default();
    deck(&mut mixer, DeckId::A, 0.5, 120.0);
    deck(&mut mixer, DeckId::B, 0.515, 120.0);
    mixer.set_beat_grid(
        DeckId::B,
        BeatGrid::new(120.0, (0..8).map(|n| n as f64 * 0.515).collect()).unwrap(),
    );
    mixer.control(DeckId::A, DeckControl::Seek(10.13));
    mixer.control(DeckId::B, DeckControl::Seek(11.81));
    mixer.control(DeckId::A, DeckControl::PlayPause);
    mixer.control(DeckId::B, DeckControl::PlayPause);
    mixer.sync_to(DeckId::B, DeckId::A).unwrap();
    let [a, b] = mixer.snapshot().decks;
    let error = (a.position / 0.5 - b.position / 0.515 + 0.5).rem_euclid(1.0) - 0.5;
    assert!(
        error.abs() < 0.001,
        "Extrapolated sync uses a different tempo from the beat map: {error}"
    );
}

#[test]
fn unstable_intro_estimates_do_not_reject_or_drop_a_compatible_sync_pair() {
    let mut mixer = Mixer::default();
    deck(&mut mixer, DeckId::A, 0.5, 120.0);
    deck(&mut mixer, DeckId::B, 0.5, 120.0);
    let mut beats: Vec<_> = (0..6).map(|n| n as f64 * 0.25).collect();
    beats.extend((3..100).map(|n| n as f64 * 0.5));
    mixer.set_beat_grid(DeckId::B, BeatGrid::new(120.0, beats).unwrap());
    mixer.control(DeckId::A, DeckControl::PlayPause);
    mixer.control(DeckId::B, DeckControl::PlayPause);
    mixer
        .sync_to(DeckId::B, DeckId::A)
        .expect("Compatible track tempos were rejected by intro estimates");
    assert!(
        !mixer.snapshot().decks[1].sync_phase,
        "Unusable intro data must not report beat sync"
    );
    for _ in 0..400 {
        mixer.render(&mut [0.0; 100], 1000, 2);
        assert_eq!(mixer.snapshot().decks[1].sync_leader, Some(DeckId::A));
    }
    let [a, b] = mixer.snapshot().decks;
    assert!(b.sync_phase, "Beat sync did not resume after the intro");
    let error = ((a.position - b.position) / 0.5 + 0.5).rem_euclid(1.0) - 0.5;
    assert!(
        error.abs() < 0.05,
        "Beat sync did not recover after the intro: {error}"
    );
}

#[test]
fn cue_and_set_cue_keep_a_synced_follower_armed_for_the_next_start() {
    for control in [DeckControl::Cue, DeckControl::SetCue] {
        let mut mixer = Mixer::default();
        deck(&mut mixer, DeckId::A, 0.5, 120.0);
        deck(&mut mixer, DeckId::B, 0.48, 125.0);
        mixer.control(DeckId::A, DeckControl::Seek(4.13));
        mixer.control(DeckId::B, DeckControl::Seek(8.81));
        mixer.control(DeckId::A, DeckControl::PlayPause);
        mixer.control(DeckId::B, DeckControl::PlayPause);
        mixer.sync_to(DeckId::B, DeckId::A).unwrap();
        mixer.control(DeckId::B, control);
        let stopped = mixer.snapshot().decks[1];
        assert_eq!(stopped.sync_leader, Some(DeckId::A), "Cue disarmed sync");
        assert!(!stopped.playing && !stopped.sync_phase);
        mixer.control(DeckId::B, DeckControl::PlayPause);
        let [a, b] = mixer.snapshot().decks;
        let error = (a.position / 0.5 - b.position / 0.48 + 0.5).rem_euclid(1.0) - 0.5;
        assert!(
            error.abs() < 0.001,
            "Cue restart did not realign the follower"
        );
    }
}
