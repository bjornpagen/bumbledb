//! Test-only ascending position iteration over a [`View`].

use super::View;

impl View {
    /// Iterates the view's image positions in ascending order.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: an image beyond the u32
    /// position space (the scale axiom sits orders of magnitude below).
    #[cfg(test)]
    pub fn positions(&self) -> impl Iterator<Item = u32> + '_ {
        // Chained empty arms keep one concrete iterator type without
        // boxing: exactly one arm is nonempty.
        let (all, survivors) = match self {
            Self::Unbound => (0..0u32, [].iter()),
            Self::All(image) => (
                0..u32::try_from(image.row_count()).expect("row_count < u32::MAX"),
                [].iter(),
            ),
            Self::Survivors { positions, .. } => (0..0u32, positions.iter()),
        };
        all.chain(survivors.copied())
    }
}
