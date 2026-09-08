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
    Scratch(Box<ScratchStage>),
}

/// A derived set that exceeded resident eligibility or was sealed from a
/// spilled sink without rebuilding a complete image slab.
pub(crate) struct ScratchStage {
    pub(crate) rows: ScratchRelation,
    pub(crate) field_types: Vec<ValueType>,
    pub(crate) row_words: usize,
    pub(crate) count: u64,
    pub(crate) generation: crate::work::GenerationHandle,
    pub(crate) texts: crate::image::TextOwners,
}

/// One reusable row for nested scans that must release the stage borrow
/// before evaluating the next join depth. This never collects a stage.
#[derive(Default)]
pub(crate) struct ScratchRowBuffer {
    encoded: Vec<u8>,
    words: Vec<u64>,
}

impl ScratchRowBuffer {
    pub(crate) fn read(&mut self, stage: &mut ScratchStage, index: u64) -> Result<&[u64]> {
        if !stage.rows.get(&index.to_be_bytes(), &mut self.encoded)? {
            return Err(Error::Corruption(
                crate::error::CorruptionError::MalformedValue("derived scratch row"),
            ));
        }
        decode_scratch_words(stage.row_words, &self.encoded, &mut self.words)?;
        Ok(&self.words)
    }
}

impl SealedStage {
    pub(super) fn check_generation(
        &self,
        generation: &crate::work::GenerationHandle,
    ) -> Result<()> {
        let owner = match self {
            Self::Resident(image) => image.generation(),
            Self::Scratch(stage) => &stage.generation,
        };
        if owner.ptr_eq(generation) {
            Ok(())
        } else {
            Err(Error::Corruption(
                crate::error::CorruptionError::MalformedValue("derived stage text generation"),
            ))
        }
    }

    /// Keep the dest that [`crate::exec::sink::AggregateSink::stream_finalize`]
    /// wrote. `dest.spilled()` / `dest.scratch_path()` are the environment
    /// witness — this never `force_spill`s or opens a second relation.
    pub(crate) fn from_aggregate_dest(
        dest: ScratchRelation,
        field_types: &[ValueType],
        count: u64,
        generation: crate::work::GenerationHandle,
        texts: crate::image::TextOwners,
    ) -> Self {
        debug_assert_eq!(dest.len(), count);
        let row_words = field_types
            .iter()
            .map(|ty| crate::ir::normalize::SlotWidth::of(ty).slots())
            .sum();
        Self::Scratch(Box::new(ScratchStage {
            rows: dest,
            field_types: field_types.to_vec(),
            row_words,
            count,
            generation,
            texts,
        }))
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
    #[cfg(test)]
    pub(crate) fn resident(&self) -> Option<&Arc<RelationImage>> {
        match self {
            Self::Resident(image) => Some(image),
            Self::Scratch(_) => None,
        }
    }

    /// Walk every row in insertion order through the fallible
    /// visitor. `Err` is immediate; `Ok(false)` is a clean early stop.
    /// Peak decode storage is one row; the relation visitor checks cancellation.
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
        stage.rows.visit(&mut |_: &[u8], value: &[u8]| {
            work.checkpoint().map_err(super::source::work_error)?;
            decode_scratch_words(row_words, value, &mut words)?;
            visit(&words)
        })
    }
}

/// Exact-width decode of a sealed scratch value. Shared by indexed get
/// and the borrowed visitor so both paths refuse the same corruption.
fn decode_scratch_words(row_words: usize, encoded: &[u8], out: &mut Vec<u64>) -> Result<()> {
    if row_words.checked_mul(8) != Some(encoded.len()) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn stage(disk: bool) -> ScratchStage {
        let mut rows = ScratchRelation::new(&WorkContext::new());
        if disk {
            rows.force_spill().unwrap();
        }
        for index in 0..1024u64 {
            rows.put(&index.to_be_bytes(), &(index * 3).to_be_bytes())
                .unwrap();
        }
        ScratchStage {
            rows,
            field_types: vec![ValueType::U64],
            row_words: 1,
            count: 1024,
            generation: crate::image::test_generation(),
            texts: crate::image::TextOwners::default(),
        }
    }

    #[test]
    fn nested_scratch_reads_reuse_two_row_buffers_without_row_allocations() {
        for disk in [false, true] {
            let mut stage = stage(disk);
            let mut outer = ScratchRowBuffer::default();
            let mut inner = ScratchRowBuffer::default();
            outer.read(&mut stage, 0).unwrap();
            inner.read(&mut stage, 0).unwrap();
            #[cfg(feature = "alloc-counter")]
            let before = crate::alloc_counter::snapshot().window;
            for index in 0..1024u64 {
                let row = outer.read(&mut stage, index).unwrap();
                assert_eq!(
                    inner.read(&mut stage, 1023 - index).unwrap(),
                    &[(1023 - index) * 3]
                );
                assert_eq!(
                    row,
                    &[index * 3],
                    "the next depth cannot overwrite its parent"
                );
            }
            #[cfg(feature = "alloc-counter")]
            if !disk {
                assert_eq!(crate::alloc_counter::snapshot().window, before);
            }
            assert!(outer.read(&mut stage, 1024).is_err());
        }
    }

    #[test]
    fn scratch_visitors_borrow_rows_and_stop_before_later_corruption() {
        for disk in [false, true] {
            let mut stage = stage(disk);
            stage.rows.put(&1u64.to_be_bytes(), &[0]).unwrap();
            let mut visited = 0;
            SealedStage::for_each_scratch_row(&mut stage, &WorkContext::new(), |row| {
                assert_eq!(row, &[0]);
                visited += 1;
                Ok(false)
            })
            .unwrap();
            assert_eq!(visited, 1);
            let result =
                SealedStage::for_each_scratch_row(&mut stage, &WorkContext::new(), |_| Ok(true));
            assert!(matches!(result, Err(Error::Corruption(_))));
            assert!(ScratchRowBuffer::default().read(&mut stage, 1).is_err());
            stage.row_words = usize::MAX;
            assert!(ScratchRowBuffer::default().read(&mut stage, 0).is_err());
        }
    }

    #[test]
    fn scratch_visit_observes_consumer_cancellation_with_a_live_producer() {
        for disk in [false, true] {
            let mut stage = stage(disk);
            let work = WorkContext::new();
            let mut visited = 0;
            let result = SealedStage::for_each_scratch_row(&mut stage, &work, |_| {
                visited += 1;
                work.cancel();
                Ok(true)
            });
            assert!(matches!(result, Err(Error::Store(error)) if matches!(
                *error, crate::storage::store::StoreError::Work(crate::work::WorkError::Cancelled)
            )));
            assert_eq!(visited, 1);
            let mut visited = 0;
            SealedStage::for_each_scratch_row(&mut stage, &WorkContext::new(), |_| {
                visited += 1;
                Ok(true)
            })
            .unwrap();
            assert_eq!(visited, 1024);
        }
    }
}
