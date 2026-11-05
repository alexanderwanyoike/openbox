// SPDX-License-Identifier: GPL-2.0-or-later
// Adapted from Mixxx BeatUtils, copyright 2001-2026 Mixxx Development Team.
// See LICENSE.mixxx for the upstream revision and terms.

mod regions;

use openbox_domain::Beat;

pub(super) fn stabilize(beats: &[Beat], duration: f64) -> Option<(f64, Vec<Beat>)> {
    if beats.len() < 16 {
        return None;
    }
    let times: Vec<_> = beats.iter().map(|beat| beat.time_seconds).collect();
    let (bpm, first) = regions::constant(&regions::retrieve(&times))?;
    let interval = 60.0 / bpm;
    let mut adjustment = 0.0;
    let mut count = 0;
    for time in &times {
        let mut offset = (time - first).rem_euclid(interval);
        if offset > interval / 2.0 {
            offset -= interval;
        }
        if offset.abs() < regions::PHASE_ERROR {
            adjustment += offset;
            count += 1;
        }
    }
    let first = (first + adjustment / f64::from(count.max(1))).rem_euclid(interval);
    let beats = (0..)
        .map(|i| Beat {
            time_seconds: first + f64::from(i) * interval,
            bpm,
            number: (i % 4 + 1) as u16,
        })
        .take_while(|beat| beat.time_seconds < duration)
        .collect();
    Some((bpm, beats))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detections(bpm: f64, count: usize, phase: f64) -> Vec<Beat> {
        (0..count)
            .map(|i| Beat {
                time_seconds: phase + i as f64 * 60.0 / bpm + (i % 3) as f64 * 0.002 - 0.002,
                bpm: bpm + (i % 3) as f64 - 1.0,
                number: (i % 4 + 1) as u16,
            })
            .collect()
    }

    #[test]
    fn a_long_grid_retains_phase_through_a_break_and_a_single_detector_outlier() {
        let mut coarse = detections(124.0, 1240, 2.17);
        coarse[320].time_seconds += 0.06;
        coarse.drain(400..520);
        let duration = 604.0;
        let (bpm, grid) = stabilize(&coarse, duration).unwrap();
        assert!((bpm - 124.0).abs() < 0.001, "{bpm}");
        assert!(grid.len() > coarse.len() + 100);
        for i in [0, 400, 800, 1200] {
            let expected = (2.17_f64 % (60.0 / 124.0)) + i as f64 * 60.0 / 124.0;
            assert!((grid[i].time_seconds - expected).abs() < 0.003);
        }
        assert!(grid.last().unwrap().time_seconds < duration);
        assert!(grid.last().unwrap().time_seconds + 60.0 / bpm >= duration);
    }

    #[test]
    fn fractional_tempos_are_not_forced_to_the_nearest_integer() {
        let coarse = detections(123.45, 1200, 0.17);
        let (bpm, grid) = stabilize(&coarse, 580.0).unwrap();
        assert!((bpm - 123.45).abs() < 0.005, "{bpm}");
        assert!((grid[1000].time_seconds - (0.17 + 1000.0 * 60.0 / 123.45)).abs() < 0.003);
    }

    #[test]
    fn sparse_or_unstable_detections_do_not_invent_a_constant_tempo() {
        assert!(stabilize(&[], 10.0).is_none());
        assert!(stabilize(&detections(120.0, 8, 0.1), 10.0).is_none());
        let mut unstable = detections(120.0, 100, 0.1);
        for (i, beat) in unstable.iter_mut().enumerate() {
            beat.time_seconds += (i as f64 / 5.0).sin() * 0.2;
        }
        assert!(stabilize(&unstable, 52.0).is_none());
    }
}
