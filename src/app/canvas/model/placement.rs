pub(in crate::app::canvas) const CARD_WIDTH: f64 = 240.;
pub(in crate::app::canvas) const CARD_HEIGHT: f64 = 156.;

pub(in crate::app::canvas) fn slot(index: usize) -> [f64; 2] {
    [
        64. + ((index % 4 + (index / 336) * 4) % 60) as f64 * 320.,
        64. + (index / 4 % 84) as f64 * 232.,
    ]
}

pub(in crate::app::canvas) fn vacant(positions: impl IntoIterator<Item = [f64; 2]>) -> [f64; 2] {
    let positions: Vec<_> = positions.into_iter().collect();
    (0..5040)
        .map(slot)
        .find(|p| {
            positions.iter().all(|q| {
                (p[0] - q[0]).abs() >= CARD_WIDTH + 24. || (p[1] - q[1]).abs() >= CARD_HEIGHT + 24.
            })
        })
        .unwrap_or([64., 64.])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canvas_001_legacy_slots_and_vacant_position_stay_stable() {
        assert_eq!(slot(0), [64., 64.]);
        assert_eq!(slot(4), [64., 296.]);
        assert_eq!(vacant([slot(0), slot(1)]), slot(2));
        assert_eq!(slot(336), [1344., 64.]);
    }
}
