use super::{App, Message};
use iced::{
    Element, Task,
    widget::{button, text, tooltip},
};
use openbox_updater::{Client, Release, UpdateError};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub enum Event {
    Check,
    Checked(Result<Option<Release>, UpdateError>),
    Install,
    Installed(Result<PathBuf, UpdateError>),
    Restart,
    Restarted(Result<(), UpdateError>),
}

#[derive(Debug, Clone, Default)]
pub(super) enum State {
    #[default]
    Idle,
    Checking,
    Current,
    Available(Release),
    Installing(String),
    Restarting(PathBuf),
    RestartFailed(PathBuf, String),
    Failed(String),
}
impl State {
    pub fn locked(&self) -> bool {
        matches!(self, Self::Installing(_) | Self::Restarting(_))
    }
}

pub(super) type Progress = Arc<Mutex<(u64, Option<u64>)>>;

impl App {
    pub fn startup(&self) -> Task<Message> {
        check()
    }

    fn update_allowed(&self) -> bool {
        !self.busy
            && !self.preview.snapshot.decks.iter().any(|deck| deck.playing)
            && !self.preview.decks.iter().any(|deck| deck.loading)
    }

    pub(super) fn update_application(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Check if !self.updates.locked() => {
                self.updates = State::Checking;
                return check();
            }
            Event::Checked(result) => {
                self.updates = match result {
                    Ok(Some(release)) => State::Available(release),
                    Ok(None) => State::Current,
                    Err(error) => State::Failed(error.to_string()),
                }
            }
            Event::Install if self.update_allowed() => {
                if let State::Available(release) = &self.updates {
                    let release = release.clone();
                    self.updates = State::Installing(release.version.clone());
                    *self.update_progress.lock().unwrap() = (0, None);
                    let progress = Arc::clone(&self.update_progress);
                    return run(
                        move || {
                            let path = openbox_updater::installation_path()?;
                            Client::default().install(
                                env!("CARGO_PKG_VERSION"),
                                &release,
                                &path,
                                move |downloaded, total| {
                                    *progress.lock().unwrap() = (downloaded, total)
                                },
                            )?;
                            Ok(path)
                        },
                        Event::Installed,
                    );
                }
            }
            Event::Installed(Ok(path)) => return self.restart_updated(path),
            Event::Installed(Err(error)) => self.updates = State::Failed(error.to_string()),
            Event::Restart if self.update_allowed() => {
                if let State::RestartFailed(path, _) = &self.updates {
                    return self.restart_updated(path.clone());
                }
            }
            Event::Restarted(Ok(())) => return iced::exit(),
            Event::Restarted(Err(error)) => {
                if let State::Restarting(path) = &self.updates {
                    self.updates = State::RestartFailed(path.clone(), error.to_string());
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn restart_updated(&mut self, path: PathBuf) -> Task<Message> {
        self.updates = State::Restarting(path.clone());
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        run(
            move || openbox_updater::restart(&path, args),
            Event::Restarted,
        )
    }

    pub(super) fn update_footer(&self) -> Element<'_, Message> {
        let version = env!("CARGO_PKG_VERSION");
        let (label, action, hint) = match &self.updates {
            State::Available(release) => (
                format!("v{} available", release.version),
                Some(Event::Install),
                "Download, update and restart".to_owned(),
            ),
            State::Installing(version) => {
                let (done, total) = *self.update_progress.lock().unwrap();
                let progress = total.filter(|total| *total > 0).map_or_else(
                    || "...".to_owned(),
                    |total| format!(" {:.0}%", done as f64 / total as f64 * 100.0),
                );
                (
                    format!("Updating v{version}{progress}"),
                    None,
                    "Downloading and verifying the update".to_owned(),
                )
            }
            State::Checking => (
                format!("v{version} · Checking..."),
                None,
                "Checking GitHub Releases".into(),
            ),
            State::Restarting(_) => (
                "Restarting...".into(),
                None,
                "Starting the updated application".into(),
            ),
            State::RestartFailed(_, error) => (
                "Restart Openbox".into(),
                Some(Event::Restart),
                error.clone(),
            ),
            State::Failed(error) => (
                format!("v{version} · Retry update"),
                Some(Event::Check),
                error.clone(),
            ),
            State::Idle | State::Current => (
                format!("v{version} · Check"),
                Some(Event::Check),
                "Check for updates".into(),
            ),
        };
        let installing = matches!(action, Some(Event::Install | Event::Restart));
        let allowed = !installing || self.update_allowed();
        tooltip(
            button(text(label).size(10))
                .style(super::view::quiet)
                .padding([0, 4])
                .on_press_maybe(action.filter(|_| allowed).map(Message::Updates)),
            text(if allowed {
                hint
            } else {
                "Finish transfers and pause playback before updating".into()
            })
            .size(11),
            tooltip::Position::Top,
        )
        .delay(std::time::Duration::from_millis(500))
        .into()
    }
}

fn check() -> Task<Message> {
    run(
        || Client::default().check(env!("CARGO_PKG_VERSION")),
        Event::Checked,
    )
}
fn run<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, UpdateError> + Send + 'static,
    event: impl FnOnce(Result<T, UpdateError>) -> Event + Send + 'static,
) -> Task<Message> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(work)
                .await
                .unwrap_or_else(|error| Err(UpdateError::Service(error.to_string())))
        },
        move |result| Message::Updates(event(result)),
    )
}
