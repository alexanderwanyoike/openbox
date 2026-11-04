use openbox_domain::Beat;

#[derive(Debug)]
pub(super) struct Tick {
    pub x: f32,
    pub major: bool,
    pub label: Option<String>,
}

pub(super) fn beats(beats: &[Beat], start: f64, span: f64, duration: f64, width: f32) -> Vec<Tick> {
    let mut ticks = Vec::new();
    let first = beats
        .partition_point(|b| b.time_seconds < start)
        .saturating_sub(1);
    let mut last_label = -100.0;
    for (index, beat) in beats
        .iter()
        .enumerate()
        .skip(first)
        .take_while(|(_, b)| b.time_seconds <= start + span)
    {
        let interval = beats
            .get(index + 1)
            .map_or(60.0 / beat.bpm.max(1.0), |next| {
                next.time_seconds - beat.time_seconds
            });
        let beat_width = interval / span * f64::from(width);
        let subdivisions = if beat_width >= 40.0 {
            4
        } else if beat_width >= 20.0 {
            2
        } else {
            1
        };
        for part in 0..subdivisions {
            let at = beat.time_seconds + interval * f64::from(part) / f64::from(subdivisions);
            if at < start || at > (start + span).min(duration) || at < 0.0 {
                continue;
            }
            let x = ((at - start) / span) as f32 * width;
            let major = part == 0 && beat.number == 1;
            let label = if part == 0
                && (major || beat_width >= 64.0)
                && x - last_label >= 64.0
                && x + 42.0 <= width
            {
                last_label = x;
                Some(if beat_width >= 64.0 {
                    format!("{}.{}", index / 4 + 1, beat.number)
                } else {
                    format!("{}", index / 4 + 1)
                })
            } else {
                None
            };
            ticks.push(Tick { x, major, label });
        }
    }
    ticks
}

pub(super) fn time(start: f64, span: f64, duration: f64, width: f32) -> Vec<Tick> {
    let step = [
        0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 30.0, 60.0, 120.0, 300.0,
    ]
    .into_iter()
    .find(|step| step / span * f64::from(width) >= 70.0)
    .unwrap_or(600.0);
    let minor = step / 5.0;
    let first = (start.max(0.0) / minor).ceil() as u64;
    let last = ((start + span).min(duration).max(0.0) / minor).floor() as u64;
    (first..=last)
        .map(|n| {
            let at = n as f64 * minor;
            let major = n % 5 == 0;
            let label =
                (major && ((at - start) / span) as f32 * width + 48.0 <= width).then(|| {
                    let tenths = (at * 10.0).round() as u64;
                    if step < 1.0 {
                        format!("{}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10)
                    } else {
                        format!("{}:{:02}", tenths / 600, tenths / 10 % 60)
                    }
                });
            Tick {
                x: ((at - start) / span) as f32 * width,
                major,
                label,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoomed_rulers_reveal_subdivisions_and_zoomed_out_labels_do_not_overlap() {
        let grid: Vec<_> = (0..200)
            .map(|n| Beat {
                time_seconds: n as f64 * 0.5,
                bpm: 120.0,
                number: n % 4 + 1,
            })
            .collect();
        let close = beats(&grid, 9.75, 1.0, 100.0, 600.0);
        assert!(close.len() > 3);
        assert!(close.iter().any(|t| t.label.as_deref() == Some("6.1")));
        let wide = beats(&grid, 0.0, 64.0, 100.0, 600.0);
        assert!(wide.iter().any(|t| t.label.is_some()));
        let labels: Vec<_> = wide.iter().filter(|t| t.label.is_some()).collect();
        assert!(labels.windows(2).all(|p| p[1].x - p[0].x >= 60.0));
        let clock = time(-0.5, 1.0, 100.0, 600.0);
        assert!(clock.iter().any(|t| t.label.as_deref() == Some("0:00.0")));
        assert!(clock.iter().any(|t| t.label.as_deref() == Some("0:00.2")));
        assert!(clock.iter().all(|t| t.x >= 300.0 && t.x <= 600.0));
        assert!(time(99.0, 4.0, 100.0, 600.0).iter().all(|t| t.x <= 150.0));
    }
}
