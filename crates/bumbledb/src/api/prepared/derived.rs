//! Sealed intermediate relations: the finished stages and rec tables rules
//! join. Only resident images are joinable.

use std::sync::Arc;

use crate::image::RelationImage;

/// One finished derived stage or rec table.
pub(crate) enum SealedStage {
    Resident(Arc<RelationImage>),
    /// An aggregate stage finalized outside RAM. A rule reading it refuses
    /// with `Capacity::ResidentRows`.
    Scratch,
}

impl SealedStage {
    #[must_use]
    pub(crate) fn is_resident(&self) -> bool {
        matches!(self, Self::Resident(_))
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn resident(&self) -> Option<&Arc<RelationImage>> {
        match self {
            Self::Resident(image) => Some(image),
            Self::Scratch => None,
        }
    }
}
