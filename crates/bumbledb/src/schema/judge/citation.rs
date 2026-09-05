//! Canonical bounded citation selection (C4 / CORE-021).
//!
//! Offending facts are ranked by portable [`crate::canonical::fact_sort_key`]
//! bytes **before** the example budget truncates. Local row ids, insertion
//! order and physical reminting cannot change the kept set. Resource
//! exhaustion refuses; it never becomes a shorter verdict.

use crate::canonical::{self, RowError};
use crate::schema::{RelationId, Schema};
use crate::work::ByteReservation;
use crate::{Value, WorkContext};

use super::grouped::values_charge;
use super::{CandidateFact, JudgeError};

/// Bounded top-k over canonical fact bytes. Capacity is the labeled
/// example budget; zero keeps the verdict and cites nothing.
pub(super) struct CitationTopK {
    budget: usize,
    /// Strictly increasing sort keys; length ≤ budget.
    chosen: Vec<(Vec<u8>, CandidateFact)>,
    charges: Vec<ByteReservation>,
    extra: bool,
    considered: u64,
}

impl CitationTopK {
    pub(super) fn new(budget: usize) -> Self {
        Self {
            budget,
            chosen: Vec::new(),
            charges: Vec::new(),
            extra: false,
            considered: 0,
        }
    }

    /// Offer one decoded fact. Selection is by logical bytes, then
    /// truncation. Duplicates of the same sort key collapse.
    pub(super) fn offer<E>(
        &mut self,
        schema: &Schema,
        work: &WorkContext,
        relation: RelationId,
        values: &[Value],
    ) -> Result<(), JudgeError<E>> {
        let fields = schema.relation(relation).fields();
        let key = canonical::fact_sort_key(fields, values, work).map_err(|error| match error {
            RowError::Work(work) => JudgeError::Work(work),
            _ => unreachable!("citation keys follow already-decoded rows"),
        })?;
        if self
            .chosen
            .iter()
            .any(|(existing, fact)| existing == &key && fact.relation == relation)
        {
            return Ok(());
        }
        self.considered = self.considered.saturating_add(1);
        if self.budget == 0 {
            self.extra = true;
            return Ok(());
        }
        let at = self
            .chosen
            .binary_search_by(|(existing, _)| existing.as_slice().cmp(key.as_slice()));
        match at {
            Ok(_) => Ok(()),
            Err(index) if self.chosen.len() < self.budget => {
                self.push_at(schema, work, index, key, relation, values)
            }
            Err(index) if index < self.budget => {
                self.extra = true;
                let _ = self.chosen.pop();
                let _ = self.charges.pop();
                self.push_at(schema, work, index, key, relation, values)
            }
            Err(_) => {
                self.extra = true;
                Ok(())
            }
        }
    }

    fn push_at<E>(
        &mut self,
        _schema: &Schema,
        work: &WorkContext,
        index: usize,
        key: Vec<u8>,
        relation: RelationId,
        values: &[Value],
    ) -> Result<(), JudgeError<E>> {
        let charge = work
            .reserve(crate::work::ByteKind::Working, values_charge(values))
            .map_err(JudgeError::Work)?;
        self.charges.insert(index, charge);
        self.chosen.insert(
            index,
            (
                key,
                CandidateFact {
                    relation,
                    values: values.to_vec().into_boxed_slice(),
                },
            ),
        );
        Ok(())
    }

    #[must_use]
    pub(super) fn truncated(&self) -> bool {
        self.extra || self.considered > u64::try_from(self.budget).unwrap_or(u64::MAX)
    }

    pub(super) fn into_examples(
        self,
    ) -> (
        Box<[CandidateFact]>,
        bool,
        Vec<ByteReservation>,
    ) {
        let truncated = self.truncated();
        let examples = self
            .chosen
            .into_iter()
            .map(|(_, fact)| fact)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        (examples, truncated, self.charges)
    }
}
