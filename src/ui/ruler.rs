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
