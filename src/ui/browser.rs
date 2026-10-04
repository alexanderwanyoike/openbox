use super::selection::Selection;
use openbox_domain::{Library, Track};

pub(super) const ROW_HEIGHT: f32 = 28.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Order,
    Title,
    Artist,
    Album,
    Genre,
    Bpm,
    Key,
    Duration,
}

#[derive(Clone)]
pub(super) struct TrackBrowser {
    pub query: String,
    pub selection: Selection,
    pub sort: SortColumn,
    pub descending: bool,
    pub offset: f32,
    pub height: f32,
}

impl Default for TrackBrowser {
    fn default() -> Self {
        Self {
            query: String::new(),
            selection: Selection::default(),
            sort: SortColumn::Order,
            descending: false,
            offset: 0.0,
            height: 500.0,
        }
    }
}

impl TrackBrowser {
    pub fn visible<'a>(&self, library: &'a Library, playlist: Option<u64>) -> Vec<&'a Track> {
        let mut tracks: Vec<_> = match playlist {
            Some(id) => library
                .playlist(id)
                .map(|p| {
                    p.track_ids
                        .iter()
                        .filter_map(|id| library.track(*id))
                        .collect()
                })
                .unwrap_or_default(),
            None => library.tracks().iter().collect(),
        };
        let query = self.query.trim().to_lowercase();
        if !query.is_empty() {
            tracks.retain(|t| {
                format!(
                    "{} {} {} {} {} {}",
                    t.title,
                    t.artist,
                    t.details.album,
                    t.details.genre,
                    t.details.key,
                    t.details.bpm.map(|b| b.to_string()).unwrap_or_default()
                )
                .to_lowercase()
                .contains(&query)
            });
        }
        if self.sort != SortColumn::Order {
            tracks.sort_by(|a, b| {
                let order = match self.sort {
                    SortColumn::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                    SortColumn::Artist => a.artist.to_lowercase().cmp(&b.artist.to_lowercase()),
                    SortColumn::Album => a
                        .details
                        .album
                        .to_lowercase()
                        .cmp(&b.details.album.to_lowercase()),
                    SortColumn::Genre => a
                        .details
                        .genre
                        .to_lowercase()
                        .cmp(&b.details.genre.to_lowercase()),
                    SortColumn::Bpm => a
                        .details
                        .bpm
                        .unwrap_or(0.0)
                        .total_cmp(&b.details.bpm.unwrap_or(0.0)),
                    SortColumn::Key => a.details.key.cmp(&b.details.key),
                    SortColumn::Duration => {
                        a.details.duration_seconds.cmp(&b.details.duration_seconds)
                    }
                    SortColumn::Order => std::cmp::Ordering::Equal,
                };
                if self.descending {
                    order.reverse()
                } else {
                    order
                }
            });
        }
        tracks
    }

    pub fn ids(&self, library: &Library, playlist: Option<u64>) -> Vec<u64> {
        self.visible(library, playlist)
            .iter()
            .map(|t| t.id)
            .collect()
    }

    pub fn sort_by(&mut self, column: SortColumn) {
        self.descending = column != SortColumn::Order && self.sort == column && !self.descending;
        self.sort = column;
        self.offset = 0.0;
    }

    pub fn window(&self, count: usize) -> std::ops::Range<usize> {
        let start = (self.offset / ROW_HEIGHT).floor().max(0.0) as usize;
        start.saturating_sub(3).min(count)
            ..(start + (self.height / ROW_HEIGHT).ceil() as usize + 4).min(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtering_and_sorting_keep_bulk_selection_in_the_displayed_order() {
        let mut library = Library::default();
        let a = library.import("a.mp3".into(), "Zulu".into(), "House".into());
        let b = library.import("b.mp3".into(), "Alpha".into(), "House".into());
        let c = library.import("c.mp3".into(), "Hidden".into(), "Techno".into());
        let mut browser = TrackBrowser {
            query: "house".into(),
            ..Default::default()
        };
        browser.sort_by(SortColumn::Title);
        let visible = browser.ids(&library, None);
        browser.selection.select_all(&visible);
        assert_eq!(browser.selection.ordered(&visible), [b, a]);
        assert!(!browser.selection.contains(c));
        browser.sort_by(SortColumn::Title);
        assert_eq!(
            browser.selection.ordered(&browser.ids(&library, None)),
            [a, b]
        );
        browser.offset = 28.0 * 400.0;
        browser.height = 280.0;
        let window = browser.window(1000);
        assert!(window.start <= 400 && window.end >= 410);
        assert!(window.len() < 25);
    }
}
