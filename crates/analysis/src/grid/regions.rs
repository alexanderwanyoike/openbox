// SPDX-License-Identifier: GPL-2.0-or-later
// Adapted from Mixxx BeatUtils, copyright 2001-2026 Mixxx Development Team.
// See LICENSE.mixxx for the upstream revision and terms.

pub(super) const PHASE_ERROR: f64 = 0.025;
pub(super) const MIN_BEATS: f64 = 16.0;

#[derive(Clone, Copy)]
pub(super) struct Region {
    pub first: f64,
    pub interval: f64,
}

pub(super) fn retrieve(beats: &[f64]) -> Vec<Region> {
    let mut regions = Vec::new();
    let mut left = 0;
    let mut right = beats.len() - 1;
    while left < beats.len() - 1 {
        let interval = (beats[right] - beats[left]) / (right - left) as f64;
        let mut outliers = 0;
        let mut error_sum = 0.0;
        let stable = (left + 1..=right).all(|i| {
            let error = beats[left] + (i - left) as f64 * interval - beats[i];
            error_sum += error;
            if error.abs() > PHASE_ERROR {
                outliers += 1;
                if outliers > 1 || i == left + 1 {
                    return false;
                }
            }
            error_sum.abs() <= 0.1
        });
        let border_error = if right > left + 2 {
            (beats[left + 1] - beats[left] + beats[right] - beats[right - 1] - 2.0 * interval).abs()
        } else {
            0.0
        };
        if stable && border_error < PHASE_ERROR / 2.0 {
            regions.push(Region {
                first: beats[left],
                interval,
            });
            left = right;
            right = beats.len() - 1;
        } else {
            right -= 1;
        }
    }
    regions.push(Region {
        first: beats[beats.len() - 1],
        interval: 0.0,
    });
    regions
}

pub(super) fn constant(regions: &[Region]) -> Option<(f64, f64)> {
    let (middle, longest) = regions
        .windows(2)
        .enumerate()
        .max_by(|(_, a), (_, b)| (a[1].first - a[0].first).total_cmp(&(b[1].first - b[0].first)))?;
    let mut interval = longest[0].interval;
    let mut count = ((longest[1].first - longest[0].first) / interval).round();
    if count < MIN_BEATS {
        return None;
    }
    let mut start = middle;
    for i in 0..middle {
        if let Some((combined, beats)) = combine(
            interval,
            count,
            regions[i],
            regions[i + 1].first - regions[i].first,
            regions[middle + 1].first - regions[i].first,
        ) {
            interval = combined;
            count = beats;
            start = i;
            break;
        }
    }
    for i in (middle + 1..regions.len() - 1).rev() {
        if let Some((combined, beats)) = combine(
            interval,
            count,
            regions[i],
            regions[i + 1].first - regions[i].first,
            regions[i + 1].first - regions[start].first,
        ) {
            interval = combined;
            count = beats;
            break;
        }
    }
    let minimum = 60.0 / (interval + PHASE_ERROR / count);
    let maximum = 60.0 / (interval - PHASE_ERROR / count);
    let center = 60.0 / interval;
    let bpm = round_bpm(minimum, center, maximum);
    Some((bpm, regions[start].first.rem_euclid(60.0 / bpm)))
}

fn combine(
    interval: f64,
    count: f64,
    region: Region,
    region_length: f64,
    combined_length: f64,
) -> Option<(f64, f64)> {
    let region_count = (region_length / region.interval).round();
    if region_count < MIN_BEATS {
        return None;
    }
    let lower = region.interval - PHASE_ERROR / region_count;
    let upper = region.interval + PHASE_ERROR / region_count;
    if interval <= lower || interval >= upper {
        return None;
    }
    let allowed_lower = interval - PHASE_ERROR / count;
    let allowed_upper = interval + PHASE_ERROR / count;
    let most_beats = (combined_length / allowed_lower.max(lower)).round();
    let fewest_beats = (combined_length / allowed_upper.min(upper)).round();
    if most_beats != fewest_beats || fewest_beats <= 0.0 {
        return None;
    }
    let combined = combined_length / fewest_beats;
    (combined > allowed_lower && combined < allowed_upper).then_some((combined, fewest_beats))
}

fn round_bpm(minimum: f64, center: f64, maximum: f64) -> f64 {
    for fraction in [
        Some(1.0),
        (center < 85.0).then_some(2.0),
        (center > 127.0).then_some(2.0 / 3.0),
        Some(3.0),
        Some(12.0),
    ]
    .into_iter()
    .flatten()
    {
        let snapped = (center * fraction).round() / fraction;
        if snapped > minimum && snapped < maximum {
            return snapped;
        }
    }
    center
}
