//! Production binding of complete and incremental judgment (C4) to the
//! store candidate protocol.
//!
//! [`SchemaJudge::judge`] (the [`CandidateJudge`] entry) is **always**
//! complete final-state judgment. An empty `ChangeSet` cannot skip
//! standing laws. Incremental judgment is a separate method that requires
//! a [`LawfulParent`] minted by checked create/open or a prior admitted
//! commit — an [`UnreadyStore`](super::staging::UnreadyStore) cannot
//! supply it.
//!
//! Charged [`crate::canonical::DecodedRow`] values are borrowed for the
//! visit; this bridge does not extract owning boxes.

use bumbledb_theory::schema::RelationId;

use super::candidate::{CandidateJudge, CandidateState, Judgment, RowIndexer};
use super::error::{StoreCorruption, StoreError, StoreResult};
use crate::Value;
use crate::changes::ChangeKind;
use crate::schema::compiled::CompiledProjection;
use crate::schema::{ProjectionId, Schema, StatementId};
use crate::schema::judge::{
    CandidateFacts, DeltaFacts, DeltaShape, JudgeBudget, JudgeError, JudgedViolation, JudgeScratch,
    Judgment as SchemaJudgment, LawfulParent, judge_incremental, store_fault,
};
use crate::work::WorkContext;

/// The production C4 judge over a store candidate.
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

    /// Complete judgment over the candidate's populated final state.
    /// Staging, restore, migrate and the verifier call this entry.
    ///
    /// # Errors
    /// Store, work, or judge-refused failures — never a forged admission.
    pub fn judge_complete(
        &self,
        candidate: &CandidateState<'_, '_>,
        work: &WorkContext,
    ) -> StoreResult<Judgment<Box<[JudgedViolation]>>> {
        let view = CandidateView::new(candidate, self.schema, work);
        map_judged(crate::schema::judge::judge_final_state_with_scratch(
            self.schema,
            &view,
            work,
            self.budget,
            JudgeScratch::channel(store_fault),
        ))
    }

    /// Incremental judgment. Requires a lawful parent; empty delta is not
    /// complete validation.
    ///
    /// # Errors
    /// As [`Self::judge_complete`].
    pub fn judge_incremental(
        &self,
        parent: LawfulParent,
        candidate: &CandidateState<'_, '_>,
        work: &WorkContext,
    ) -> StoreResult<Judgment<Box<[JudgedViolation]>>> {
        let view = CandidateView::new(candidate, self.schema, work);
        map_judged(judge_incremental(
            parent,
            self.schema,
            &view,
            work,
            self.budget,
            JudgeScratch::channel(store_fault),
        ))
    }
}

fn map_judged(
    judged: Result<SchemaJudgment, JudgeError<StoreError>>,
) -> StoreResult<Judgment<Box<[JudgedViolation]>>> {
    match judged {
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
        Err(JudgeError::Compile(error)) => Err(StoreError::Compile(error)),
    }
}

struct CandidateView<'v, 'a, 'store> {
    state: &'v CandidateState<'a, 'store>,
    schema: &'v Schema,
    work: &'v WorkContext,
    delta: Vec<(RelationId, DeltaShape)>,
}

impl<'v, 'a, 'store> CandidateView<'v, 'a, 'store> {
    fn new(
        state: &'v CandidateState<'a, 'store>,
        schema: &'v Schema,
        work: &'v WorkContext,
    ) -> Self {
        let mut delta: Vec<(RelationId, DeltaShape)> = Vec::new();
        if let Some(changes) = state.changes() {
            for record in changes.records() {
                let shape = match delta.binary_search_by_key(&record.relation, |&(id, _)| id) {
                    Ok(at) => &mut delta[at].1,
                    Err(at) => {
                        delta.insert(at, (record.relation, DeltaShape::default()));
                        &mut delta[at].1
                    }
                };
                match record.kind {
                    ChangeKind::Add => shape.adds = true,
                    ChangeKind::Remove => shape.removes = true,
                }
            }
        }
        Self {
            state,
            schema,
            work,
            delta,
        }
    }

    fn decode_visit(
        &self,
        relation: RelationId,
        bytes: &[u8],
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StoreError>,
    ) -> StoreResult<bool> {
        let Some(view) = self.schema.relation_checked(relation) else {
            return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
                "judged relation unknown to schema",
            )));
        };
        let decoded = crate::canonical::decode(view.fields(), bytes, self.work)?;
        visit(decoded.values())
    }

    fn visit_change_kind(
        &self,
        relation: RelationId,
        kind: ChangeKind,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StoreError>,
    ) -> StoreResult<()> {
        let Some(changes) = self.state.changes() else {
            return Ok(());
        };
        for record in changes.records() {
            if record.relation == relation && record.kind == kind {
                if !self.decode_visit(relation, record.row, visit)? {
                    break;
                }
            }
        }
        Ok(())
    }
}

impl CandidateFacts for CandidateView<'_, '_, '_> {
    type Error = StoreError;

    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        for entry in self.state.rows(relation)? {
            let (_, bytes) = entry?;
            if !self.decode_visit(relation, bytes, visit)? {
                break;
            }
        }
        Ok(())
    }
}

impl DeltaFacts for CandidateView<'_, '_, '_> {
    fn delta_shape(&self, relation: RelationId) -> DeltaShape {
        self.delta
            .binary_search_by_key(&relation, |&(id, _)| id)
            .map_or_else(|_| DeltaShape::default(), |at| self.delta[at].1)
    }

    fn visit_added_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StoreError>,
    ) -> Result<(), StoreError> {
        self.visit_change_kind(relation, ChangeKind::Add, visit)
    }

    fn visit_removed_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StoreError>,
    ) -> Result<(), StoreError> {
        self.visit_change_kind(relation, ChangeKind::Remove, visit)
    }

    fn visit_key_competitors(
        &self,
        statement: StatementId,
        determinant: &[Value],
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StoreError>,
    ) -> Result<Option<()>, StoreError> {
        let theory = self.schema.compiled_theory().map_err(StoreError::Compile)?;
        let Some(compiled) = theory.projection_of_statement(statement) else {
            return Ok(None);
        };
        visit_compiled_bucket(self, compiled, determinant, visit)
    }

    fn visit_compiled_group(
        &self,
        projection: &CompiledProjection,
        determinant: &[Value],
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StoreError>,
    ) -> Result<Option<()>, StoreError> {
        visit_compiled_bucket(self, projection, determinant, visit)
    }
}

fn visit_compiled_bucket(
    view: &CandidateView<'_, '_, '_>,
    compiled: &CompiledProjection,
    determinant: &[Value],
    visit: &mut dyn FnMut(&[Value]) -> Result<bool, StoreError>,
) -> Result<Option<()>, StoreError> {
    let projected = super::det_index::determinant_bytes(compiled, determinant, view.work)?;
    let fields = view.schema.relation(compiled.relation).fields();
    view.state.visit_determinant_bucket(
        compiled.id,
        &projected,
        view.work,
        &mut |_, bytes| {
            let decoded = crate::canonical::decode(fields, bytes, view.work)?;
            if compiled.scalar_values(decoded.values()).as_slice() == determinant {
                visit(decoded.values())
            } else {
                Ok(true)
            }
        },
    )?;
    Ok(Some(()))
}

impl CandidateJudge for SchemaJudge<'_> {
    type Rejection = Box<[JudgedViolation]>;

    /// Always complete. Empty or present deltas cannot select incremental
    /// judgment; that entry requires [`LawfulParent`].
    fn judge(
        &self,
        candidate: &CandidateState<'_, '_>,
        work: &WorkContext,
    ) -> StoreResult<Judgment<Self::Rejection>> {
        self.judge_complete(candidate, work)
    }
}

/// Declares no auxiliary determinant entries. **Not** "no index": the
/// store derives and maintains every interned projection's determinant
/// entries itself (`det_index`, compiled at open) inside the same
/// transaction as each row mutation. This indexer exists for auxiliary
/// projections a caller wants bucketed beyond the schema's indexes; it
/// must never reuse a [`ProjectionId`] with a different byte convention.
#[derive(Debug, Clone, Copy)]
pub struct UnindexedRows;

impl RowIndexer for UnindexedRows {
    fn index_row(
        &self,
        _relation: RelationId,
        _row: &[u8],
        _work: &WorkContext,
        _emit: &mut dyn FnMut(ProjectionId, &[u8], Option<&[u8]>) -> StoreResult<()>,
    ) -> StoreResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{SchemaJudge, UnindexedRows};
    use crate::schema::judge::LawfulParent;
    use crate::schema::{
        FieldDescriptor, FieldId, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
        StatementDescriptor, ValidateDescriptor as _, ValueType,
    };
    use crate::storage::store::candidate::{Judgment, Prepared};
    use crate::storage::store::error::StoreError;
    use crate::storage::store::map::MapPolicy;
    use crate::storage::store::staging::UnreadyStore;
    use crate::storage::store::store_env::Store;
    use crate::work::ExecutionPolicy;
    use crate::{ChangeSet, Value};
    use std::time::Duration;

    fn work() -> crate::WorkContext {
        ExecutionPolicy {
            input_bytes: 1 << 20,
            working_bytes: 1 << 20,
            scratch_bytes: 1 << 20,
            result_bytes: 0,
            rows: 1 << 16,
            work_units: 1 << 20,
            timeout: Duration::from_secs(30),
        }
        .start()
        .expect("work")
    }

    fn keyed_email() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "User".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "email".into(),
                        value_type: ValueType::String,
                    },
                ],
            }],
            statements: vec![StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::from([FieldId(1)]),
            }],
        }
        .validate()
        .expect("keyed")
    }

    fn user(id: u64, email: &str) -> Vec<Value> {
        vec![Value::U64(id), Value::String(email.into())]
    }

    fn changes(schema: &Schema, rows: &[Vec<Value>]) -> ChangeSet {
        let mut builder = ChangeSet::builder(schema, work());
        for row in rows {
            builder.insert(RelationId(0), row).expect("insert");
        }
        builder.finish().expect("sealed")
    }

    fn empty_delta(schema: &Schema) -> ChangeSet {
        ChangeSet::builder(schema, work()).finish().expect("empty")
    }

    fn temp_dest(tag: &str) -> (crate::testutil::TempDir, std::path::PathBuf) {
        let dir = crate::testutil::TempDir::new(tag);
        let dest = dir.path().join("store");
        let _ = std::fs::create_dir_all(dir.path());
        (dir, dest)
    }

    /// D26 consumer: complete vs incremental on the same populated-invalid
    /// empty delta. Incremental-with-parent admits; complete rejects.
    #[test]
    fn d26_complete_rejects_empty_delta_that_incremental_parent_would_admit() {
        let schema = keyed_email();
        let work = work();
        let (_dir, dest) = temp_dest("l02-d26-complete-vs-inc");
        let (store, _) = Store::create(&dest, &schema, MapPolicy::default()).expect("create");
        let first = changes(&schema, &[user(1, "dup@ex")]);
        let second = changes(&schema, &[user(2, "dup@ex")]);
        {
            let mut owner = store.writer(&work).expect("writer");
            owner.ingest(&first, &UnindexedRows).expect("ingest first");
        }
        {
            let mut owner = store.writer(&work).expect("writer");
            owner.ingest(&second, &UnindexedRows).expect("ingest second");
        }

        let complete = store
            .judge_populated(&schema, &work)
            .expect("complete populated");
        assert!(
            matches!(complete, Judgment::Rejected(_)),
            "CandidateJudge/complete must convict standing key conflicts"
        );

        let mut owner = store.writer(&work).expect("writer");
        let incremental = owner
            .prepare_incremental(
                LawfulParent::established(),
                &empty_delta(&schema),
                &UnindexedRows,
                &SchemaJudge::new(&schema),
            )
            .expect("incremental empty");
        assert!(
            matches!(incremental, Prepared::Admitted(_)),
            "empty-delta incremental under a parent skips standing facts — staging must not call it"
        );
    }

    /// D26 consumer: UnreadyStore::admit uses complete judgment and cannot
    /// mint [`LawfulParent`]. A populated conflict with no further delta
    /// rejects; destination stays absent.
    #[test]
    fn d26_unready_admit_cannot_mint_parent_and_must_reject() {
        let schema = keyed_email();
        let work = work();
        let (_dir, dest) = temp_dest("l02-d26-unready-admit");
        let unready =
            UnreadyStore::begin(&dest, &schema, MapPolicy::default(), &work).expect("begin");
        let first = changes(&schema, &[user(1, "dup@ex")]);
        let second = changes(&schema, &[user(2, "dup@ex")]);
        unready
            .populate(&work, |stage, work| {
                stage.apply(&first, work)?;
                stage.apply(&second, work)?;
                Ok(())
            })
            .expect("populate");
        let error = unready.admit(&schema, &work).expect_err("admit must reject");
        assert!(
            matches!(error, StoreError::JudgeRefused { .. }),
            "unready admit is complete judgment, got {error:?}"
        );
        assert!(!dest.exists(), "rejected admit leaves destination absent");
    }
}
