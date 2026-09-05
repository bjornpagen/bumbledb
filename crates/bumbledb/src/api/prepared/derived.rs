//! Sealed intermediate relations: resident images or scratch-backed rows
//! (chapter 61 sealed query sources). A spilled producer never forces a
//! complete resident rematerialization before consumption — the fallback
//! and reach resolver read either backing through one row-access contract.

use std::sync::Arc;

use crate::error::{Error, Result};
use crate::exec::scratch::ScratchRelation;
use crate::image::RelationImage;
use crate::work::WorkContext;
use bumbledb_theory::schema::ValueType;

/// One finished derived stage or rec table: either a shareable resident
/// image or an exact scratch row map keyed by insertion ordinal.
pub(crate) enum SealedStage {
    Resident(Arc<RelationImage>),
    Scratch(ScratchStage),
}

/// A derived set that exceeded resident eligibility or was sealed from a
/// spilled sink without rebuilding a complete image slab.
pub(crate) struct ScratchStage {
    pub(crate) rows: ScratchRelation,
    pub(crate) field_types: Vec<ValueType>,
    pub(crate) row_words: usize,
    pub(crate) count: u64,
}

impl SealedStage {
    /// Keep the dest that [`crate::exec::sink::AggregateSink::stream_finalize`]
    /// wrote. `dest.spilled()` / `dest.scratch_path()` are the environment
    /// witness — this never `force_spill`s or opens a second relation.
    pub(crate) fn from_aggregate_dest(
        dest: ScratchRelation,
        field_types: &[ValueType],
        count: u64,
    ) -> Self {
        debug_assert_eq!(dest.len(), count);
        let row_words = field_types
            .iter()
            .map(|ty| crate::ir::normalize::SlotWidth::of(ty).slots())
            .sum();
        Self::Scratch(ScratchStage {
            rows: dest,
            field_types: field_types.to_vec(),
            row_words,
            count,
        })
    }

    #[must_use]
    pub(crate) fn row_count(&self) -> u64 {
        match self {
            Self::Resident(image) => image.row_count() as u64,
            Self::Scratch(stage) => stage.count,
        }
    }

    #[must_use]
    pub(crate) fn is_resident(&self) -> bool {
        matches!(self, Self::Resident(_))
    }

    #[must_use]
    pub(crate) fn resident(&self) -> Option<&Arc<RelationImage>> {
        match self {
            Self::Resident(image) => Some(image),
            Self::Scratch(_) => None,
        }
    }

    /// Decode one scratch row into flat column words.
    /// # Errors
    /// Corruption or scratch read failure.
    pub(crate) fn scratch_row_words(
        stage: &mut ScratchStage,
        index: u64,
        out: &mut Vec<u64>,
    ) -> Result<()> {
        let mut encoded = Vec::new();
        if !stage.rows.get(&index.to_be_bytes(), &mut encoded)? {
            return Err(Error::Corruption(
                crate::error::CorruptionError::MalformedValue("derived scratch row"),
            ));
        }
        decode_scratch_words(stage.row_words, &encoded, out)
    }

    /// Walk every row in insertion order through L03's charged fallible
    /// visitor. `Err` is immediate; `Ok(false)` is a clean early stop.
    /// Peak decode storage is one row. The relation visit already charges
    /// the live ledger — do not step a second time per row.
    /// # Errors
    /// Storage/work failure, corruption, or visitor refusal.
    pub(crate) fn for_each_scratch_row(
        stage: &mut ScratchStage,
        work: &WorkContext,
        mut visit: impl FnMut(&[u64]) -> Result<bool>,
    ) -> Result<()> {
        work.checkpoint().map_err(super::source::work_error)?;
        let mut words = Vec::new();
        let row_words = stage.row_words;
        stage.rows.visit(&mut |_, value| {
            decode_scratch_words(row_words, value, &mut words)?;
            visit(&words)
        })
    }
}

/// Exact-width decode of a sealed scratch value. Shared by indexed get
/// and the charged visitor so both paths refuse the same corruption.
fn decode_scratch_words(row_words: usize, encoded: &[u8], out: &mut Vec<u64>) -> Result<()> {
    if encoded.len() != row_words * 8 {
        return Err(Error::Corruption(
            crate::error::CorruptionError::MalformedValue("derived scratch row"),
        ));
    }
    out.clear();
    out.reserve(row_words);
    for chunk in encoded.as_chunks::<8>().0 {
        out.push(u64::from_be_bytes(*chunk));
    }
    Ok(())
}
