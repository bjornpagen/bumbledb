//! Binding of P01's reference final-state judge (C03,
//! `crate::schema::judge`) to the store's candidate protocol (C04).
//!
//! [`SchemaJudge`] is the production judgment: it presents the candidate
//! transaction's proposed final state to `judge_final_state`, which decides
//! by exact decoded values — no fingerprint participates in a verdict, so a
//! forced bucket collision can slow judgment but never change it.
//!
//! [`UnindexedRows`] is the selected reference indexing path: the complete
//! judge inspects full affected relations (chapter 10 allows exactly this),
//! so the store maintains no determinant acceleration entries until a
//! schema-derived indexer establishes equivalence. It is the reference
//! semantics, not a stub: admission is complete without acceleration.

use bumbledb_theory::schema::{RelationId, StatementId};

use super::candidate::{CandidateJudge, CandidateState, Judgment, RowIndexer};
use super::error::{StoreCorruption, StoreError, StoreResult};
use crate::Value;
use crate::schema::Schema;
use crate::schema::judge::{
    CandidateFacts, JudgeBudget, JudgeError, JudgedViolation, Judgment as SchemaJudgment,
    judge_final_state,
};
use crate::work::WorkContext;

/// The production C03 judge over a store candidate.
#[derive(Debug, Clone, Copy)]
pub struct SchemaJudge<'s> {
    pub schema: &'s Schema,
    pub budget: JudgeBudget,
}

impl<'s> SchemaJudge<'s> {
    #[must_use]
    pub fn new(schema: &'s Schema) -> Self {
        Self {
            schema,
            budget: JudgeBudget::default(),
        }
    }
}

struct CandidateView<'v, 'a, 'store> {
    state: &'v CandidateState<'a, 'store>,
    schema: &'v Schema,
    work: &'v WorkContext,
}

impl CandidateFacts for CandidateView<'_, '_, '_> {
    type Error = StoreError;

    fn rows(
        &self,
        relation: RelationId,
    ) -> Box<dyn Iterator<Item = Result<Box<[Value]>, Self::Error>> + '_> {
        let Some(view) = self.schema.relation_checked(relation) else {
            return Box::new(std::iter::once(Err(StoreError::Corruption(
                StoreCorruption::MalformedKey("judged relation unknown to schema"),
            ))));
        };
        let fields = view.fields();
        match self.state.rows(relation) {
            Err(error) => Box::new(std::iter::once(Err(error))),
            Ok(iterator) => Box::new(iterator.map(move |entry| {
                let (_, bytes) = entry?;
                let decoded = crate::canonical::decode(fields, bytes, self.work)?;
                Ok(decoded.values.into_boxed_slice())
            })),
        }
    }
}

impl CandidateJudge for SchemaJudge<'_> {
    type Rejection = Box<[JudgedViolation]>;

    fn judge(
        &self,
        candidate: &CandidateState<'_, '_>,
        work: &WorkContext,
    ) -> StoreResult<Judgment<Self::Rejection>> {
        let view = CandidateView {
            state: candidate,
            schema: self.schema,
            work,
        };
        match judge_final_state(self.schema, &view, work, self.budget) {
            Ok(SchemaJudgment::Admitted) => Ok(Judgment::Admitted),
            Ok(SchemaJudgment::Rejected(violations)) => Ok(Judgment::Rejected(violations)),
            Err(JudgeError::Work(error)) => Err(StoreError::Work(error)),
            Err(JudgeError::State(error)) => Err(error),
            Err(JudgeError::UndefinedDuration { statement }) => Err(StoreError::JudgeRefused {
                statement,
                detail: "undefined ray duration in a measured position",
            }),
            Err(JudgeError::MeasureOverflow { statement }) => Err(StoreError::JudgeRefused {
                statement,
                detail: "grouped measure exceeded the widened accumulator",
            }),
        }
    }
}

/// Reference indexing: no acceleration entries. Complete judgment scans
/// the affected relations; the determinant namespace stays empty until a
/// schema-derived indexer proves equivalence (recorded C03/C04 follow-up).
#[derive(Debug, Clone, Copy)]
pub struct UnindexedRows;

impl RowIndexer for UnindexedRows {
    fn index_row(
        &self,
        _relation: RelationId,
        _row: &[u8],
        _work: &WorkContext,
        _emit: &mut dyn FnMut(StatementId, &[u8]) -> StoreResult<()>,
    ) -> StoreResult<()> {
        Ok(())
    }
}
