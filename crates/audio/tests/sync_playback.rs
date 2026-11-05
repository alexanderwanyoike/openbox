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
        assert!(error.abs() < 0.02, "Synced beats drifted by {error:.4} beats");
    }
}
