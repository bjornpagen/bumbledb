//! Schema-derived determinant indexing — physical arm of [`CompiledTheory`]
//! (chapter 10 §4, chapter 40).
//!
//! One multimap entry per live row and interned projection:
//! `[TAG_DETERMINANT, projection id, routing bytes, optional interval tail, row id]`.
//! Routing is either compact exact scalar bytes (≤16) or a 16-byte
//! fingerprint over the canonical projected row encoding. Every consumer
//! confirms with full decoded canonical values. Candidate indexes remain
//! multimaps so conflicting tentative rows survive until judgment.

use bumbledb_theory::schema::{RelationId, StatementId};

use super::error::{StoreError, StoreResult};
use super::fingerprint::FP_LEN;
use crate::schema::compiled::{
    CompileError, CompiledProjection, CompiledTheory, DistinctnessWitness, KeyEncoding,
    ProjectionBinding, ProjectionId, ProjectionInternKey, VisitControl, VisitOutcome,
    encode_scalar_group,
};
use crate::schema::{FieldDescriptor, Schema};
use crate::work::WorkContext;

/// The store's view of the sealed schema's compiled theory — no second
/// interpretation; shares the schema's interned [`CompiledTheory`].
pub(crate) struct DeterminantTable {
    theory: std::sync::Arc<CompiledTheory>,
}

impl DeterminantTable {
    pub(crate) fn compile(schema: &Schema) -> Result<Self, CompileError> {
        Ok(Self {
            theory: schema.shared_compiled_theory()?,
        })
    }

    #[must_use]
    pub(crate) fn theory(&self) -> &CompiledTheory {
        &self.theory
    }

    pub(crate) fn fields_of(&self, relation: RelationId) -> Option<&[FieldDescriptor]> {
        self.theory.fields_of(relation)
    }

    pub(crate) fn keys_of(
        &self,
        relation: RelationId,
    ) -> impl Iterator<Item = &CompiledProjection> {
        self.theory
            .key_projections_of(relation)
            .iter()
            .filter_map(|id| self.theory.projection(*id))
    }

    pub(crate) fn projections_of(
        &self,
        relation: RelationId,
    ) -> impl Iterator<Item = &CompiledProjection> {
        self.theory
            .projections_of_relation(relation)
            .iter()
            .filter_map(|id| self.theory.projection(*id))
    }

    pub(crate) fn projection(&self, id: ProjectionId) -> Option<&CompiledProjection> {
        self.theory.projection(id)
    }

    pub(crate) fn projection_of(&self, statement: StatementId) -> Option<&CompiledProjection> {
        self.theory.projection_of_statement(statement)
    }

    pub(crate) fn source_of(&self, statement: StatementId) -> Option<&CompiledProjection> {
        self.theory.source_projection(statement)
    }

    pub(crate) fn target_of(&self, statement: StatementId) -> Option<&CompiledProjection> {
        self.theory.target_projection(statement)
    }

    pub(crate) fn source_binding(&self, statement: StatementId) -> Option<&ProjectionBinding> {
        self.theory.source_binding(statement)
    }

    pub(crate) fn target_binding(&self, statement: StatementId) -> Option<&ProjectionBinding> {
        self.theory.target_binding(statement)
    }

    pub(crate) fn key_for(
        &self,
        relation: RelationId,
        projection: &[bumbledb_theory::schema::FieldId],
    ) -> Option<&CompiledProjection> {
        self.theory.key_for(relation, projection)
    }

    /// Emit every interned physical projection of one stored row.
    /// Shared indexes emit once. Callers persist [`ProjectionId`], not a
    /// restated statement.
    ///
    /// # Errors
    /// Work exhaustion, malformed stored row, or sink failure.
    pub(crate) fn emit_row(
        &self,
        relation: RelationId,
        row: &[u8],
        work: &WorkContext,
        emit: &mut dyn FnMut(ProjectionId, &[u8], Option<&[u8]>) -> StoreResult<()>,
    ) -> StoreResult<()> {
        let projections: Vec<_> = self.theory.projections_of_relation(relation).to_vec();
        if projections.is_empty() {
            return Ok(());
        }
        let fields = self.fields_of(relation).ok_or(StoreError::ForeignSchema)?;
        let decoded = crate::canonical::decode(fields, row, work)?;
        self.emit_decoded(relation, decoded.values(), work, emit)
    }

    /// Descriptor-based visit of one decoded row's interned projections.
    /// The optional third argument is the ordered interval tail; it is not
    /// part of the 16-byte scalar grouping width.
    ///
    /// # Errors
    /// Work exhaustion or sink failure.
    pub(crate) fn emit_decoded(
        &self,
        relation: RelationId,
        values: &[crate::Value],
        work: &WorkContext,
        emit: &mut dyn FnMut(ProjectionId, &[u8], Option<&[u8]>) -> StoreResult<()>,
    ) -> StoreResult<()> {
        for id in self.theory.projections_of_relation(relation) {
            let projection = self.theory.projection(*id).expect("indexed id");
            work.step(1)?;
            let scalars = projection.scalar_values(values);
            let projected = determinant_bytes(projection, &scalars, work)?;
            let tail = projection.interval_tail_bytes(values);
            emit(*id, &projected, tail.as_deref())?;
        }
        Ok(())
    }

    /// Walk candidates under a compiled witness. Existence-only stops after
    /// the first sufficient exact witness; `Stop` and source errors halt.
    pub(crate) fn consume_visits<T, E>(
        &self,
        witness: DistinctnessWitness,
        candidates: impl IntoIterator<Item = T>,
        visit: &mut dyn FnMut(T) -> Result<VisitControl, E>,
    ) -> Result<VisitOutcome, E> {
        CompiledTheory::consume_visits(witness, candidates, visit)
    }
}

/// Projected routing bytes for one determinant group. Exact-bounded paths
/// use compact order-preserving scalar bytes; fingerprint paths use the
/// canonical row encoding of the scalar values (the ONE tagged convention for
/// hashing and exact confirmation).
/// # Errors
/// Work exhaustion or allocation failure.
pub(crate) fn determinant_bytes(
    projection: &CompiledProjection,
    values: &[crate::Value],
    work: &WorkContext,
) -> StoreResult<Vec<u8>> {
    match projection.encoding {
        KeyEncoding::ExactBounded { .. } => encode_scalar_group(values, &projection.scalar_fields)
            .ok_or(StoreError::ForeignSchema),
        KeyEncoding::FingerprintBucket => {
            let row = crate::canonical::CanonicalRow::encode(
                &projection.scalar_fields,
                values,
                work,
            )?;
            Ok(row.as_bytes().to_vec())
        }
    }
}

/// Fingerprint routing for a projected byte slice (fingerprint arm only).
pub(crate) fn fingerprint_routing(
    fingerprinter: super::fingerprint::Fingerprinter,
    projection: ProjectionId,
    projected: &[u8],
) -> [u8; FP_LEN] {
    fingerprinter.determinant(projection, projected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use crate::encoding::encode_u64;
    use crate::schema::compiled::{CompileError, KeyEncoding};
    use crate::schema::tests::{capacity, containment, fd, field, side};
    use crate::schema::{
        IntervalElement, RelationDescriptor, SchemaDescriptor, ValidateDescriptor as _, ValueType,
    };
    use crate::Interval;
    use crate::work::ExecutionPolicy;
    use bumbledb_theory::schema::FieldId;
    use std::time::Duration;

    fn work() -> WorkContext {
        ExecutionPolicy {
            input_bytes: 1_000_000,
            working_bytes: 1_000_000,
            scratch_bytes: 0,
            result_bytes: 0,
            rows: 1000,
            work_units: 1_000_000,
            timeout: Duration::from_secs(60),
        }
        .start()
        .expect("work")
    }

    fn table(schema: &Schema) -> DeterminantTable {
        DeterminantTable {
            theory: std::sync::Arc::new(
                CompiledTheory::compile(schema).expect("projection ids"),
            ),
        }
    }

    #[test]
    fn d04_emit_decoded_uses_projection_ids_and_compact_u64_bytes() {
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "T".into(),
                    fields: vec![field("id", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "S".into(),
                    fields: vec![field("id", ValueType::U64), field("parent", ValueType::U64)],
                },
            ],
            statements: vec![
                fd(RelationId(0), &[FieldId(0)]),
                fd(RelationId(1), &[FieldId(0)]),
                containment(
                    side(RelationId(1), &[FieldId(1)]),
                    side(RelationId(0), &[FieldId(0)]),
                ),
                capacity(
                    side(RelationId(1), &[FieldId(1)]),
                    0,
                    Some(4),
                    side(RelationId(0), &[FieldId(0)]),
                ),
            ],
        }
        .validate()
        .expect("valid");
        let det = table(&schema);
        let mut t_emits = Vec::new();
        det.emit_decoded(
            RelationId(0),
            &[Value::U64(9)],
            &work(),
            &mut |id, bytes, tail| {
                assert!(tail.is_none());
                t_emits.push((id, bytes.to_vec()));
                Ok(())
            },
        )
        .expect("T emit");
        assert_eq!(t_emits.len(), 1, "T key and containment/capacity target share");
        assert_eq!(t_emits[0].1, encode_u64(9));
        let intern = CompiledTheory::intern_key(det.projection(t_emits[0].0).expect("id"));
        assert_eq!(intern.encoding, KeyEncoding::ExactBounded { scalar_width: 8 });

        let mut s_emits = Vec::new();
        det.emit_decoded(
            RelationId(1),
            &[Value::U64(1), Value::U64(9)],
            &work(),
            &mut |id, bytes, tail| {
                assert!(tail.is_none());
                s_emits.push((id, bytes.to_vec()));
                Ok(())
            },
        )
        .expect("S emit");
        assert_eq!(
            s_emits.len(),
            2,
            "S key plus one shared source group for containment and capacity"
        );
        let parent = det
            .source_of(StatementId(2))
            .expect("containment source")
            .id;
        assert_eq!(
            det.source_of(StatementId(3)).expect("capacity source").id,
            parent
        );
        assert!(s_emits.iter().any(|(id, bytes)| *id == parent && bytes == &encode_u64(9)));
    }

    #[test]
    fn d04_unrelated_groups_do_not_increase_one_row_visits() {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![field("k", ValueType::U64), field("payload", ValueType::U64)],
            }],
            statements: vec![fd(RelationId(0), &[FieldId(0)])],
        }
        .validate()
        .expect("valid");
        let det = table(&schema);
        let mut visits = Vec::new();
        for n in [1u64, 8, 64] {
            let mut count = 0usize;
            for group in 0..n {
                det.emit_decoded(
                    RelationId(0),
                    &[Value::U64(group), Value::U64(group.saturating_mul(3))],
                    &work(),
                    &mut |_, _, _| {
                        count += 1;
                        Ok(())
                    },
                )
                .expect("emit");
            }
            visits.push((n, count / n as usize));
        }
        assert!(
            visits.iter().all(|&(_, per_row)| per_row == 1),
            "eligible local emit does not scan unrelated groups: {visits:?}"
        );
    }

    #[test]
    fn d04_conflicting_rows_emit_the_same_projection_twice() {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![field("k", ValueType::U64), field("payload", ValueType::U64)],
            }],
            statements: vec![fd(RelationId(0), &[FieldId(0)])],
        }
        .validate()
        .expect("valid");
        let det = table(&schema);
        let mut entries = Vec::new();
        for payload in [10u64, 11] {
            det.emit_decoded(
                RelationId(0),
                &[Value::U64(1), Value::U64(payload)],
                &work(),
                &mut |id, bytes, _tail| {
                    entries.push((id, bytes.to_vec()));
                    Ok(())
                },
            )
            .expect("emit");
        }
        assert_eq!(entries.len(), 2, "multimap keeps both tentative rows");
        assert_eq!(entries[0], entries[1]);
    }

    #[test]
    fn d04_pointwise_emit_carries_ordered_interval_tail() {
        let iv = ValueType::Interval {
            element: IntervalElement::U64,
        };
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "Booking".into(),
                fields: vec![field("room", ValueType::U64), field("during", iv)],
            }],
            statements: vec![fd(RelationId(0), &[FieldId(0), FieldId(1)])],
        }
        .validate()
        .expect("valid");
        let det = table(&schema);
        let span = Interval::new(3u64, 8).expect("span");
        let mut tails = Vec::new();
        det.emit_decoded(
            RelationId(0),
            &[Value::U64(4), Value::IntervalU64(span)],
            &work(),
            &mut |id, routing, tail| {
                assert_eq!(routing, encode_u64(4));
                tails.push((id, tail.map(<[u8]>::to_vec)));
                Ok(())
            },
        )
        .expect("emit");
        assert_eq!(tails.len(), 1);
        let tail = tails[0].1.as_ref().expect("ordered tail");
        assert_eq!(tail.len(), 16);
        assert_eq!(&tail[..8], &encode_u64(3));
        assert_eq!(&tail[8..], &encode_u64(8));
    }

    #[test]
    fn d10_existence_suffix_and_sink_stop_use_compiled_owner() {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![field("id", ValueType::U64)],
            }],
            statements: vec![fd(RelationId(0), &[FieldId(0)])],
        }
        .validate()
        .expect("valid");
        let det = table(&schema);
        let projection = det.projection_of(StatementId(0)).expect("key").id;
        let mut seen = 0usize;
        let stopped = det
            .consume_visits(
                DistinctnessWitness::ExistenceOnly { projection },
                0..32,
                &mut |_| {
                    seen += 1;
                    Ok::<_, StoreError>(VisitControl::Sufficient)
                },
            )
            .expect("existence");
        assert_eq!(stopped, VisitOutcome::Sufficient { visited: 1 });
        assert_eq!(seen, 1);

        seen = 0;
        let halt = det
            .consume_visits(
                DistinctnessWitness::ScalarKeyUnique { projection },
                0..32,
                &mut |_| {
                    seen += 1;
                    Ok::<_, StoreError>(VisitControl::Stop)
                },
            )
            .expect("stop");
        assert_eq!(halt, VisitOutcome::Stopped { visited: 1 });
        assert_eq!(seen, 1);
    }

    #[test]
    fn compile_error_is_the_explicit_exhaustion_signal() {
        assert_eq!(
            format!("{}", CompileError::ProjectionIdExhausted),
            "compiled projection id space exhausted"
        );
    }

    #[test]
    fn intern_key_is_the_shared_roster_identity() {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![field("id", ValueType::U64)],
            }],
            statements: vec![fd(RelationId(0), &[FieldId(0)])],
        }
        .validate()
        .expect("valid");
        let det = table(&schema);
        let proj = det.projection_of(StatementId(0)).expect("key");
        let key = CompiledTheory::intern_key(proj);
        assert_eq!(
            key,
            ProjectionInternKey {
                relation: RelationId(0),
                projection: Box::from([FieldId(0)]),
                encoding: KeyEncoding::ExactBounded { scalar_width: 8 },
            }
        );
    }
}
