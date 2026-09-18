//! Typed dependency projections. A contextual Event constant is syntax with a
//! world-space interpretation; it is never a Boolean value or a fabricated field.

use super::FieldId;

/// The ordinary field tuple, optionally followed by the full Event of the
/// statement group's checked world context. The full constant is supported only
/// in the trailing region position. Fields before it must be scalar; schema
/// admission establishes that premise. Field-only containments may permute their
/// region position along with the other fields.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Projection<F = FieldId> {
    Fields(Box<[F]>),
    EventFull(Box<[F]>),
}

impl<F> Projection<F> {
    /// Stored fields in their authored order. This excludes the contextual
    /// constant; callers needing logical arity must use [`Self::arity`].
    #[must_use]
    pub fn fields(&self) -> &[F] {
        match self {
            Self::Fields(fields) | Self::EventFull(fields) => fields,
        }
    }

    #[must_use]
    pub const fn is_event_full(&self) -> bool {
        matches!(self, Self::EventFull(_))
    }

    /// Number of logical tuple positions, including the constant if present.
    #[must_use]
    pub fn arity(&self) -> usize {
        self.fields().len() + usize::from(self.is_event_full())
    }
}

impl<F> From<Box<[F]>> for Projection<F> {
    fn from(fields: Box<[F]>) -> Self {
        Self::Fields(fields)
    }
}

impl<F, const N: usize> From<Box<[F; N]>> for Projection<F> {
    fn from(fields: Box<[F; N]>) -> Self {
        Self::Fields(fields)
    }
}

impl<F> From<Vec<F>> for Projection<F> {
    fn from(fields: Vec<F>) -> Self {
        Self::Fields(fields.into_boxed_slice())
    }
}

impl<F, const N: usize> From<[F; N]> for Projection<F> {
    fn from(fields: [F; N]) -> Self {
        Self::Fields(Box::new(fields))
    }
}

impl<F: Clone> From<&[F]> for Projection<F> {
    fn from(fields: &[F]) -> Self {
        Self::Fields(fields.into())
    }
}

impl<F> FromIterator<F> for Projection<F> {
    fn from_iter<T: IntoIterator<Item = F>>(iter: T) -> Self {
        Self::Fields(iter.into_iter().collect())
    }
}
