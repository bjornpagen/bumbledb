//! Schema-derived determinant indexing: the physical arm of
//! [`CompiledTheory`]. Each relation's home projection is its narrowest
//! exact scalar key; every other interned projection keeps one multimap
//! entry per live row, so conflicting proposals coexist until judgment.
//! Routing is exact scalar bytes (at most 16) or a 16-byte fingerprint over
//! the canonical projected encoding; consumers confirm by decoded values.

use bumbledb_theory::schema::{RelationId, StatementId};

use crate::error::{Error, Result};
use crate::schema::compiled::{
    CompileError, CompiledProjection, CompiledTheory, DistinctnessWitness, KeyEncoding,
    MAX_EXACT_SCALAR_BYTES, ProjectionId, encode_scalar_group_into,
};
#[cfg(test)]
use crate::schema::compiled::{VisitControl, VisitOutcome};
use crate::schema::{FieldDescriptor, Schema};
use crate::work::WorkContext;

/// The store's view of the sealed schema's compiled theory — no second
/// interpretation; shares the schema's interned [`CompiledTheory`].
pub(crate) struct DeterminantTable {
    theory: std::sync::Arc<CompiledTheory>,
    home: Box<[Option<ProjectionId>]>,
}

/// One emitted projection of a row: the compiled projection and its
/// projected determinant bytes.
pub(crate) type Emit<'a> = &'a mut dyn FnMut(&CompiledProjection, &[u8]) -> Result<()>;

impl DeterminantTable {
    pub(crate) fn compile(schema: &Schema) -> Result<Self, CompileError> {
        let theory = schema.shared_compiled_theory()?;
        let home = (0..schema.relations().len())
            .map(|index| {
                let relation = RelationId(u32::try_from(index).expect("sealed relation id fits"));
                theory
                    .key_projections_of(relation)
                    .iter()
                    .filter_map(|id| theory.projection(*id))
                    .filter(|projection| {
                        matches!(projection.encoding, KeyEncoding::ExactBounded { .. })
                            && matches!(
                                theory.distinctness_witness(projection.id),
                                Some(DistinctnessWitness::ScalarKeyUnique { .. })
                            )
                    })
                    .min_by_key(|projection| (projection.encoding.routing_width(), projection.id))
                    .map(|projection| projection.id)
            })
            .collect();
        Ok(Self { theory, home })
    }

    /// The relation's home: its narrowest exact scalar key, the stable
    /// projection id breaking ties. Rows of a relation without one are homed
    /// by their fingerprint.
    pub(crate) fn home_projection(&self, relation: RelationId) -> Option<&CompiledProjection> {
        self.home
            .get(relation.0 as usize)
            .copied()
            .flatten()
            .and_then(|id| self.theory.projection(id))
    }

    /// The home projection is served by the row bucket, not a `det` entry.
    pub(crate) fn is_home(&self, projection: &CompiledProjection) -> bool {
        self.home.get(projection.relation.0 as usize) == Some(&Some(projection.id))
    }

    /// Declaration order, including empty and closed relations.
    pub(crate) fn relations(&self) -> impl ExactSizeIterator<Item = RelationId> + '_ {
        (0..self.home.len())
            .map(|index| RelationId(u32::try_from(index).expect("sealed relation id fits")))
    }

    pub(crate) fn fields_of(&self, relation: RelationId) -> Option<&[FieldDescriptor]> {
        self.theory.fields_of(relation)
    }

    pub(crate) fn projection(&self, id: ProjectionId) -> Option<&CompiledProjection> {
        self.theory.projection(id)
    }

    pub(crate) fn projection_of(&self, statement: StatementId) -> Option<&CompiledProjection> {
        self.theory.projection_of_statement(statement)
    }

    #[cfg(test)]
    pub(crate) fn source_of(&self, statement: StatementId) -> Option<&CompiledProjection> {
        self.theory.source_projection(statement)
    }

    pub(crate) fn key_for(
        &self,
        relation: RelationId,
        projection: &[bumbledb_theory::schema::FieldId],
    ) -> Option<&CompiledProjection> {
        self.theory.key_for(relation, projection)
    }

    /// Descriptor-based visit of one decoded row's interned projections.
    /// Only scalar routing is emitted; interval values stay in canonical rows.
    ///
    /// # Errors
    /// Work exhaustion or sink failure.
    pub(crate) fn emit_decoded(
        &self,
        relation: RelationId,
        values: &[crate::Value],
        work: &WorkContext,
        emit: Emit<'_>,
    ) -> Result<()> {
        for id in self.theory.projections_of_relation(relation) {
            let projection = self.theory.projection(*id).expect("indexed id");
            work.checkpoint()?;
            let mut exact = [0; MAX_EXACT_SCALAR_BYTES];
            let fingerprinted;
            let projected = match projection.encoding {
                KeyEncoding::ExactBounded { .. } => projection
                    .encode_scalar_row(values, &mut exact)
                    .ok_or(Error::ForeignSchema)?,
                KeyEncoding::FingerprintBucket => {
                    let scalars = projection.scalar_values(values);
                    fingerprinted = determinant_bytes(projection, &scalars, work)?;
                    fingerprinted.as_slice()
                }
            };
            emit(projection, projected)?;
        }
        Ok(())
    }
}

/// Owns projected bytes until their last index consumer finishes. Small
/// exact routes live inline; canonical projections keep their original
/// allocation and working-byte reservation together, without a second copy.
#[derive(Debug)]
pub(crate) enum DeterminantBytes {
    Exact {
        bytes: [u8; MAX_EXACT_SCALAR_BYTES],
        len: u8,
    },
    Canonical(crate::canonical::CanonicalRow),
}

impl DeterminantBytes {
    pub(crate) fn as_slice(&self) -> &[u8] {
        match self {
            Self::Exact { bytes, len } => &bytes[..usize::from(*len)],
            Self::Canonical(row) => row.as_bytes(),
        }
    }
}

impl std::ops::Deref for DeterminantBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
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
) -> Result<DeterminantBytes> {
    match projection.encoding {
        KeyEncoding::ExactBounded { scalar_width } => {
            let mut bytes = [0; MAX_EXACT_SCALAR_BYTES];
            let encoded = encode_scalar_group_into(values, &projection.scalar_fields, &mut bytes)
                .ok_or(Error::ForeignSchema)?;
            if encoded.len() != usize::from(scalar_width) {
                return Err(Error::ForeignSchema);
            }
            Ok(DeterminantBytes::Exact {
                bytes,
                len: scalar_width,
            })
        }
        KeyEncoding::FingerprintBucket => Ok(DeterminantBytes::Canonical(
            crate::canonical::CanonicalRow::encode(&projection.scalar_fields, values, work)?,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Interval;
    use crate::Value;
    use crate::encoding::encode_u64;
    use crate::schema::compiled::ProjectionInternKey;
    use crate::schema::compiled::{CompileError, KeyEncoding};
    use crate::schema::tests::{capacity, containment, fd, field, side};
    use crate::schema::{
        IntervalElement, RelationDescriptor, SchemaDescriptor, ValidateDescriptor as _, ValueType,
    };
    use crate::work::WorkContext;
    use bumbledb_theory::schema::FieldId;

    fn work() -> WorkContext {
        WorkContext::new()
    }

    fn projection(fields: Vec<FieldDescriptor>, encoding: KeyEncoding) -> CompiledProjection {
        CompiledProjection {
            id: ProjectionId(0),
            relation: RelationId(0),
            projection: (0..fields.len())
                .map(|index| FieldId(u16::try_from(index).unwrap()))
                .collect(),
            scalar_positions: (0..fields.len()).collect(),
            scalar_fields: fields.into(),
            encoding,
            interval_position: None,
            interval_type: None,
        }
    }

    #[test]
    fn exact_projection_owner_is_inline_and_checks_its_compiled_width() {
        let context = work();
        let mut projection = projection(
            vec![field("uuid", ValueType::Uuid)],
            KeyEncoding::ExactBounded { scalar_width: 16 },
        );
        let uuid = crate::Uuid::from_bytes([0xAB; 16]);
        let before = crate::alloc_counter::snapshot().window;
        let owner = determinant_bytes(&projection, &[Value::Uuid(uuid)], &context).unwrap();
        let after = crate::alloc_counter::snapshot().window;
        assert_eq!(
            after.allocs, before.allocs,
            "fixed-width determinant stays inline"
        );
        assert!(matches!(owner, DeterminantBytes::Exact { .. }));
        assert_eq!(owner.as_slice(), uuid.as_bytes());
        projection.encoding = KeyEncoding::ExactBounded { scalar_width: 8 };
        assert_eq!(
            determinant_bytes(&projection, &[Value::Uuid(uuid)], &context).unwrap_err(),
            Error::ForeignSchema
        );
        projection.scalar_fields = Box::new([]);
        projection.encoding = KeyEncoding::ExactBounded { scalar_width: 0 };
        assert!(
            determinant_bytes(&projection, &[], &context)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn canonical_projection_moves_storage_and_releases_it_with_its_owner() {
        let projection = projection(
            vec![field("text", ValueType::String)],
            KeyEncoding::FingerprintBucket,
        );
        let values = [Value::String("héllo🦀".repeat(1000).into())];
        let donor = work();
        let expected =
            crate::canonical::CanonicalRow::encode(&projection.scalar_fields, &values, &donor)
                .unwrap();
        let context = work();
        let owner = determinant_bytes(&projection, &values, &context).unwrap();
        assert!(matches!(owner, DeterminantBytes::Canonical(_)));
        assert_eq!(owner.as_slice(), expected.as_bytes());
        let bytes = owner.len() as u64;
        let address = owner.as_slice().as_ptr();
        let moved = owner;
        assert_eq!(moved.as_slice().as_ptr(), address);
        assert_eq!(moved.as_slice(), expected.as_bytes());
        let fingerprinter = super::super::fingerprint::Fingerprinter::Blake3;
        assert_eq!(
            fingerprinter.determinant(projection.id, &moved),
            fingerprinter.determinant(projection.id, expected.as_bytes())
        );
        let before = crate::alloc_counter::snapshot().window;
        drop(moved);
        let after = crate::alloc_counter::snapshot().window;
        assert!(after.dealloc_bytes - before.dealloc_bytes >= bytes);
    }

    #[test]
    fn canonical_projection_owners_are_independent_and_honor_cancellation() {
        use crate::canonical::RowError;
        use crate::work::WorkError;
        let projection = projection(
            vec![field("text", ValueType::String)],
            KeyEncoding::FingerprintBucket,
        );
        let values = [Value::String("hello".into())];
        let context = work();
        let first = determinant_bytes(&projection, &values, &context).unwrap();
        let second = determinant_bytes(&projection, &values, &context).unwrap();
        assert_eq!(first.as_slice(), second.as_slice());
        assert_ne!(first.as_slice().as_ptr(), second.as_slice().as_ptr());
        let bytes = first.as_slice().to_vec();
        drop(first);
        assert_eq!(second.as_slice(), bytes);
        context.cancel();
        assert_eq!(
            determinant_bytes(&projection, &values, &context).unwrap_err(),
            Error::from(RowError::Work(WorkError::Cancelled))
        );
        assert_eq!(
            second.as_slice(),
            bytes,
            "cancellation does not invalidate existing owners"
        );
    }

    fn table(schema: &Schema) -> DeterminantTable {
        DeterminantTable::compile(schema).expect("projection ids")
    }

    #[test]
    fn home_selection_uses_shortest_exact_key_then_stable_projection_id() {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![
                    field("wide", ValueType::Uuid),
                    field("first", ValueType::U64),
                    field("second", ValueType::I64),
                ],
            }],
            statements: vec![
                fd(RelationId(0), &[FieldId(0)]),
                fd(RelationId(0), &[FieldId(1)]),
                fd(RelationId(0), &[FieldId(2)]),
            ],
        }
        .validate()
        .unwrap();
        let det = table(&schema);
        let selected = det.home_projection(RelationId(0)).unwrap();
        assert_eq!(selected.projection.as_ref(), &[FieldId(1)]);
        assert!(det.is_home(selected));
        for projection in det.theory.projections() {
            assert_eq!(det.is_home(projection), projection.id == selected.id);
        }
        assert_eq!(
            selected.encoding,
            KeyEncoding::ExactBounded { scalar_width: 8 }
        );
        assert_eq!(
            table(&schema).home_projection(RelationId(0)).unwrap().id,
            selected.id
        );
        assert_eq!(det.relations().collect::<Vec<_>>(), [RelationId(0)]);
        assert!(det.home_projection(RelationId(1)).is_none());
    }

    #[test]
    fn home_selection_excludes_hash_interval_and_nonkey_projections() {
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "Target".into(),
                    fields: vec![field("id", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Source".into(),
                    fields: vec![field("parent", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Text".into(),
                    fields: vec![field("key", ValueType::String)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Interval".into(),
                    fields: vec![
                        field("key", ValueType::U64),
                        field(
                            "span",
                            ValueType::Interval {
                                element: IntervalElement::I64,
                            },
                        ),
                    ],
                },
                RelationDescriptor {
                    extension: None,
                    name: "Wide".into(),
                    fields: vec![field("key", ValueType::FixedBytes { len: 17 })],
                },
            ],
            statements: vec![
                fd(RelationId(0), &[FieldId(0)]),
                containment(
                    side(RelationId(1), &[FieldId(0)]),
                    side(RelationId(0), &[FieldId(0)]),
                ),
                fd(RelationId(2), &[FieldId(0)]),
                fd(RelationId(3), &[FieldId(0), FieldId(1)]),
                fd(RelationId(4), &[FieldId(0)]),
            ],
        }
        .validate()
        .unwrap();
        let det = table(&schema);
        assert!(det.home_projection(RelationId(0)).is_some());
        assert!(
            det.source_of(StatementId(1)).is_some(),
            "source has a real index but no key law"
        );
        for relation in [RelationId(1), RelationId(2), RelationId(3), RelationId(4)] {
            assert!(det.home_projection(relation).is_none());
        }
        assert_eq!(det.relations().len(), 5);
    }

    #[test]
    fn bounded_emission_preserves_work_refusals_and_rejects_bad_scalar_shape() {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "T".into(),
                fields: vec![field("id", ValueType::U64)],
            }],
            statements: vec![fd(RelationId(0), &[FieldId(0)])],
        }
        .validate()
        .unwrap();
        let det = table(&schema);
        let context = work();
        let mut visits = 0;
        det.emit_decoded(
            RelationId(0),
            &[Value::U64(42)],
            &context,
            &mut |_, routing| {
                assert_eq!(routing, encode_u64(42));
                visits += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(visits, 1);
        for row in [vec![], vec![Value::Bool(true)]] {
            let error = det
                .emit_decoded(RelationId(0), &row, &context, &mut |_, _| {
                    panic!("invalid shape must not reach sink")
                })
                .unwrap_err();
            assert_eq!(error, Error::ForeignSchema);
        }
        context.cancel();
        assert!(matches!(
            det.emit_decoded(RelationId(0), &[Value::U64(42)], &context, &mut |_, _| {
                panic!("cancelled work must not reach sink")
            }),
            Err(Error::Cancelled)
        ));
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
            &mut |id, bytes| {
                t_emits.push((id.id, bytes.to_vec()));
                Ok(())
            },
        )
        .expect("T emit");
        assert_eq!(
            t_emits.len(),
            1,
            "T key and containment/capacity target share"
        );
        assert_eq!(t_emits[0].1, encode_u64(9));
        let intern = CompiledTheory::intern_key(det.projection(t_emits[0].0).expect("id"));
        assert_eq!(
            intern.encoding,
            KeyEncoding::ExactBounded { scalar_width: 8 }
        );

        let mut s_emits = Vec::new();
        det.emit_decoded(
            RelationId(1),
            &[Value::U64(1), Value::U64(9)],
            &work(),
            &mut |id, bytes| {
                s_emits.push((id.id, bytes.to_vec()));
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
        assert!(
            s_emits
                .iter()
                .any(|(id, bytes)| *id == parent && bytes == &encode_u64(9))
        );
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
                    &mut |_, _| {
                        count += 1;
                        Ok(())
                    },
                )
                .expect("emit");
            }
            visits.push((n, count / usize::try_from(n).expect("bounded test count")));
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
                &mut |id, bytes| {
                    entries.push((id.id, bytes.to_vec()));
                    Ok(())
                },
            )
            .expect("emit");
        }
        assert_eq!(entries.len(), 2, "multimap keeps both tentative rows");
        assert_eq!(entries[0], entries[1]);
    }

    #[test]
    fn d04_pointwise_emit_groups_distinct_intervals_by_scalars_only() {
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
        let mut entries = Vec::new();
        for (start, end) in [(10u64, 20), (3, 8)] {
            let span = Interval::new(start, end).expect("span");
            det.emit_decoded(
                RelationId(0),
                &[Value::U64(4), Value::IntervalU64(span)],
                &work(),
                &mut |id, routing| {
                    assert_eq!(routing, encode_u64(4));
                    entries.push((id.id, routing.to_vec()));
                    Ok(())
                },
            )
            .expect("emit");
        }
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0], entries[1],
            "the row ordinal separates intervals"
        );
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
        let stopped = CompiledTheory::consume_visits(
            DistinctnessWitness::ExistenceOnly { projection },
            0..32,
            &mut |_| {
                seen += 1;
                Ok::<_, Error>(VisitControl::Sufficient)
            },
        )
        .expect("existence");
        assert_eq!(stopped, VisitOutcome::Sufficient { visited: 1 });
        assert_eq!(seen, 1);

        seen = 0;
        let halt = CompiledTheory::consume_visits(
            DistinctnessWitness::ScalarKeyUnique { projection },
            0..32,
            &mut |_| {
                seen += 1;
                Ok::<_, Error>(VisitControl::Stop)
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
