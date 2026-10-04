use super::background;
use iced::Task;
use openbox_audio::{Band, Clip, DeckControl, DeckId, Engine, Snapshot};
use openbox_domain::Track;

#[derive(Debug, Clone)]
pub enum Event {
    Toggle,
    Retry,
    Load(DeckId),
    Ready(Result<Engine, String>),
    Loaded(DeckId, u64, Result<Clip, String>),
    Control(DeckId, DeckControl),
    Crossfade(f32),
    Master(f32),
    Zoom(DeckId, f64),
    ResetMixer,
}

#[derive(Clone)]
pub(super) struct Deck {
    pub track: Option<Track>,
    pub clip: Option<Clip>,
    pub pending: Option<Track>,
    pub loading: bool,
    pub generation: u64,
    pub gain: f32,
    pub fader: f32,
    pub eq: [f32; 3],
    pub pitch: f64,
    pub zoom: f64,
}

impl Default for Deck {
    fn default() -> Self {
        Self {
            track: None,
            clip: None,
            pending: None,
            loading: false,
            generation: 0,
            gain: 0.8,
            fader: 1.0,
            eq: [0.0; 3],
            pitch: 0.0,
            zoom: 16.0,
        }
    }
}

#[derive(Clone)]
pub(super) struct Preview {
    pub visible: bool,
    pub starting: bool,
    pub engine: Option<Engine>,
    pub decks: [Deck; 2],
    pub snapshot: Snapshot,
    pub crossfade: f32,
    pub master: f32,
    pub error: Option<String>,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            visible: false,
            starting: false,
            engine: None,
            decks: [Deck::default(), Deck::default()],
            snapshot: Snapshot::default(),
            crossfade: 0.5,
            master: 0.8,
            error: None,
        }
    }
}

impl Preview {
    pub fn load(&mut self, id: DeckId, track: Track) -> Task<Event> {
        let deck = &mut self.decks[id.index()];
        deck.generation += 1;
        deck.loading = true;
        deck.pending = Some(track.clone());
        if let Some(engine) = &self.engine {
            engine.control(id, DeckControl::Cue);
        }
        let generation = deck.generation;
        Task::perform(
            background(move || Clip::decode(&track.path)),
            move |result| Event::Loaded(id, generation, result),
        )
    }

    pub fn tick(&mut self) {
        if let Some(engine) = &self.engine {
            self.snapshot = engine.snapshot();
            if let Some(error) = engine.take_error() {
                self.error = Some(error);
                self.engine = None;
                for deck in &mut self.snapshot.decks {
                    deck.playing = false;
                }
            }
        }
    }

    pub fn update(&mut self, event: Event) -> Task<Event> {
        match event {
            Event::Toggle => {
                self.visible = !self.visible;
                if self.visible && self.engine.is_none() && !self.starting {
                    return self.start();
                }
            }
            Event::Retry if !self.starting => {
                self.engine = None;
                return self.start();
            }
            Event::Ready(result) => {
                self.starting = false;
                match result {
                    Ok(engine) => {
                        for id in [DeckId::A, DeckId::B] {
                            let deck = &self.decks[id.index()];
                            if let Some(clip) = &deck.clip {
                                engine.load(id, clip.clone());
                            }
                            engine.control(id, DeckControl::Gain(deck.gain));
                            engine.control(id, DeckControl::Fader(deck.fader));
                            engine.control(id, DeckControl::Rate(1.0 + deck.pitch / 100.0));
                            for band in [Band::Low, Band::Mid, Band::High] {
                                engine.control(id, DeckControl::Eq(band, deck.eq[band.index()]));
                            }
                        }
                        engine.crossfade(self.crossfade);
                        engine.master(self.master);
                        self.engine = Some(engine);
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Event::Loaded(id, generation, result) => {
                let deck = &mut self.decks[id.index()];
                if deck.generation != generation {
                    return Task::none();
                }
                deck.loading = false;
                match result {
                    Ok(clip) => {
                        if let Some(engine) = &self.engine {
                            engine.load(id, clip.clone());
                        }
                        deck.track = deck.pending.take();
                        deck.clip = Some(clip);
                        deck.pitch = 0.0;
                        self.snapshot.decks[id.index()] = openbox_audio::DeckSnapshot {
                            duration: deck.clip.as_ref().unwrap().duration(),
                            rate: 1.0,
                            ..Default::default()
                        };
                        if self.engine.is_some() {
                            self.error = None;
                        }
                    }
                    Err(error) => {
                        deck.pending = None;
                        self.error = Some(error);
                    }
                }
            }
            Event::Control(id, control) => {
                let deck = &mut self.decks[id.index()];
                match control {
                    DeckControl::Gain(value) => deck.gain = value,
                    DeckControl::Fader(value) => deck.fader = value,
                    DeckControl::Eq(band, value) => deck.eq[band.index()] = value,
                    DeckControl::Rate(value) => deck.pitch = (value - 1.0) * 100.0,
                    _ => {}
                }
                if let Some(engine) = &self.engine {
                    engine.control(id, control);
                    self.snapshot = engine.snapshot();
                }
            }
            Event::Crossfade(value) => {
                self.crossfade = value;
                if let Some(engine) = &self.engine {
                    engine.crossfade(value);
                }
            }
            Event::Master(value) => {
                self.master = value;
                if let Some(engine) = &self.engine {
                    engine.master(value);
                }
            }
            Event::Zoom(id, span) => self.decks[id.index()].zoom = span.clamp(4.0, 64.0),
            Event::ResetMixer => {
                for id in [DeckId::A, DeckId::B] {
                    let _ = self.update(Event::Control(id, DeckControl::Gain(0.8)));
                    let _ = self.update(Event::Control(id, DeckControl::Fader(1.0)));
                    for band in [Band::Low, Band::Mid, Band::High] {
                        let _ = self.update(Event::Control(id, DeckControl::Eq(band, 0.0)));
                    }
                }
                let _ = self.update(Event::Crossfade(0.5));
                let _ = self.update(Event::Master(0.8));
            }
            _ => {}
        }
        Task::none()
    }

    fn start(&mut self) -> Task<Event> {
        self.starting = true;
        self.error = None;
        Task::perform(background(Engine::start), Event::Ready)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openbox_domain::Library;

    #[test]
    fn loading_another_song_keeps_a_stale_decoder_result_out_of_the_deck() {
        let mut library = Library::default();
        let a = library.import("a.wav".into(), "First".into(), String::new());
        let b = library.import("b.wav".into(), "Second".into(), String::new());
        let mut preview = Preview::default();
        let _ = preview.load(DeckId::A, library.track(a).unwrap().clone());
        let old = preview.decks[0].generation;
        let _ = preview.load(DeckId::A, library.track(b).unwrap().clone());
        let current = preview.decks[0].generation;
        let clip = Clip::from_pcm(vec![0; 200], 100).unwrap();
        let _ = preview.update(Event::Loaded(DeckId::A, old, Ok(clip.clone())));
        assert!(preview.decks[0].loading);
        assert!(preview.decks[0].clip.is_none());
        let _ = preview.update(Event::Loaded(DeckId::A, current, Ok(clip)));
        assert!(!preview.decks[0].loading);
        assert_eq!(preview.decks[0].track.as_ref().unwrap().title, "Second");
    }
}
