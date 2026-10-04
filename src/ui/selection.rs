use std::collections::HashSet;

#[derive(Clone, Default)]
pub(super) struct Selection {
    ids: HashSet<u64>,
    anchor: Option<u64>,
}

impl Selection {
    pub fn click(&mut self, id: u64, visible: &[u64], toggle: bool, range: bool) {
        if !visible.contains(&id) {
            return;
        }
        if range
            && let Some(anchor) = self
                .anchor
                .and_then(|id| visible.iter().position(|n| *n == id))
        {
            let end = visible.iter().position(|n| *n == id).unwrap();
            if !toggle {
                self.ids.clear();
            }
            self.ids
                .extend(visible[anchor.min(end)..=anchor.max(end)].iter().copied());
        } else {
            if toggle {
                if !self.ids.remove(&id) {
                    self.ids.insert(id);
                }
            } else {
                self.ids.clear();
                self.ids.insert(id);
            }
            self.anchor = Some(id);
        }
    }

    pub fn ordered(&self, visible: &[u64]) -> Vec<u64> {
        visible
            .iter()
            .copied()
            .filter(|id| self.ids.contains(id))
            .collect()
    }

    pub fn contains(&self, id: u64) -> bool {
        self.ids.contains(&id)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn single(&self) -> Option<u64> {
        (self.ids.len() == 1).then(|| *self.ids.iter().next().unwrap())
    }

    pub fn select_all(&mut self, visible: &[u64]) {
        self.ids = visible.iter().copied().collect();
        self.anchor = visible.first().copied();
    }

    pub fn retain(&mut self, visible: &[u64]) {
        let visible: HashSet<_> = visible.iter().copied().collect();
        self.ids.retain(|id| visible.contains(id));
        if self.anchor.is_some_and(|id| !visible.contains(&id)) {
            self.anchor = None;
        }
    }

    pub fn clear(&mut self) {
        self.ids.clear();
        self.anchor = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_and_toggle_selection_follow_visible_order_without_selecting_hidden_tracks() {
        let mut selection = Selection::default();
        let visible = [8, 2, 6, 4, 10];
        selection.click(2, &visible, false, false);
        selection.click(4, &visible, false, true);
        assert_eq!(selection.ordered(&visible), [2, 6, 4]);
        selection.click(6, &visible, true, false);
        selection.click(10, &visible, true, false);
        assert_eq!(selection.ordered(&visible), [2, 4, 10]);
        selection.select_all(&[8, 10]);
        assert_eq!(selection.ordered(&visible), [8, 10]);
        selection.retain(&[10]);
        assert_eq!(selection.ordered(&visible), [10]);
        selection.clear();
        assert!(selection.ordered(&visible).is_empty());
    }
}
