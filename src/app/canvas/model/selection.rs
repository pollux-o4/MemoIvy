use uuid::Uuid;

/// 선택 순서를 보존한다. 마지막 항목이 패널에 표시되는 주 선택이다.
#[derive(Clone, Default, PartialEq, Debug)]
pub(in crate::app::canvas) struct Selection(Vec<Uuid>);

impl Selection {
    pub fn ids(&self) -> &[Uuid] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn contains(&self, id: Uuid) -> bool {
        self.0.contains(&id)
    }
    /// 하나만 선택됐을 때의 대상. 다중 선택은 정렬 도구가 맡는다.
    pub fn single(&self) -> Option<Uuid> {
        match self.0.as_slice() {
            [id] => Some(*id),
            _ => None,
        }
    }
    pub fn set(&mut self, ids: impl IntoIterator<Item = Uuid>) {
        self.0.clear();
        for id in ids {
            if !self.0.contains(&id) {
                self.0.push(id);
            }
        }
    }
    pub fn toggle(&mut self, id: Uuid) {
        if let Some(index) = self.0.iter().position(|other| *other == id) {
            self.0.remove(index);
        } else {
            self.0.push(id);
        }
    }
    pub fn retain(&mut self, keep: impl Fn(Uuid) -> bool) {
        self.0.retain(|id| keep(*id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_001_selection_is_unique_and_ordered() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let mut selection = Selection::default();
        selection.set([a, a]);
        assert_eq!(selection.single(), Some(a));
        selection.toggle(b);
        assert_eq!(selection.ids(), [a, b]);
        assert_eq!(selection.single(), None);
        selection.toggle(a);
        assert_eq!(selection.ids(), [b]);
        selection.retain(|id| id != b);
        assert!(selection.is_empty());
    }
}
