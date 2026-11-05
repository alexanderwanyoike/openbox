use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub struct WavePeak {
    pub peak: f32,
    pub rms: f32,
    pub bands: [f32; 3],
}

impl WavePeak {
    fn merge(a: Self, b: Self) -> Self {
        Self {
            peak: a.peak.max(b.peak),
            rms: ((a.rms * a.rms + b.rms * b.rms) * 0.5).sqrt(),
            bands: std::array::from_fn(|i| {
                ((a.bands[i].powi(2) + b.bands[i].powi(2)) * 0.5).sqrt()
            }),
        }
    }

    fn interpolate(a: Self, b: Self, blend: f32) -> Self {
        Self {
            peak: a.peak + (b.peak - a.peak) * blend,
            rms: a.rms + (b.rms - a.rms) * blend,
            bands: std::array::from_fn(|i| a.bands[i] + (b.bands[i] - a.bands[i]) * blend),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Waveform {
    pub peaks: Arc<[WavePeak]>,
    pub duration: f64,
    pub bins_per_second: f64,
    levels: Arc<[Arc<[WavePeak]>]>,
}

impl Waveform {
    pub(crate) fn from_pcm(samples: &[i16], sample_rate: u32) -> Self {
        let hop = (sample_rate / 500).max(1) as usize;
        let coefficients = [200.0, 2500.0]
            .map(|hz| 1.0 - (-std::f32::consts::TAU * hz / sample_rate as f32).exp());
        let mut filters = [[0.0; 2]; 2];
        let peaks: Arc<[WavePeak]> = samples
            .chunks(hop * 2)
            .map(|chunk| {
                let mut peak = 0.0_f32;
                let mut energy = [0.0_f64; 4];
                for (n, &sample) in chunk.iter().enumerate() {
                    let value = f32::from(sample) / 32768.0;
                    peak = peak.max(value.abs());
                    let filter = &mut filters[n % 2];
                    for i in 0..2 {
                        filter[i] += coefficients[i] * (value - filter[i]);
                    }
                    let bands = [filter[0], filter[1] - filter[0], value - filter[1]];
                    energy[0] += f64::from(value * value);
                    for i in 0..3 {
                        energy[i + 1] += f64::from(bands[i] * bands[i]);
                    }
                }
                WavePeak {
                    peak,
                    rms: (energy[0] / chunk.len() as f64).sqrt() as f32,
                    bands: std::array::from_fn(|i| {
                        (energy[i + 1] / chunk.len() as f64).sqrt() as f32
                    }),
                }
            })
            .collect();
        let mut levels = vec![peaks.clone()];
        while levels.last().unwrap().len() > 1 {
            let next = levels
                .last()
                .unwrap()
                .chunks(2)
                .map(|pair| WavePeak::merge(pair[0], *pair.get(1).unwrap_or(&pair[0])))
                .collect();
            levels.push(next);
        }
        Self {
            peaks,
            levels: levels.into(),
            duration: samples.len() as f64 / 2.0 / f64::from(sample_rate),
            bins_per_second: f64::from(sample_rate) / hop as f64,
        }
    }

    pub fn peak_at(&self, time: f64, seconds_per_pixel: f64) -> WavePeak {
        if !time.is_finite() || !(0.0..self.duration).contains(&time) {
            return WavePeak::default();
        }
        // Each pixel reads two precomputed bins, including a full-track overview.
        let level = (seconds_per_pixel.max(1.0 / self.bins_per_second) * self.bins_per_second)
            .log2()
            .floor()
            .max(0.0) as usize;
        let level = level.min(self.levels.len() - 1);
        let peaks = &self.levels[level];
        let index = (time * self.bins_per_second / (1_u64 << level) as f64 - 0.5).max(0.0);
        let left = (index as usize).min(peaks.len() - 1);
        let right = (left + 1).min(peaks.len() - 1);
        WavePeak::interpolate(peaks[left], peaks[right], (index - index.floor()) as f32)
    }
}
