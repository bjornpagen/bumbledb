//! Exact comparison of pinned text tokens in one resolver generation.
//!
//! Callers retain canonical text owners alongside token-bearing images,
//! constants or execution state. Tokens are unique within the bound generation;
//! comparisons need neither byte copies nor an interner lock on the warm path.

use super::intern::SENTINEL_WORD;
use crate::error::{CorruptionError, Error, Result};
use crate::work::GenerationHandle;

#[derive(Clone, Copy)]
pub struct TextEq<'a> {
    generation: Option<&'a GenerationHandle>,
}

impl<'a> TextEq<'a> {
    #[must_use]
    pub const fn bind(generation: &'a GenerationHandle) -> Self {
        Self::from_optional_generation(Some(generation))
    }

    pub(crate) const fn from_optional_generation(generation: Option<&'a GenerationHandle>) -> Self {
        Self { generation }
    }

    /// Identity of an already-pinned token. The missing-text sentinel is not
    /// an identity. Crossing generations requires `GenerationHandle::tokens_equal`.
    pub fn canonical(self, token: u64) -> Result<Option<u64>> {
        self.generation
            .ok_or(Error::Corruption(CorruptionError::MalformedValue(
                "text outside a sealed text-free probe",
            )))?;
        Ok((token != SENTINEL_WORD).then_some(token))
    }

    pub fn identity(self, token: u64) -> Result<Option<u64>> {
        self.canonical(token)
    }

    pub fn tokens_equal(self, left: u64, right: u64) -> Result<bool> {
        Ok(
            matches!((self.canonical(left)?, self.canonical(right)?), (Some(left), Some(right)) if left == right),
        )
    }
}
