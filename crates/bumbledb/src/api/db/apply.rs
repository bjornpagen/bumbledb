//! One collection applicator: poison, empty, closed, per-row apply, report.

use super::{MutationReport, WriteTx};
use crate::error::{Error, Result};
use crate::storage::delta::Disposition;
use bumbledb_theory::schema::RelationId;

/// A parsed/encoded row ready to enter the delta, or a proven no-op skip
/// (delete of a never-interned string: the fact cannot exist).
pub(super) enum ApplyRow {
    Ready,
    Skip,
}

impl<S> WriteTx<'_, S> {
    pub(super) fn refuse_poisoned(&self) -> Result<()> {
        match &self.phase {
            WritePhase::Poisoned(source) => Err(Error::TransactionPoisoned {
                source: source.clone(),
            }),
            WritePhase::Clean | WritePhase::Applied => Ok(()),
        }
    }

    /// After a prefix entered: store the original error for later
    /// mutation / `Db::write`. The first failing call still returns
    /// `err` itself.
    pub(super) fn poison(&mut self, err: Error) -> Error {
        if let WritePhase::Applied = self.phase {
            self.phase = WritePhase::Poisoned(Box::new(err.clone()));
        }
        err
    }

    fn note_entered(&mut self) {
        if let WritePhase::Clean = self.phase {
            self.phase = WritePhase::Applied;
        }
    }

    /// Parse-then-apply loop. Poison and the report live here — typed
    /// encode and dyn [`super::encode_dyn::ParsedRow`] both arrive as
    /// already-checked rows.
    pub(super) fn apply_collection<T>(
        &mut self,
        relation: RelationId,
        want: Disposition,
        facts: impl IntoIterator<Item = T>,
        mut encode: impl FnMut(&mut Self, T, &mut Vec<u8>) -> Result<ApplyRow>,
    ) -> Result<MutationReport> {
        self.refuse_poisoned()?;
        let mut iter = facts.into_iter();
        let Some(first) = iter.next() else {
            return Ok(MutationReport::EMPTY);
        };
        self.refuse_closed(relation)?;
        self.apply_rows(
            relation,
            want,
            std::iter::once(first).chain(iter),
            &mut encode,
        )
    }

    /// The apply walk after a nonempty collection has passed shape/closed.
    fn apply_rows<T>(
        &mut self,
        relation: RelationId,
        want: Disposition,
        facts: impl IntoIterator<Item = T>,
        encode: &mut impl FnMut(&mut Self, T, &mut Vec<u8>) -> Result<ApplyRow>,
    ) -> Result<MutationReport> {
        let mut submitted = 0u64;
        let mut changed = 0u64;
        for fact in facts {
            submitted += 1;
            let result = self.with_scratch(|tx, bytes| match encode(tx, fact, bytes)? {
                ApplyRow::Skip => Ok(false),
                ApplyRow::Ready => tx.delta.apply(&tx.view, relation, bytes, want),
            });
            match result {
                Ok(false) => {}
                Ok(true) => {
                    self.note_entered();
                    changed += 1;
                }
                Err(error) => return Err(self.poison(error)),
            }
        }
        Ok(MutationReport::from_counts(submitted, changed))
    }
}

/// Apply state of one write transaction. Three states, all legal.
pub(super) enum WritePhase {
    /// No fact has entered the delta.
    Clean,
    /// At least one fact entered (a no-op does not count).
    Applied,
    /// An apply failure after [`Self::Applied`]. Later mutation returns
    /// [`Error::TransactionPoisoned`] wrapping this error.
    Poisoned(Box<Error>),
}
