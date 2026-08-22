//! Test-only ascending position iteration over a [`View`].

use super::View;

impl View {
    /// # Panics

    /// Only on a programmer-invariant violation: an image beyond the u32
    #[cfg(test)]
    pub fn positions(&self) -> impl Iterator<Item = u32> + '_ {
        let (all, survivors) = match self {
            Self::Unbound => (0..0u32, [].iter()),
            Self::Bound(super::BoundView::All(image)) => (
                0..u32::try_from(image.row_count()).expect("row_count < u32::MAX"),
                [].iter(),
            ),
            Self::Bound(super::BoundView::Survivors { positions, .. }) => {
                (0..0u32, positions.iter())
            }
        };
        all.chain(survivors.copied())
    }
}
