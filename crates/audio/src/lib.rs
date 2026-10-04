mod clip;
mod engine;
mod mixer;

pub use clip::{Clip, WavePeak, Waveform};
pub use engine::Engine;
pub use mixer::{Band, DeckControl, DeckId, DeckSnapshot, Mixer, Snapshot};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_faders_scale_trim_independently_without_stopping_or_muting_the_other_deck() {
        let mut mixer = Mixer::default();
        for (id, sample) in [(DeckId::A, 8192), (DeckId::B, -8192)] {
            mixer.load(id, Clip::from_pcm(vec![sample; 200], 10).unwrap());
            mixer.control(id, DeckControl::Gain(1.0));
            mixer.control(id, DeckControl::PlayPause);
        }
        mixer.set_master(1.0);
        mixer.set_crossfade(0.0);
        let mut output = [0.0; 4];
        mixer.control(DeckId::A, DeckControl::Fader(0.5));
        mixer.render(&mut output, 10, 2);
        assert!(output.iter().all(|v| (*v - 0.125).abs() < 0.0001));
        mixer.control(DeckId::A, DeckControl::Gain(0.5));
        mixer.render(&mut output, 10, 2);
        assert!(output.iter().all(|v| (*v - 0.0625).abs() < 0.0001));
        mixer.control(DeckId::A, DeckControl::Fader(0.0));
        mixer.render(&mut output, 10, 2);
        assert!(output.iter().all(|v| v.abs() < 0.0001));
        assert!((mixer.snapshot().decks[0].position - 0.6).abs() < 0.0001);
        assert!(mixer.snapshot().decks[0].peak > 0.1);
        mixer.set_crossfade(1.0);
        mixer.render(&mut output, 10, 2);
        assert!(output.iter().all(|v| (*v + 0.25).abs() < 0.0001));
        mixer.set_crossfade(0.0);
        mixer.control(DeckId::A, DeckControl::Fader(2.0));
        mixer.control(DeckId::A, DeckControl::Fader(f32::NAN));
        mixer.render(&mut output, 10, 2);
        assert!(output.iter().all(|v| (*v - 0.125).abs() < 0.0001));
    }

    #[test]
    fn a_finished_deck_stops_exactly_at_the_end_and_cue_returns_to_the_start() {
        let mut mixer = Mixer::default();
        mixer.load(DeckId::A, Clip::from_pcm(vec![8192; 4], 10).unwrap());
        mixer.control(DeckId::A, DeckControl::Seek(0.1));
        mixer.control(DeckId::A, DeckControl::Rate(1.08));
        mixer.control(DeckId::A, DeckControl::PlayPause);
        mixer.render(&mut [0.0; 2], 10, 2);
        assert_eq!(mixer.snapshot().decks[0].position, 0.2);
        assert!(!mixer.snapshot().decks[0].playing);
        mixer.control(DeckId::A, DeckControl::Cue);
        assert_eq!(mixer.snapshot().decks[0].position, 0.0);
    }

    #[test]
    fn cue_eq_and_output_limits_apply_to_preview_audio_without_changing_the_clip() {
        let clip = Clip::from_pcm(vec![30000; 96000], 48000).unwrap();
        let mut mixer = Mixer::default();
        mixer.load(DeckId::A, clip.clone());
        mixer.set_crossfade(0.0);
        mixer.set_master(1.0);
        mixer.control(DeckId::A, DeckControl::Gain(1.0));
        mixer.control(DeckId::A, DeckControl::Seek(0.25));
        mixer.control(DeckId::A, DeckControl::SetCue);
        mixer.control(DeckId::A, DeckControl::Seek(0.5));
        mixer.control(DeckId::A, DeckControl::PlayPause);
        mixer.control(DeckId::A, DeckControl::Eq(Band::Low, 6.0));
        let mut output = vec![0.0; 8192];
        mixer.render(&mut output, 48000, 2);
        assert!(mixer.snapshot().clipped);
        assert!(output.iter().all(|v| v.abs() <= 1.0));
        mixer.control(DeckId::A, DeckControl::Eq(Band::Low, -24.0));
        mixer.render(&mut output, 48000, 2);
        assert!(output[output.len() - 1].abs() < 0.06);
        mixer.control(DeckId::A, DeckControl::Cue);
        assert_eq!(mixer.snapshot().decks[0].position, 0.25);
        assert!(!mixer.snapshot().decks[0].playing);
        assert_eq!(clip.duration(), 1.0);
    }

    #[test]
    fn decoding_real_audio_builds_a_seekable_waveform_and_rejects_bad_files() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("track.wav");
        openbox_test_support::audio(&path);
        let clip = Clip::decode(&path).unwrap();
        assert!((clip.duration() - 2.0).abs() < 0.01);
        assert!(!clip.waveform.peaks.is_empty());
        assert!(clip.waveform.peaks.iter().any(|p| p.peak > 0.01));
        assert!((clip.waveform.duration - clip.duration()).abs() < 0.001);
        std::fs::write(&path, b"not audio").unwrap();
        assert!(Clip::decode(&path).is_err());
    }

    #[test]
    fn two_decks_mix_seek_pause_and_crossfade_through_the_same_render_path() {
        let a = Clip::from_pcm(vec![8192; 40], 10).unwrap();
        let b = Clip::from_pcm(vec![-8192; 40], 10).unwrap();
        let mut mixer = Mixer::default();
        mixer.load(DeckId::A, a);
        mixer.load(DeckId::B, b);
        mixer.control(DeckId::A, DeckControl::Gain(1.0));
        mixer.control(DeckId::B, DeckControl::Gain(1.0));
        mixer.set_master(1.0);
        mixer.control(DeckId::A, DeckControl::PlayPause);
        mixer.control(DeckId::B, DeckControl::PlayPause);
        mixer.set_crossfade(0.0);
        let mut output = [0.0; 4];
        mixer.render(&mut output, 10, 2);
        assert!(output.iter().all(|v| (*v - 0.25).abs() < 0.0001));
        assert!((mixer.snapshot().decks[0].position - 0.2).abs() < 0.0001);
        mixer.set_crossfade(1.0);
        mixer.render(&mut output, 10, 2);
        assert!(output.iter().all(|v| (*v + 0.25).abs() < 0.0001));
        mixer.control(DeckId::A, DeckControl::Seek(1.0));
        mixer.control(DeckId::A, DeckControl::PlayPause);
        mixer.set_crossfade(0.0);
        mixer.render(&mut output, 10, 2);
        assert!(output.iter().all(|v| v.abs() < 0.0001));
        assert_eq!(mixer.snapshot().decks[0].position, 1.0);
        mixer.control(DeckId::A, DeckControl::PlayPause);
        mixer.control(DeckId::A, DeckControl::Rate(1.08));
        mixer.render(&mut output, 10, 2);
        assert!((mixer.snapshot().decks[0].position - 1.216).abs() < 0.0001);
    }
}
