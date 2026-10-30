//! Bounded undo/redo history, independent of the board and GTK.
pub(in crate::app::canvas) struct History<T> {
    past: Vec<T>,
    future: Vec<T>,
    limit: usize,
}

impl<T> History<T> {
    pub fn new(limit: usize) -> Self {
        assert!(limit > 0);
        Self {
            past: Vec::new(),
            future: Vec::new(),
            limit,
        }
    }

    pub fn record(&mut self, before: T) {
        if self.past.len() == self.limit {
            self.past.remove(0);
        }
        self.past.push(before);
        self.future.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.past.len()
    }

    pub fn step(&mut self, current: T, backwards: bool) -> Option<T> {
        let (from, to) = if backwards {
            (&mut self.past, &mut self.future)
        } else {
            (&mut self.future, &mut self.past)
        };
        let previous = from.pop()?;
        to.push(current);
        Some(previous)
    }
}

#[cfg(test)]
mod tests {
    use super::History;

    #[test]
    fn canvas_003_history_bounds_branches_and_empty_steps() {
        let mut h = History::new(2);
        assert_eq!(h.step(0, true), None);
        h.record(0);
        h.record(1);
        h.record(2);
        assert_eq!(h.len(), 2);
        assert_eq!(h.step(3, true), Some(2));
        assert_eq!(h.step(2, true), Some(1));
        assert_eq!(h.step(1, true), None);
        assert_eq!(h.step(1, false), Some(2));
        h.record(2);
        assert!(!h.can_redo());
        assert_eq!(h.step(9, true), Some(2));
    }
}
