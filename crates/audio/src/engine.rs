use crate::{Clip, DeckControl, DeckId, Mixer, Snapshot};
use cpal::{
    FromSample, SizedSample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use std::sync::{Arc, Mutex, mpsc};

struct Shared {
    mixer: Arc<Mutex<Mixer>>,
    error: Arc<Mutex<Option<String>>>,
    shutdown: mpsc::Sender<()>,
    device: String,
}

impl Drop for Shared {
    fn drop(&mut self) {
        let _ = self.shutdown.send(());
    }
}

#[derive(Clone)]
pub struct Engine(Arc<Shared>);

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Engine").field(&self.0.device).finish()
    }
}

impl Engine {
    pub fn start() -> Result<Self, String> {
        let mixer = Arc::new(Mutex::new(Mixer::default()));
        let error = Arc::new(Mutex::new(None));
        let (shutdown, stopped) = mpsc::channel();
        let (ready, started) = mpsc::sync_channel(1);
        let audio = mixer.clone();
        let errors = error.clone();
        std::thread::Builder::new()
            .name("openbox-audio".into())
            .spawn(move || match output(audio, errors) {
                Ok((stream, device)) => {
                    let _ = ready.send(Ok(device));
                    let _ = stopped.recv();
                    drop(stream);
                }
                Err(error) => {
                    let _ = ready.send(Err(error));
                }
            })
            .map_err(|e| e.to_string())?;
        let device = started.recv().map_err(|e| e.to_string())??;
        Ok(Self(Arc::new(Shared {
            mixer,
            error,
            shutdown,
            device,
        })))
    }

    pub fn load(&self, deck: DeckId, clip: Clip) {
        self.0.mixer.lock().unwrap().load(deck, clip);
    }
    pub fn control(&self, deck: DeckId, control: DeckControl) {
        self.0.mixer.lock().unwrap().control(deck, control);
    }
    pub fn crossfade(&self, value: f32) {
        self.0.mixer.lock().unwrap().set_crossfade(value);
    }
    pub fn master(&self, value: f32) {
        self.0.mixer.lock().unwrap().set_master(value);
    }
    pub fn snapshot(&self) -> Snapshot {
        self.0.mixer.lock().unwrap().snapshot()
    }
    pub fn device(&self) -> &str {
        &self.0.device
    }
    pub fn take_error(&self) -> Option<String> {
        self.0.error.lock().unwrap().take()
    }
}

fn output(
    mixer: Arc<Mutex<Mixer>>,
    error: Arc<Mutex<Option<String>>>,
) -> Result<(cpal::Stream, String), String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("No audio output is available. Check your desktop sound settings.")?;
    let supported = device
        .default_output_config()
        .map_err(|e| format!("Cannot open audio output: {e}"))?;
    let name = device.name().unwrap_or_else(|_| "Default output".into());
    let config = supported.config();
    if config.channels == 0 || config.channels > 32 {
        return Err("Unsupported audio output channel count".into());
    }
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => build::<f32>(&device, &config, mixer, error),
        cpal::SampleFormat::F64 => build::<f64>(&device, &config, mixer, error),
        cpal::SampleFormat::I16 => build::<i16>(&device, &config, mixer, error),
        cpal::SampleFormat::I32 => build::<i32>(&device, &config, mixer, error),
        cpal::SampleFormat::U16 => build::<u16>(&device, &config, mixer, error),
        format => return Err(format!("Unsupported audio output format: {format}")),
    }
    .map_err(|e| format!("Cannot start audio output: {e}"))?;
    stream
        .play()
        .map_err(|e| format!("Cannot play audio: {e}"))?;
    Ok((stream, name))
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mixer: Arc<Mutex<Mixer>>,
    error: Arc<Mutex<Option<String>>>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    let channels = usize::from(config.channels);
    let rate = config.sample_rate.0;
    let mut scratch = [0.0_f32; 4096];
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            // A busy UI must never block the output callback or allocate on its audio thread.
            let Ok(mut mixer) = mixer.try_lock() else {
                data.fill(T::EQUILIBRIUM);
                return;
            };
            for chunk in data.chunks_mut(4096 / channels * channels) {
                let rendered = &mut scratch[..chunk.len()];
                mixer.render(rendered, rate, channels);
                for (target, &sample) in chunk.iter_mut().zip(rendered.iter()) {
                    *target = T::from_sample(sample);
                }
            }
        },
        move |e| {
            *error.lock().unwrap() = Some(format!("Audio output stopped: {e}"));
        },
        None,
    )
}
