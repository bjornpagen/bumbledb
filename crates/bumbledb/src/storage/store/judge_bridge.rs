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
use crate::canonical::DecodeScratch;
use crate::changes::ChangeKind;
use crate::schema::compiled::CompiledProjection;
use crate::schema::judge::{
    CandidateFacts, DeltaFacts, DeltaShape, JudgeBudget, JudgeError, JudgeScratch, JudgedViolation,
    Judgment as SchemaJudgment, LawfulParent, RankedRowVisitor, judge_incremental, store_fault,
};
use crate::schema::{ProjectionId, Schema, StatementId};
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
        let view = CandidateView::incremental(candidate, self.schema, work, parent);
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
        Err(JudgeError::Allocation) => Err(StoreError::Allocation),
        Err(JudgeError::UndefinedDuration { statement }) => {
            Err(StoreError::UndefinedDuration { statement })
        }
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
    lawful_parent: bool,
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
            lawful_parent: false,
        }
    }

    fn incremental(
        state: &'v CandidateState<'a, 'store>,
        schema: &'v Schema,
        work: &'v WorkContext,
        _parent: LawfulParent,
    ) -> Self {
        let mut view = Self::new(state, schema, work);
        view.lawful_parent = true;
        view
    }

    fn decode_visit(
        &self,
        relation: RelationId,
        bytes: &[u8],
        decode: &mut DecodeScratch<'_>,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StoreError>,
    ) -> StoreResult<bool> {
        let Some(view) = self.schema.relation_checked(relation) else {
            return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
                "judged relation unknown to schema",
            )));
        };
        decode.with_decoded(view.fields(), bytes, visit)
    }

    fn visit_change_kind(
        &self,
        relation: RelationId,
        kind: ChangeKind,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StoreError>,
    ) -> StoreResult<()> {
        let shape = self.delta_shape(relation);
        if match kind {
            ChangeKind::Add => !shape.adds,
            ChangeKind::Remove => !shape.removes,
        } {
            return Ok(());
        }
        let Some(changes) = self.state.changes() else {
            return Ok(());
        };
        // This visit owns its workspace, not CandidateView. A callback may
        // re-enter another stream without aliasing its live borrowed row.
        let mut decode = DecodeScratch::new(self.work);
        for record in changes.records() {
            // ChangeSet sealing guarantees relation/full-row order.
            if record.relation > relation {
                break;
            }
            if record.relation == relation
                && record.kind == kind
                && !self.decode_visit(relation, record.row, &mut decode, visit)?
            {
                break;
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
        self.visit_ranked_rows(relation, &mut |_, row| visit(row))
    }

    fn visit_ranked_rows(
        &self,
        relation: RelationId,
        visit: RankedRowVisitor<'_, Self::Error>,
    ) -> Result<(), Self::Error> {
        let mut decode = DecodeScratch::new(self.work);
        for entry in self.state.rows(relation)? {
            let (id, bytes) = entry?;
            if !self.decode_visit(relation, bytes, &mut decode, &mut |row| visit(id.id.0, row))? {
                break;
            }
        }
        Ok(())
    }
}

impl DeltaFacts for CandidateView<'_, '_, '_> {
    fn scalar_key_preserved(&self, statement: StatementId) -> bool {
        self.lawful_parent && self.state.preserves_home_key(self.schema, statement)
    }

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
        visit_compiled_bucket(self, compiled, determinant, &mut |_, row| visit(row))
    }

    fn visit_compiled_group(
        &self,
        projection: &CompiledProjection,
        determinant: &[Value],
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, StoreError>,
    ) -> Result<Option<()>, StoreError> {
        visit_compiled_bucket(self, projection, determinant, &mut |_, row| visit(row))
    }

    fn visit_ranked_compiled_group(
        &self,
        projection: &CompiledProjection,
        determinant: &[Value],
        visit: RankedRowVisitor<'_, Self::Error>,
    ) -> Result<Option<()>, Self::Error> {
        visit_compiled_bucket(self, projection, determinant, visit)
    }
}

fn visit_compiled_bucket(
    view: &CandidateView<'_, '_, '_>,
    compiled: &CompiledProjection,
    determinant: &[Value],
    visit: RankedRowVisitor<'_, StoreError>,
) -> Result<Option<()>, StoreError> {
    let projected = super::det_index::determinant_bytes(compiled, determinant, view.work)?;
    let fields = view.schema.relation(compiled.relation).fields();
    // The bucket visit, not each row, owns decode capacity. Nested visits
    // allocate their own workspace and cannot overwrite this borrowed row.
    let mut decode = DecodeScratch::new(view.work);
    view.state
        .visit_determinant_bucket(compiled.id, &projected, view.work, &mut |id, bytes| {
            decode.with_decoded(fields, bytes, |values| {
                if determinant.len() == compiled.scalar_positions.len()
                    && compiled.scalar_positions.iter().zip(determinant).all(
                        |(&position, expected)| {
                            &values[usize::from(compiled.projection[position].0)] == expected
                        },
                    )
                {
                    visit(id.0, values)
                } else {
                    Ok(true)
                }
            })
        })?;
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
        _emit: &mut dyn FnMut(ProjectionId, &[u8]) -> StoreResult<()>,
    ) -> StoreResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CandidateView, SchemaJudge, UnindexedRows};
    use crate::schema::judge::{CandidateFacts, DeltaFacts, LawfulParent};
    use crate::schema::{
        FieldDescriptor, FieldId, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
        StatementDescriptor, ValidateDescriptor as _, ValueType,
    };
    use crate::storage::store::candidate::{Judgment, Prepared};
    use crate::storage::store::error::StoreError;
    use crate::storage::store::map::MapPolicy;
    use crate::storage::store::staging::UnreadyStore;
    use crate::storage::store::store_env::Store;
    use crate::work::WorkContext;
    use crate::{ChangeSet, Value};

    fn work() -> crate::WorkContext {
        WorkContext::new()
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

    #[test]
    fn ranked_store_visits_preserve_row_ids_through_collisions_deletion_and_early_stop() {
        let schema = keyed_email();
        let work = work();
        let (_dir, dest) = temp_dest("ranked-judge-visits");
        let store = Store::create_forced_fingerprint(
            &dest,
            &schema,
            MapPolicy::default(),
            [7; crate::storage::store::FP_LEN],
        )
        .unwrap();
        let initial = changes(
            &schema,
            &[
                user(0, "remove@ex"),
                user(1, "dup@ex"),
                user(2, "other@ex"),
                user(3, "dup@ex"),
            ],
        );
        store
            .writer(&work)
            .unwrap()
            .ingest(&initial, &UnindexedRows)
            .unwrap();
        let mut removal = ChangeSet::builder(&schema, work.clone());
        removal
            .delete(RelationId(0), &user(0, "remove@ex"))
            .unwrap();
        store
            .writer(&work)
            .unwrap()
            .ingest(&removal.finish().unwrap(), &UnindexedRows)
            .unwrap();
        let _owner = store.writer(&work).unwrap();
        let txn = store.gated_write_txn(&work).unwrap();
        let candidate = super::CandidateState::of_committed(&store, &txn);
        let view = CandidateView::new(&candidate, &schema, &work);
        let raw_ids: Vec<_> = candidate
            .rows(RelationId(0))
            .unwrap()
            .map(|entry| entry.unwrap().0.id.0)
            .collect();
        let mut complete = Vec::new();
        view.visit_ranked_rows(RelationId(0), &mut |rank, row| {
            complete.push((rank, row.to_vec()));
            Ok(true)
        })
        .unwrap();
        assert_eq!(
            complete.iter().map(|(rank, _)| *rank).collect::<Vec<_>>(),
            raw_ids
        );
        assert_eq!(complete.len(), 3);
        let compiled = schema
            .compiled_theory()
            .unwrap()
            .projection(crate::schema::ProjectionId(0))
            .unwrap();
        let determinant = [Value::String("dup@ex".into())];
        let expected: Vec<_> = complete
            .iter()
            .filter(|(_, row)| row[1] == determinant[0])
            .cloned()
            .collect();
        assert_eq!(expected.len(), 2);
        let mut indexed = Vec::new();
        assert_eq!(
            view.visit_ranked_compiled_group(compiled, &determinant, &mut |rank, row| {
                indexed.push((rank, row.to_vec()));
                Ok(true)
            })
            .unwrap(),
            Some(())
        );
        assert_eq!(
            indexed, expected,
            "bucket rank is the retained full-state row identity"
        );
        assert_ranked_visit_stopping(&view, compiled, &determinant, expected[0].0);
        assert_ranked_visit_reentrant(&view, compiled, &determinant, &expected);
    }

    fn assert_ranked_visit_reentrant(
        view: &CandidateView<'_, '_, '_>,
        compiled: &crate::schema::compiled::CompiledProjection,
        determinant: &[Value],
        expected: &[(u64, Vec<Value>)],
    ) {
        let before = crate::alloc_counter::snapshot().absolute.live_bytes;
        let mut outer = 0;
        view.visit_ranked_compiled_group(compiled, determinant, &mut |rank, row| {
            let retained = row.to_vec();
            let mut nested = Vec::new();
            view.visit_ranked_compiled_group(compiled, determinant, &mut |inner_rank, inner| {
                nested.push((inner_rank, inner.to_vec()));
                Ok(true)
            })?;
            assert_eq!(nested, expected);
            assert_eq!(
                row, retained,
                "nested decode cannot reuse a live outer workspace"
            );
            assert_eq!((rank, retained), expected[outer]);
            outer += 1;
            Ok(true)
        })
        .unwrap();
        assert_eq!(outer, expected.len());
        #[cfg(feature = "alloc-counter")]
        assert_eq!(crate::alloc_counter::snapshot().absolute.live_bytes, before);
        let _ = before;

        let cancelled = work();
        let cancel_view = CandidateView::new(view.state, view.schema, &cancelled);
        let before_cancel = crate::alloc_counter::snapshot().absolute.live_bytes;
        let mut calls = 0;
        let result = cancel_view.visit_ranked_compiled_group(compiled, determinant, &mut |_, _| {
            calls += 1;
            cancelled.cancel();
            Ok(true)
        });
        assert_eq!(result, Err(StoreError::Work(crate::WorkError::Cancelled)));
        assert_eq!(calls, 1, "reused decode cannot bypass cancellation");
        #[cfg(feature = "alloc-counter")]
        assert_eq!(
            crate::alloc_counter::snapshot().absolute.live_bytes,
            before_cancel
        );
        let _ = before_cancel;
    }

    fn assert_ranked_visit_stopping(
        view: &CandidateView<'_, '_, '_>,
        compiled: &crate::schema::compiled::CompiledProjection,
        determinant: &[Value],
        first_rank: u64,
    ) {
        let before = crate::alloc_counter::snapshot().absolute.live_bytes;
        let mut stopped = 0;
        view.visit_ranked_compiled_group(compiled, determinant, &mut |rank, _| {
            assert_eq!(rank, first_rank);
            stopped += 1;
            Ok(false)
        })
        .unwrap();
        assert_eq!(stopped, 1);
        stopped = 0;
        view.visit_ranked_rows(RelationId(0), &mut |_, _| {
            stopped += 1;
            Ok(false)
        })
        .unwrap();
        assert_eq!(stopped, 1);
        let error = StoreError::ForeignSchema;
        assert_eq!(
            view.visit_ranked_rows(RelationId(0), &mut |_, _| Err(error.clone())),
            Err(error.clone())
        );
        assert_eq!(
            view.visit_ranked_compiled_group(compiled, determinant, &mut |_, _| Err(error.clone())),
            Err(error)
        );
        #[cfg(feature = "alloc-counter")]
        assert_eq!(crate::alloc_counter::snapshot().absolute.live_bytes, before);
        let _ = before;
    }

    struct StreamDecodeJudge<'a> {
        schema: &'a Schema,
        old: &'a [Value],
        added: &'a [Vec<Value>],
    }
    impl super::CandidateJudge for StreamDecodeJudge<'_> {
        type Rejection = ();
        fn judge(
            &self,
            candidate: &super::CandidateState<'_, '_>,
            _: &crate::WorkContext,
        ) -> Result<Judgment<()>, StoreError> {
            let context = work();
            let view = CandidateView::new(candidate, self.schema, &context);
            let mut seen = Vec::new();
            view.visit_added_rows(RelationId(0), &mut |row| {
                let retained = row.to_vec();
                let mut nested = 0;
                view.visit_ranked_rows(RelationId(1), &mut |_, narrow| {
                    assert_eq!(narrow, &[Value::U64(9)]);
                    nested += 1;
                    Ok(true)
                })?;
                assert_eq!(nested, 1);
                assert_eq!(
                    row,
                    retained.as_slice(),
                    "nested different-arity stream cannot overwrite its caller's row"
                );
                seen.push(retained);
                Ok(true)
            })?;
            assert_eq!(seen.as_slice(), self.added);
            let mut removed = 0;
            view.visit_removed_rows(RelationId(0), &mut |row| {
                assert_eq!(row, self.old);
                removed += 1;
                Ok(true)
            })?;
            assert_eq!(removed, 1);
            let mut narrow_added = 0;
            view.visit_added_rows(RelationId(1), &mut |row| {
                assert_eq!(row, &[Value::U64(9)]);
                narrow_added += 1;
                Ok(true)
            })?;
            assert_eq!(narrow_added, 1);
            view.visit_removed_rows(RelationId(1), &mut |_| {
                panic!("relation-one Add must never enter a Remove stream")
            })?;
            for relation in [RelationId(2), RelationId(u32::MAX)] {
                view.visit_added_rows(relation, &mut |_| panic!("absent relation Add"))?;
                view.visit_removed_rows(relation, &mut |_| panic!("absent relation Remove"))?;
            }
            let mut complete = Vec::new();
            view.visit_ranked_rows(RelationId(0), &mut |_, row| {
                complete.push(row.to_vec());
                Ok(true)
            })?;
            assert_eq!(complete.len(), seen.len());
            assert!(complete.iter().all(|row| seen.contains(row)));

            assert_delta_visit_exits(&view, &context)?;
            Ok(Judgment::Rejected(())) // The test candidate is never published.
        }
    }

    fn assert_delta_visit_exits(
        view: &CandidateView<'_, '_, '_>,
        context: &crate::WorkContext,
    ) -> Result<(), StoreError> {
        let before = crate::alloc_counter::snapshot().absolute.live_bytes;
        let mut stopped = 0;
        view.visit_added_rows(RelationId(0), &mut |_| {
            stopped += 1;
            Ok(false)
        })?;
        assert_eq!(stopped, 1);
        #[cfg(feature = "alloc-counter")]
        assert_eq!(crate::alloc_counter::snapshot().absolute.live_bytes, before);
        assert_eq!(
            view.visit_added_rows(RelationId(0), &mut |_| Err(StoreError::ForeignSchema)),
            Err(StoreError::ForeignSchema)
        );
        #[cfg(feature = "alloc-counter")]
        assert_eq!(crate::alloc_counter::snapshot().absolute.live_bytes, before);
        let mut cancelled = 0;
        let result = view.visit_added_rows(RelationId(0), &mut |_| {
            cancelled += 1;
            context.cancel();
            Ok(true)
        });
        assert_eq!(
            result,
            Err(StoreError::Changes(crate::changes::ChangeError::Row(
                crate::canonical::RowError::Work(crate::WorkError::Cancelled)
            )))
        );
        assert_eq!(
            cancelled, 1,
            "reused capacity must still check before the next callback"
        );
        #[cfg(feature = "alloc-counter")]
        assert_eq!(crate::alloc_counter::snapshot().absolute.live_bytes, before);
        let _ = before;
        Ok(())
    }

    #[test]
    fn delta_decode_streams_are_reentrant_and_release_scratch_on_every_exit() {
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Wide".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "id".into(),
                            value_type: ValueType::U64,
                        },
                        FieldDescriptor {
                            name: "text".into(),
                            value_type: ValueType::String,
                        },
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Narrow".into(),
                    fields: vec![FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    }],
                },
            ],
            statements: vec![],
        }
        .validate()
        .unwrap();
        let old = user(0, "removed");
        let added = vec![user(1, &"long".repeat(2048)), user(2, "short")];
        let (_dir, path) = temp_dest("judge-stream-decode");
        let (store, _) = Store::create(&path, &schema, MapPolicy::default()).unwrap();
        store
            .writer(&work())
            .unwrap()
            .ingest(
                &changes(&schema, std::slice::from_ref(&old)),
                &UnindexedRows,
            )
            .unwrap();
        let mut builder = ChangeSet::builder(&schema, work());
        builder.delete(RelationId(0), &old).unwrap();
        // Deliberately submit relation one before later relation-zero rows.
        // Both builder sealing and the import parser must canonicalize/check
        // the relation ordering consumed by the delta prefix shortcut.
        builder.insert(RelationId(1), &[Value::U64(9)]).unwrap();
        for row in &added {
            builder.insert(RelationId(0), row).unwrap();
        }
        let delta = builder.finish().unwrap();
        let delta = ChangeSet::parse(&schema, delta.as_bytes(), &work()).unwrap();

        let context = work();
        let mut writer = store.writer(&context).unwrap();
        assert!(matches!(
            writer
                .prepare(
                    &delta,
                    &UnindexedRows,
                    &StreamDecodeJudge {
                        schema: &schema,
                        old: &old,
                        added: &added
                    }
                )
                .unwrap(),
            Prepared::Rejected { rejection: (), .. }
        ));
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
            owner
                .ingest(&second, &UnindexedRows)
                .expect("ingest second");
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

    /// D26 consumer: `UnreadyStore::admit` uses complete judgment and cannot
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
        let error = unready
            .admit(&schema, &work)
            .err()
            .expect("admit must reject");
        assert!(
            matches!(error, StoreError::JudgeRefused { .. }),
            "unready admit is complete judgment, got {error:?}"
        );
        assert!(!dest.exists(), "rejected admit leaves destination absent");
    }

    struct CountedKeys<S> {
        inner: S,
        additions: std::cell::Cell<usize>,
        probes: std::cell::Cell<usize>,
    }

    impl<S: DeltaFacts> CandidateFacts for CountedKeys<S> {
        type Error = S::Error;
        fn visit_rows(
            &self,
            relation: RelationId,
            visit: crate::schema::judge::RowVisitor<'_, Self::Error>,
        ) -> Result<(), Self::Error> {
            self.inner.visit_rows(relation, visit)
        }
        fn visit_ranked_rows(
            &self,
            relation: RelationId,
            visit: crate::schema::judge::RankedRowVisitor<'_, Self::Error>,
        ) -> Result<(), Self::Error> {
            self.inner.visit_ranked_rows(relation, visit)
        }
    }

    impl<S: DeltaFacts> DeltaFacts for CountedKeys<S> {
        fn scalar_key_preserved(&self, statement: crate::schema::StatementId) -> bool {
            self.inner.scalar_key_preserved(statement)
        }
        fn delta_shape(&self, relation: RelationId) -> crate::schema::judge::DeltaShape {
            self.inner.delta_shape(relation)
        }
        fn visit_added_rows(
            &self,
            relation: RelationId,
            visit: crate::schema::judge::RowVisitor<'_, Self::Error>,
        ) -> Result<(), Self::Error> {
            self.additions.set(self.additions.get() + 1);
            self.inner.visit_added_rows(relation, visit)
        }
        fn visit_removed_rows(
            &self,
            relation: RelationId,
            visit: crate::schema::judge::RowVisitor<'_, Self::Error>,
        ) -> Result<(), Self::Error> {
            self.inner.visit_removed_rows(relation, visit)
        }
        fn visit_key_competitors(
            &self,
            statement: crate::schema::StatementId,
            determinant: &[Value],
            visit: crate::schema::judge::RowVisitor<'_, Self::Error>,
        ) -> Result<Option<()>, Self::Error> {
            self.probes.set(self.probes.get() + 1);
            self.inner
                .visit_key_competitors(statement, determinant, visit)
        }
    }

    struct CheckHome<'s> {
        schema: &'s Schema,
        preserved: bool,
        calls: std::cell::Cell<usize>,
    }

    impl crate::storage::store::CandidateJudge for CheckHome<'_> {
        type Rejection = Box<[crate::schema::judge::JudgedViolation]>;

        fn judge(
            &self,
            candidate: &crate::storage::store::CandidateState<'_, '_>,
            work: &crate::WorkContext,
        ) -> Result<Judgment<Self::Rejection>, StoreError> {
            use crate::schema::judge::{JudgeScratch, Judgment as SemanticJudgment};
            let statement = crate::schema::StatementId(0);
            self.calls.set(self.calls.get() + 1);
            assert_eq!(
                candidate.preserves_home_key(self.schema, statement),
                self.preserved
            );
            assert!(
                !candidate
                    .preserves_home_key(&crate::storage::store::tests::other_schema(), statement)
            );
            let parent = LawfulParent::established();
            let counted = CountedKeys {
                inner: CandidateView::incremental(candidate, self.schema, work, parent),
                additions: std::cell::Cell::new(0),
                probes: std::cell::Cell::new(0),
            };
            let incremental = crate::schema::judge::judge_incremental(
                parent,
                self.schema,
                &counted,
                work,
                crate::schema::judge::JudgeBudget::default(),
                JudgeScratch::channel(crate::schema::judge::store_fault),
            )
            .unwrap();
            let complete_view = CandidateView::new(candidate, self.schema, work);
            assert!(!complete_view.scalar_key_preserved(statement));
            let complete = crate::schema::judge::judge_final_state_with_scratch(
                self.schema,
                // Even an incremental view advertising true preservation
                // cannot change complete judgment's interpretation.
                &counted,
                work,
                crate::schema::judge::JudgeBudget::default(),
                JudgeScratch::channel(crate::schema::judge::store_fault),
            )
            .unwrap();
            assert_eq!(
                incremental, complete,
                "all violations and canonical examples"
            );
            if let SemanticJudgment::Rejected(ref violations) = incremental {
                let SemanticJudgment::Rejected(ref expected) = complete else {
                    unreachable!()
                };
                let encode = |facts: &[crate::schema::judge::JudgedViolation]| {
                    crate::schema::evidence::encode_judged(self.schema, facts, 1 << 20, work)
                        .unwrap()
                };
                assert_eq!(encode(violations), encode(expected));
            }
            if self.preserved {
                assert_eq!(counted.additions.get(), 0, "no second delta decode");
                assert_eq!(counted.probes.get(), 0, "no second selected-home seek");
            } else {
                assert!(counted.probes.get() > 0, "conflicts use the existing judge");
            }
            // Exercise the public production entry as well as the counted view.
            SchemaJudge::new(self.schema).judge_incremental(parent, candidate, work)
        }
    }

    #[test]
    fn home_preservation_skips_real_bucket_visits_but_keeps_conflict_citations() {
        use crate::storage::store::tests::{NO_HOST, NOTE, change_set, note, schema};
        let schema = schema();
        let work = work();
        let (_dir, dest) = temp_dest("home-preservation-production");
        let store = Store::create_forced_fingerprint(
            &dest,
            &schema,
            MapPolicy::default(),
            [0; crate::storage::store::FP_LEN],
        )
        .unwrap();
        let z = (NOTE, note(1, "z"));
        let cases = [
            (vec![z.clone(), (NOTE, note(2, "two"))], vec![], true),
            // Duplicate plus fresh addition; only one row is newly written.
            (vec![z.clone(), (NOTE, note(3, "three"))], vec![], true),
            // The sealed order adds a,b,z. The duplicate z hits the oldest
            // ordinal after earlier additions have already caused a conflict.
            (
                vec![(NOTE, note(1, "a")), (NOTE, note(1, "b")), z.clone()],
                vec![],
                false,
            ),
            (vec![(NOTE, note(1, "replacement"))], vec![z], true),
            // Add wins the one-command tie; no physical remove is necessary.
            (
                vec![(NOTE, note(3, "three"))],
                vec![(NOTE, note(3, "three"))],
                true,
            ),
        ];
        for (adds, removes, preserved) in cases {
            let changes = change_set(&schema, &adds, &removes);
            let judge = CheckHome {
                schema: &schema,
                preserved,
                calls: std::cell::Cell::new(0),
            };
            let mut owner = store.writer(&work).unwrap();
            match owner.prepare(&changes, &UnindexedRows, &judge).unwrap() {
                Prepared::Admitted(prepared) => {
                    assert!(preserved);
                    prepared.seal(NO_HOST).unwrap().commit().unwrap();
                }
                Prepared::Rejected { .. } => assert!(!preserved),
            }
            assert_eq!(judge.calls.get(), 1);
        }
        let snapshot = store.snapshot(&work).unwrap();
        assert_eq!(snapshot.row_count(NOTE).unwrap(), 3);
    }

    #[test]
    fn complete_staging_admission_ignores_each_ingest_preservation_hint() {
        use crate::storage::store::tests::{NOTE, change_set, note, schema};
        let schema = schema();
        let work = work();
        let (_dir, dest) = temp_dest("home-preservation-unready");
        let unready = UnreadyStore::begin(&dest, &schema, MapPolicy::default(), &work).unwrap();
        unready
            .populate(&work, |stage, work| {
                for value in ["z", "a"] {
                    stage.apply(&change_set(&schema, &[(NOTE, note(1, value))], &[]), work)?;
                }
                // A new empty ingest starts a fresh RowWriter. Its true flag is
                // only mutation preservation and cannot attest the unlawful base.
                stage.apply(&change_set(&schema, &[], &[]), work)?;
                Ok(())
            })
            .unwrap();
        assert!(matches!(
            unready.admit(&schema, &work),
            Err(StoreError::JudgeRefused { .. })
        ));
        assert!(!dest.exists());
    }

    #[test]
    fn failed_home_insert_never_reaches_judgment_or_publication() {
        use crate::storage::store::tests::{NOTE, change_set, note, schema};
        struct Fail;
        impl crate::storage::store::RowIndexer for Fail {
            fn index_row(
                &self,
                _: RelationId,
                _: &[u8],
                _: &crate::WorkContext,
                _: crate::storage::store::ProjectionEmitter<'_>,
            ) -> Result<(), StoreError> {
                Err(StoreError::Allocation)
            }
        }
        let schema = schema();
        let work = work();
        let (_dir, dest) = temp_dest("home-preservation-error");
        let store = Store::create(&dest, &schema, MapPolicy::default())
            .unwrap()
            .0;
        let changes = change_set(&schema, &[(NOTE, note(1, "new"))], &[]);
        let judge = CheckHome {
            schema: &schema,
            preserved: true,
            calls: std::cell::Cell::new(0),
        };
        let mut owner = store.writer(&work).unwrap();
        assert!(matches!(
            owner.prepare(&changes, &Fail, &judge),
            Err(StoreError::Allocation)
        ));
        assert_eq!(judge.calls.get(), 0);
        drop(owner);
        assert_eq!(store.snapshot(&work).unwrap().row_count(NOTE).unwrap(), 0);
    }

    #[test]
    fn complete_judge_ignores_true_preservation_on_an_unlawful_parent() {
        use crate::storage::store::tests::{NOTE, change_set, note, schema};
        struct CompleteOnly<'s>(&'s Schema);
        impl crate::storage::store::CandidateJudge for CompleteOnly<'_> {
            type Rejection = Box<[crate::schema::judge::JudgedViolation]>;
            fn judge(
                &self,
                candidate: &crate::storage::store::CandidateState<'_, '_>,
                work: &crate::WorkContext,
            ) -> Result<Judgment<Self::Rejection>, StoreError> {
                assert!(candidate.preserves_home_key(self.0, crate::schema::StatementId(0)));
                SchemaJudge::new(self.0).judge_complete(candidate, work)
            }
        }
        let schema = schema();
        let work = work();
        let (_dir, dest) = temp_dest("home-preservation-complete");
        let store = Store::create(&dest, &schema, MapPolicy::default())
            .unwrap()
            .0;
        let bad = change_set(&schema, &[(NOTE, note(1, "a")), (NOTE, note(1, "z"))], &[]);
        store
            .writer(&work)
            .unwrap()
            .ingest(&bad, &UnindexedRows)
            .unwrap();
        let empty = change_set(&schema, &[], &[]);
        let mut owner = store.writer(&work).unwrap();
        assert!(matches!(
            owner
                .prepare(&empty, &UnindexedRows, &CompleteOnly(&schema))
                .unwrap(),
            Prepared::Rejected { .. }
        ));
    }

    #[test]
    fn cancelled_home_insert_never_becomes_an_admission() {
        use crate::storage::store::tests::{NOTE, change_set, note, schema};
        struct Cancel;
        impl crate::storage::store::RowIndexer for Cancel {
            fn index_row(
                &self,
                _: RelationId,
                _: &[u8],
                work: &crate::WorkContext,
                _: crate::storage::store::ProjectionEmitter<'_>,
            ) -> Result<(), StoreError> {
                work.cancel();
                Ok(())
            }
        }
        let schema = schema();
        let context = work();
        let (_dir, dest) = temp_dest("home-preservation-cancel");
        let store = Store::create(&dest, &schema, MapPolicy::default())
            .unwrap()
            .0;
        let changes = change_set(&schema, &[(NOTE, note(1, "new"))], &[]);
        let judge = CheckHome {
            schema: &schema,
            preserved: true,
            calls: std::cell::Cell::new(0),
        };
        let mut owner = store.writer(&context).unwrap();
        assert!(matches!(
            owner.prepare(&changes, &Cancel, &judge),
            Err(StoreError::Work(crate::WorkError::Cancelled))
        ));
        assert_eq!(judge.calls.get(), 0);
        drop(owner);
        assert_eq!(store.snapshot(&work()).unwrap().row_count(NOTE).unwrap(), 0);
    }
}
