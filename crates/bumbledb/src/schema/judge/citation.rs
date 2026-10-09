//! Bounded citation selection: offending facts are ranked by canonical
//! [`crate::canonical::fact_sort_key`] bytes before the example budget
//! truncates, so row ids and insertion order cannot change the kept set.

use crate::canonical::{self, CanonicalRow};
use crate::error::Result;
use crate::schema::{RelationId, Schema};
use crate::{Value, WorkContext};

use super::CandidateFact;

/// Bounded top-k over canonical fact bytes. Capacity is the labeled
/// example budget; zero keeps the verdict and cites nothing.
pub(super) struct CitationTopK {
    budget: usize,
    /// Strictly increasing sort keys; length ≤ budget.
    chosen: Vec<(CanonicalRow, CandidateFact)>,
    extra: bool,
    considered: u64,
}

impl CitationTopK {
    pub(super) fn new(budget: usize) -> Self {
        Self {
            budget,
            chosen: Vec::new(),
            extra: false,
            considered: 0,
        }
    }

    /// Offer one decoded fact. Selection is by logical bytes, then
    /// truncation. Duplicates of the same sort key collapse.
    pub(super) fn offer(
        &mut self,
        schema: &Schema,
        work: &WorkContext,
        relation: RelationId,
        values: &[Value],
    ) -> Result<()> {
        let fields = schema.relation(relation).fields();
        let key = canonical::fact_sort_key(fields, values, work)?;
        let at = self.chosen.binary_search_by(|(existing, fact)| {
            (fact.relation, existing.as_bytes()).cmp(&(relation, key.as_bytes()))
        });
        if at.is_ok() {
            return Ok(());
        }
        self.considered = self.considered.saturating_add(1);
        if self.budget == 0 {
            self.extra = true;
            return Ok(());
        }
        match at {
            Ok(_) => Ok(()),
            Err(index) if self.chosen.len() < self.budget => {
                self.push_at(index, key, relation, values);
                Ok(())
            }
            Err(index) if index < self.budget => {
                self.extra = true;
                let _ = self.chosen.pop();
                self.push_at(index, key, relation, values);
                Ok(())
            }
            Err(_) => {
                self.extra = true;
                Ok(())
            }
        }
    }

    fn push_at(&mut self, index: usize, key: CanonicalRow, relation: RelationId, values: &[Value]) {
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
    }

    #[must_use]
    pub(super) fn truncated(&self) -> bool {
        self.extra || self.considered > u64::try_from(self.budget).unwrap_or(u64::MAX)
    }

    pub(super) fn into_examples(self) -> (Box<[CandidateFact]>, bool) {
        let truncated = self.truncated();
        let examples = self
            .chosen
            .into_iter()
            .map(|(_, fact)| fact)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        (examples, truncated)
    }
}
