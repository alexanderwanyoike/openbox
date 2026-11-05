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
