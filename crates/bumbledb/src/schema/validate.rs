//! Declaration validation: the boundary that turns a [`SchemaDescriptor`]
//! into the sealed [`Schema`] witness.
//! Field checks first, then the statement roster and acceptance gate of
//! — exhaustive, one distinct
//! (the variant doc comments carry the citations). Every accepted
//! statement leaves as a typed arena witness; downstream trusts its
use std::collections::BTreeMap;

use super::{
    AxiomIndex, Bound, CapacityEnforcement, CapacityId, CapacityStatement, CompiledCheck,
    CompiledSide, CompiledSides, ContainmentId, ContainmentStatement, DisjointDeterminantProof,
    EncodableCheck, Enforcement, FactLayout, FieldDescriptor, FieldId, Generation, KeyForm, KeyId,
    KeyStatement, LiteralSet, MemberSet, Pairing, Relation, RelationBody, RelationDescriptor,
    RelationId, Schema, SchemaDescriptor, SealedBound, SealedWeight, Side, StatementDescriptor,
    StatementId, StatementRef, Survivors, ValueMismatch, ValueType, Weight, value_matches,
};
use crate::encoding::{field_bytes, field_word_bytes};
use crate::error::{Mismatch, RowIndex, SchemaError, StatementErrorKind, TargetKeyCandidate};
use crate::storage::keys::MAX_DETERMINANT_WIDTH;
use bumbledb_theory::Value;

/// The admission boundary as an extension trait: [`SchemaDescriptor`] is
/// theory data (hosted in `bumbledb-theory`), so the engine-side sealing
/// pass hangs off it here rather than as an inherent method.
pub trait ValidateDescriptor: Sized {
    /// # Errors
    fn validate(self) -> Result<Schema, SchemaError>;
}

impl ValidateDescriptor for SchemaDescriptor {
    /// # Panics
    /// Only on one programmer-invariant violation: more than 2³²
    /// [`SchemaError::TooManyStatements`]) checked before any u16 id is
    #[expect(
        clippy::too_many_lines,
        reason = "the one materialized-order sealing pass — one arm per \
                  statement form, clearer kept together"
    )]
    fn validate(self) -> Result<Schema, SchemaError> {
        for (rel_idx, decl) in self.relations.iter().enumerate() {
            let columns = derived_columns(decl);
            if columns > usize::from(u16::MAX) {
                return Err(SchemaError::RelationTooManyColumns {
                    relation: RelationId(u32::try_from(rel_idx).expect("relation count fits u32")),
                    columns,
                });
            }
        }

        let descriptors = self.materialized_statements();

        // materialized roster past it is a typed rejection before any

        if descriptors.len() > 1 << 16 {
            return Err(SchemaError::TooManyStatements {
                count: descriptors.len(),
            });
        }

        let mut relations = Vec::with_capacity(self.relations.len());
        for (rel_idx, decl) in self.relations.into_iter().enumerate() {
            let rel_id = RelationId(u32::try_from(rel_idx).expect("relation count fits u32"));
            relations.push(validate_relation(rel_id, decl)?);
        }

        for (idx, relation) in relations.iter().enumerate() {
            if relations[..idx].iter().any(|r| r.name == relation.name) {
                return Err(SchemaError::DuplicateRelationName {
                    name: relation.name.clone(),
                });
            }
        }

        // descriptor list, so a key may still be declared after its probe.

        let normalized: Vec<StatementIdentity> =
            descriptors.iter().map(StatementIdentity::of).collect();
        let key_count = descriptors
            .iter()
            .filter(|descriptor| matches!(descriptor, StatementDescriptor::Functionality { .. }))
            .count();
        let mut keys = Vec::with_capacity(key_count);
        let mut containments = Vec::new();
        let mut capacities = Vec::new();
        let mut order = Vec::with_capacity(descriptors.len());
        let mut relation_keys: Vec<Vec<KeyId>> = vec![Vec::new(); relations.len()];
        let mut relation_outgoing: Vec<Vec<ContainmentId>> = vec![Vec::new(); relations.len()];
        let mut relation_capacity_sources: Vec<Vec<CapacityId>> = vec![Vec::new(); relations.len()];
        let mut relation_capacity_targets: Vec<Vec<CapacityId>> = vec![Vec::new(); relations.len()];
        let mut dependents: Vec<Vec<ContainmentId>> = vec![Vec::new(); key_count];

        for (idx, descriptor) in descriptors.iter().enumerate() {
            let id = statement_id(idx);
            let sealed = match descriptor {
                StatementDescriptor::Functionality {
                    relation,
                    projection,
                } => {
                    let evidence = validate_functionality(
                        id,
                        *relation,
                        projection,
                        &relations,
                        &descriptors,
                    )?;
                    let key_id =
                        KeyId(u16::try_from(keys.len()).expect("statement count fits u16"));
                    relation_keys[relation.0 as usize].push(key_id);
                    keys.push(KeyStatement {
                        id,
                        relation: *relation,
                        projection: projection.clone(),
                        form: match evidence {
                            FunctionalityEvidence::Pointwise(disjoint, tail) => {
                                KeyForm::Pointwise { tail, disjoint }
                            }
                            FunctionalityEvidence::Scalar => {
                                let mint = first_fresh_field(&relations[relation.0 as usize]);
                                if projection.len() == 1 && mint == Some(projection[0]) {
                                    KeyForm::FreshRow {
                                        field: projection[0],
                                    }
                                } else {
                                    KeyForm::Scalar
                                }
                            }
                        },
                    });
                    StatementRef::Key(key_id)
                }
                StatementDescriptor::Containment { source, target } => {
                    let enforcement =
                        validate_containment(id, source, target, &relations, &descriptors)?;
                    let containment_id = ContainmentId(
                        u16::try_from(containments.len()).expect("statement count fits u16"),
                    );
                    if let Some(target_key) = enforcement.target_key() {
                        dependents[usize::from(target_key.0)].push(containment_id);
                    }
                    relation_outgoing[source.relation.0 as usize].push(containment_id);
                    containments.push(ContainmentStatement {
                        id,
                        source: canonical_side(source),
                        target: canonical_side(target),
                        enforcement,
                        survivors: survivors_of(&relations[source.relation.0 as usize]),
                        checks: CompiledSides {
                            source: compiled_side(
                                &source.selection,
                                &relations[source.relation.0 as usize],
                            ),
                            target: compiled_side(
                                &target.selection,
                                &relations[target.relation.0 as usize],
                            ),
                        },
                        pairing: Pairing::OneWay,
                    });
                    StatementRef::Containment(containment_id)
                }
                StatementDescriptor::Capacity {
                    target,
                    weight,
                    lo,
                    hi,
                    source,
                } => {
                    let sealed = validate_capacity(
                        id,
                        target,
                        *weight,
                        *lo,
                        *hi,
                        source,
                        &relations,
                        &descriptors,
                    )?;
                    let capacity_id = CapacityId(
                        u16::try_from(capacities.len()).expect("statement count fits u16"),
                    );
                    relation_capacity_sources[source.relation.0 as usize].push(capacity_id);
                    relation_capacity_targets[target.relation.0 as usize].push(capacity_id);
                    capacities.push(CapacityStatement {
                        id,
                        target: canonical_side(target),
                        weight: sealed.weight,
                        lo: *lo,
                        hi: sealed.hi,
                        source: canonical_side(source),
                        enforcement: sealed.enforcement,
                        checks: CompiledSides {
                            source: compiled_side(
                                &source.selection,
                                &relations[source.relation.0 as usize],
                            ),
                            target: compiled_side(
                                &target.selection,
                                &relations[target.relation.0 as usize],
                            ),
                        },
                    });
                    StatementRef::Capacity(capacity_id)
                }
            };

            if let Some(earlier) = normalized[..idx].iter().position(|n| *n == normalized[idx]) {
                return Err(StatementErrorKind::DuplicateStatement {
                    earlier: statement_id(earlier),
                }
                .at(id));
            }
            order.push(sealed);
        }
        pair_mirrors(&mut containments, &order, &normalized);

        for (((relation, rel_keys), outgoing), (capacity_sources, capacity_targets)) in relations
            .iter_mut()
            .zip(relation_keys)
            .zip(relation_outgoing)
            .zip(
                relation_capacity_sources
                    .into_iter()
                    .zip(relation_capacity_targets),
            )
        {
            relation.keys = rel_keys.into_boxed_slice();
            relation.outgoing = outgoing.into_boxed_slice();
            relation.capacity_sources = capacity_sources.into_boxed_slice();
            relation.capacity_targets = capacity_targets.into_boxed_slice();
            if let RelationBody::Ordinary { fresh } = &mut relation.body {
                *fresh = relation.keys.iter().copied().find(|&key_id| {
                    matches!(keys[usize::from(key_id.0)].form, KeyForm::FreshRow { .. })
                });
            }
        }

        let schema = Schema {
            identity: std::sync::OnceLock::new(),
            relations: relations.into_boxed_slice(),
            keys: keys.into_boxed_slice(),
            containments: containments.into_boxed_slice(),
            capacities: capacities.into_boxed_slice(),
            order: order.into_boxed_slice(),
            dependents: dependents.into_iter().map(Vec::into_boxed_slice).collect(),
        };
        // Compile identity once with the schema; change ingestion and command
        // sealing must not repeatedly allocate/hash its complete descriptor.
        let _ = super::fingerprint::fingerprint(&schema);
        Ok(schema)
    }
}

fn mirror_of(normalized: &[StatementIdentity], index: usize) -> Option<StatementId> {
    let StatementIdentity::Containment { source, target } = &normalized[index] else {
        return None;
    };
    normalized
        .iter()
        .enumerate()
        .find(|(other, descriptor)| {
            *other != index
                && matches!(
                    descriptor,
                    StatementIdentity::Containment {
                        source: mirror_source,
                        target: mirror_target,
                    } if mirror_source == target && mirror_target == source
                )
        })
        .map(|(other, _)| statement_id(other))
}

/// Keys and capacities cannot have a partner, so they do not occupy holes.
pub(super) fn mirror_links(
    descriptors: &[StatementDescriptor],
) -> BTreeMap<StatementId, StatementId> {
    let normalized: Vec<StatementIdentity> =
        descriptors.iter().map(StatementIdentity::of).collect();
    (0..normalized.len())
        .filter_map(|index| {
            let StatementIdentity::Containment { .. } = &normalized[index] else {
                return None;
            };
            mirror_of(&normalized, index).map(|partner| (statement_id(index), partner))
        })
        .collect()
}

/// The materialized-order [`StatementId`] for a list index (the typed
/// [`SchemaError::TooManyStatements`] gate runs before any id is minted, so the
/// expect is a true invariant).
fn statement_id(index: usize) -> StatementId {
    StatementId(u16::try_from(index).expect("statement count fits u16"))
}

/// Fill [`ContainmentStatement::pairing`] after every containment has a witness
/// id — a partner later in the list is not yet minted at push time, so the
/// stored identity is the arena id, not a re-resolved [`StatementId`].
fn pair_mirrors(
    containments: &mut [ContainmentStatement],
    order: &[StatementRef],
    normalized: &[StatementIdentity],
) {
    for containment in containments.iter_mut() {
        containment.pairing = match mirror_of(normalized, usize::from(containment.id.0)) {
            None => Pairing::OneWay,
            Some(partner) => match order[usize::from(partner.0)] {
                StatementRef::Containment(id) => Pairing::Mirror(id),
                StatementRef::Key(_) | StatementRef::Capacity(_) => {
                    unreachable!("mirror_of only pairs containments")
                }
            },
        };
    }
}

fn survivors_of(source: &Relation) -> Survivors {
    if source.body.closed_rows().is_some() {
        Survivors::SealedRows
    } else {
        Survivors::ReverseEdges
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldSet(Box<[FieldId]>);

impl FieldSet {
    fn new(fields: &[FieldId]) -> Result<Self, FieldId> {
        let mut canonical = fields.to_vec();
        canonical.sort_unstable();
        if let Some(duplicate) = canonical
            .windows(2)
            .find_map(|pair| (pair[0] == pair[1]).then_some(pair[0]))
        {
            return Err(duplicate);
        }
        Ok(Self(canonical.into_boxed_slice()))
    }
}

struct Projection<'a> {
    ordered: &'a [FieldId],
    fields: FieldSet,
}

impl Projection<'_> {
    fn ordered(&self) -> &[FieldId] {
        self.ordered
    }

    fn fields(&self) -> &FieldSet {
        &self.fields
    }
}

#[derive(Clone, Copy)]
enum FunctionalityEvidence {
    Scalar,

    Pointwise(DisjointDeterminantProof, ValueType),
}

/// Q1 — element-domain typing at interval positions: two interval types of one
/// element domain match positionally WHATEVER their widths (the pointwise
/// judgments quantify over points, which carry an element domain and not a
/// width — `lean/Bumbledb/Schema.lean: Value.points`; the coverage walk is
/// width-blind by construction, `storage/commit/judgment.rs::check_coverage`).
fn positional_types_match(a: &ValueType, b: &ValueType) -> bool {
    match (a.interval_element(), b.interval_element()) {
        (Some(ea), Some(eb)) => ea == eb,
        _ => a == b,
    }
}

fn interval_positions(fields: &[FieldDescriptor], projection: &[FieldId]) -> Vec<usize> {
    projection
        .iter()
        .enumerate()
        .filter(|(_, field)| {
            matches!(
                fields[usize::from(field.0)].value_type,
                ValueType::Interval { .. } | ValueType::FixedInterval { .. }
            )
        })
        .map(|(pos, _)| pos)
        .collect()
}

fn literal_cmp(a: &Value, b: &Value) -> std::cmp::Ordering {
    fn rank(value: &Value) -> u8 {
        match value {
            Value::Bool(_) => 0,
            Value::U64(_) => 1,
            Value::I64(_) => 2,
            Value::String(_) => 3,
            Value::FixedBytes(_) => 4,
            Value::IntervalU64(_) => 5,
            Value::IntervalI64(_) => 6,
            Value::F64(_) => 7,
        }
    }
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::U64(x), Value::U64(y)) => x.cmp(y),
        (Value::I64(x), Value::I64(y)) => x.cmp(y),
        (Value::F64(x), Value::F64(y)) => x.cmp(y),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::FixedBytes(x), Value::FixedBytes(y)) => x.cmp(y),
        (Value::IntervalU64(x), Value::IntervalU64(y)) => {
            (x.start(), x.end()).cmp(&(y.start(), y.end()))
        }
        (Value::IntervalI64(x), Value::IntervalI64(y)) => {
            (x.start(), x.end()).cmp(&(y.start(), y.end()))
        }
        _ => rank(a).cmp(&rank(b)),
    }
}

/// Duplicates were rejected by [`validate_side_shape`] before any side seals,
/// so sorting is the whole canonicalization.
fn canonical_literals(literals: &LiteralSet) -> LiteralSet {
    match literals {
        LiteralSet::One(_) => literals.clone(),
        LiteralSet::Many(values) => {
            let mut sorted = values.to_vec();
            sorted.sort_by(literal_cmp);
            LiteralSet::Many(sorted.into_boxed_slice())
        }
    }
}

fn canonical_side(side: &Side) -> Side {
    Side {
        relation: side.relation,
        projection: side.projection.clone(),
        selection: side
            .selection
            .iter()
            .map(|(field, literals)| (*field, canonical_literals(literals)))
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedSide {
    relation: RelationId,
    projection: Box<[FieldId]>,
    selection: Box<[(FieldId, LiteralSet)]>,
}

impl NormalizedSide {
    fn new(side: &Side) -> Self {
        let mut selection: Vec<_> = side
            .selection
            .iter()
            .map(|(field, literals)| (*field, canonical_literals(literals)))
            .collect();
        selection.sort_by_key(|(field, _)| *field);
        Self {
            relation: side.relation,
            projection: side.projection.clone(),
            selection: selection.into_boxed_slice(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StatementIdentity {
    Functionality {
        relation: RelationId,
        projection: Box<[FieldId]>,
    },
    Containment {
        source: NormalizedSide,
        target: NormalizedSide,
    },
    Capacity {
        target: NormalizedSide,
        weight: Weight,
        lo: u64,
        hi: Option<Bound>,
        source: NormalizedSide,
    },
}

impl StatementIdentity {
    fn of(descriptor: &StatementDescriptor) -> Self {
        match descriptor {
            StatementDescriptor::Functionality {
                relation,
                projection,
            } => Self::Functionality {
                relation: *relation,
                projection: projection.clone(),
            },
            StatementDescriptor::Containment { source, target } => Self::Containment {
                source: NormalizedSide::new(source),
                target: NormalizedSide::new(target),
            },
            StatementDescriptor::Capacity {
                target,
                weight,
                lo,
                hi,
                source,
            } => Self::Capacity {
                target: NormalizedSide::new(target),
                weight: *weight,
                lo: *lo,
                hi: *hi,
                source: NormalizedSide::new(source),
            },
        }
    }
}

fn validate_functionality(
    id: StatementId,
    relation_id: RelationId,
    projection: &[FieldId],
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
) -> Result<FunctionalityEvidence, SchemaError> {
    let relation = known_relation(id, relation_id, relations)?;
    let projection = validate_projection(id, relation_id, projection, relation)?;

    let positions = interval_positions(&relation.fields, projection.ordered());
    if positions.len() > 1 {
        return Err(StatementErrorKind::FunctionalityMultipleIntervals {
            relation: relation_id,
            field: projection.ordered()[positions[1]],
        }
        .at(id));
    }
    let interval_position = positions.first().copied();
    if let Some(pos) = interval_position
        && pos != projection.ordered().len() - 1
    {
        return Err(StatementErrorKind::FunctionalityIntervalNotLast {
            relation: relation_id,
            field: projection.ordered()[pos],
        }
        .at(id));
    }

    let tail = interval_position.map(|pos| {
        let idx = usize::from(projection.ordered()[pos].0);
        match relation.fields[idx].value_type {
            ty if ty.is_interval() => ty,
            _ => unreachable!("interval_positions found an interval field"),
        }
    });

    let this_set = projection.fields();
    for (idx, earlier) in descriptors[..usize::from(id.0)].iter().enumerate() {
        if let StatementDescriptor::Functionality {
            relation: r,
            projection: p,
        } = earlier
            && *r == relation_id
            && FieldSet::new(p).is_ok_and(|set| &set == this_set)
        {
            return Err(StatementErrorKind::DuplicateFunctionality {
                earlier: statement_id(idx),
            }
            .at(id));
        }
    }

    let width: usize = projection
        .ordered()
        .iter()
        .map(|field| relation.fields[usize::from(field.0)].value_type.width())
        .sum();
    if width > MAX_DETERMINANT_WIDTH {
        return Err(StatementErrorKind::DeterminantKeyTooWide { width }.at(id));
    }

    if let Some(rows) = relation.body.closed_rows() {
        let layout = &relation.layout;
        let scalar_len = projection.ordered().len() - usize::from(interval_position.is_some());
        for (row_idx, row) in rows.iter().enumerate() {
            for earlier in &rows[..row_idx] {
                let scalars_agree = projection.ordered()[..scalar_len].iter().all(|field| {
                    let idx = usize::from(field.0);
                    field_bytes(layout.encoded(&row.fact), idx)
                        == field_bytes(layout.encoded(&earlier.fact), idx)
                });
                if !scalars_agree {
                    continue;
                }
                let collide = match interval_position.zip(tail) {
                    None => true,
                    Some((pos, tail)) => {
                        let idx = usize::from(projection.ordered()[pos].0);

                        // programmer invariant, never data.
                        let (a_start, a_end) = crate::encoding::interval_words(
                            tail,
                            field_bytes(layout.encoded(&row.fact), idx),
                        )
                        .expect("sealed rows hold canonical interval bytes");
                        let (b_start, b_end) = crate::encoding::interval_words(
                            tail,
                            field_bytes(layout.encoded(&earlier.fact), idx),
                        )
                        .expect("sealed rows hold canonical interval bytes");
                        a_start < b_end && b_start < a_end
                    }
                };
                if collide {
                    return Err(StatementErrorKind::ClosedStatementRefuted {
                        relation: relation_id,
                        row: RowIndex(row_idx),
                    }
                    .at(id));
                }
            }
        }
    }

    Ok(match tail {
        Some(tail) => FunctionalityEvidence::Pointwise(DisjointDeterminantProof(()), tail),
        None => FunctionalityEvidence::Scalar,
    })
}

fn validate_containment(
    id: StatementId,
    source: &Side,
    target: &Side,
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
) -> Result<Enforcement, SchemaError> {
    let target_projection = validate_side_pair(id, source, target, relations)?;

    // Interval positions on closed containments: refused v0. A pointwise

    let target_fields = &relations[target.relation.0 as usize].fields;
    let source_closed = matches!(
        relations[source.relation.0 as usize].body,
        RelationBody::Closed { .. }
    );
    let target_closed = matches!(
        relations[target.relation.0 as usize].body,
        RelationBody::Closed { .. }
    );
    if (source_closed || target_closed)
        && !interval_positions(target_fields, &target.projection).is_empty()
    {
        return Err(StatementErrorKind::ClosedContainmentInterval {
            relation: if target_closed {
                target.relation
            } else {
                source.relation
            },
        }
        .at(id));
    }

    let resolved = resolve_target_key(
        id,
        source,
        target,
        &target_projection,
        relations,
        descriptors,
        relations[source.relation.0 as usize].interval_tail(&source.projection),
    )?;

    if let (Enforcement::Closed { members }, Some(rows)) = (
        &resolved,
        relations[source.relation.0 as usize].body.closed_rows(),
    ) {
        let layout = &relations[source.relation.0 as usize].layout;
        let phi = encodable_checks(
            &source.selection,
            &relations[source.relation.0 as usize].fields,
        );
        for (row_idx, row) in rows.iter().enumerate() {
            if !sealed_satisfies(&phi, layout, &row.fact) {
                continue;
            }
            let word = decoded_word(layout, source.projection[0], &row.fact);

            if !AxiomIndex::try_from(word).is_ok_and(|index| members.contains(index)) {
                return Err(StatementErrorKind::ClosedStatementRefuted {
                    relation: source.relation,
                    row: RowIndex(row_idx),
                }
                .at(id));
            }
        }
    }

    Ok(resolved)
}

struct SealedCapacity {
    enforcement: CapacityEnforcement,
    weight: SealedWeight,
    hi: SealedBound,
}

/// The premises are exactly the model's (`lean/Bumbledb/Admission.lean:
/// capacityForm`; `lean/Bumbledb/Oracle.lean: capacity_plan_decides` is the
/// promised plan): the canonical window vocabulary with its weight-sensitivity
/// law, WEIGHT typing (a `[field]` weight is a u64 SOURCE position, a
/// `[Duration(field)]` weight an interval one), DEPENDENT-BOUND typing (a bound
/// ident is a u64 or interval position of TARGET's row, by name against the
/// whole roster — C1; dimension mixing refused — C18), the shared side shapes,
/// the containment target-key rule reused verbatim, and the v0 interval refusal
/// narrowed to PROJECTIONS — the group key identifies facts, and intervals
/// enter through the measure argument (`lean/Bumbledb/Capacity.lean` § v0
/// refusals; *trigger* for lifting: a sighted counting-over-denotation
/// workload).
#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the descriptor's own field roster, threaded once — the one \
              acceptance arm per statement form (the `validate` precedent)"
)]
fn validate_capacity(
    id: StatementId,
    target: &Side,
    weight: Weight,
    lo: u64,
    hi: Option<Bound>,
    source: &Side,
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
) -> Result<SealedCapacity, SchemaError> {
    // nothing at any weight (`lean/Bumbledb/Capacity.lean:

    // duplicate spelling (`lean/Bumbledb/Subsumption.lean:

    match hi {
        Some(Bound::Lit(hi)) if hi < lo => {
            return Err(StatementErrorKind::CapacityInvertedWindow { lo, hi }.at(id));
        }
        None if lo == 0 => {
            return Err(StatementErrorKind::CapacityVacuousWindow.at(id));
        }
        None if lo == 1 && weight == Weight::Unit => {
            return Err(StatementErrorKind::CapacityContainmentWindow.at(id));
        }
        _ => {}
    }

    let target_projection = validate_side_pair(id, source, target, relations)?;

    // The v0 interval refusal, narrowed to projections: capacity

    let source_fields = &relations[source.relation.0 as usize].fields;
    let positions = interval_positions(source_fields, &source.projection);
    if let Some(pos) = positions.first() {
        return Err(StatementErrorKind::CapacityIntervalPosition {
            relation: source.relation,
            field: source.projection[*pos],
        }
        .at(id));
    }

    // signed field under `[field]` is the typed polarity refusal (an

    let sealed_weight = match weight {
        Weight::Unit => SealedWeight::Unit,
        Weight::Field(field) => {
            let descriptor = known_field(id, source.relation, field, relations)?;
            if descriptor.value_type != ValueType::U64 {
                return Err(StatementErrorKind::CapacityWeightNotU64 {
                    relation: source.relation,
                    field,
                }
                .at(id));
            }
            SealedWeight::Field(field)
        }
        Weight::DurationOf(field) => {
            let descriptor = known_field(id, source.relation, field, relations)?;
            if !descriptor.value_type.is_interval() {
                return Err(StatementErrorKind::CapacityWeightNotDuration {
                    relation: source.relation,
                    field,
                }
                .at(id));
            }
            let tail = descriptor.value_type;
            SealedWeight::Duration { field, tail }
        }
    };

    // window against it mixes dimensions and is refused. A u64-field

    let sealed_hi = match hi {
        None => SealedBound::Unbounded,
        Some(Bound::Lit(n)) => SealedBound::Lit(n),
        Some(Bound::TargetField(field)) => {
            let descriptor = known_field(id, target.relation, field, relations)?;
            if descriptor.value_type != ValueType::U64 {
                return Err(StatementErrorKind::CapacityBoundNotU64 {
                    relation: target.relation,
                    field,
                }
                .at(id));
            }
            if matches!(weight, Weight::DurationOf(_)) {
                return Err(StatementErrorKind::CapacityDimensionMixing { field }.at(id));
            }
            SealedBound::TargetField(field)
        }
        Some(Bound::TargetDuration(field)) => {
            let descriptor = known_field(id, target.relation, field, relations)?;
            if !descriptor.value_type.is_interval() {
                return Err(StatementErrorKind::CapacityBoundNotDuration {
                    relation: target.relation,
                    field,
                }
                .at(id));
            }
            let tail = descriptor.value_type;
            if !matches!(weight, Weight::DurationOf(_)) {
                return Err(StatementErrorKind::CapacityDimensionMixing { field }.at(id));
            }
            SealedBound::Duration { field, tail }
        }
    };

    let enforcement = resolve_capacity_target(
        id,
        source,
        target,
        &target_projection,
        relations,
        descriptors,
    )?;

    // (`lean/Bumbledb/Schema.lean: den_closed_constant`; a per-row

    if let (CapacityEnforcement::Closed { .. }, Some(source_rows)) = (
        &enforcement,
        relations[source.relation.0 as usize].body.closed_rows(),
    ) {
        let target_relation = &relations[target.relation.0 as usize];
        let target_rows = target_relation
            .body
            .closed_rows()
            .expect("the Closed enforcement arm resolves only against a closed target");
        let source_layout = &relations[source.relation.0 as usize].layout;
        let phi = encodable_checks(&source.selection, source_fields);
        let psi = encodable_checks(&target.selection, &target_relation.fields);
        for (row_idx, parent) in target_rows.iter().enumerate() {
            if !sealed_satisfies(&psi, &target_relation.layout, &parent.fact) {
                continue;
            }

            let resolved_hi = crate::storage::commit::judgment::resolve_bound(
                sealed_hi,
                &target_relation.layout,
                &parent.fact,
                id,
            )
            .expect("sealed extension rows carry no ray or inverted intervals");
            let measure: u128 = source_rows
                .iter()
                .filter(|child| {
                    sealed_satisfies(&phi, source_layout, &child.fact)
                        && source
                            .projection
                            .iter()
                            .zip(target.projection.iter())
                            .all(|(s, t)| {
                                field_bytes(source_layout.encoded(&child.fact), usize::from(s.0))
                                    == field_bytes(
                                        target_relation.layout.encoded(&parent.fact),
                                        usize::from(t.0),
                                    )
                            })
                })
                .map(|child| {
                    u128::from(
                        crate::storage::commit::judgment::measure_weight(
                            sealed_weight,
                            source_layout,
                            &child.fact,
                            id,
                        )
                        .expect("sealed extension rows carry no ray or inverted intervals"),
                    )
                })
                .sum();
            let over = match resolved_hi {
                crate::schema::BoundCeiling::Unbounded => false,
                crate::schema::BoundCeiling::Finite(hi) => measure > u128::from(hi),
            };
            if measure < u128::from(lo) || over {
                return Err(StatementErrorKind::ClosedStatementRefuted {
                    relation: target.relation,
                    row: RowIndex(row_idx),
                }
                .at(id));
            }
        }
    }

    Ok(SealedCapacity {
        enforcement,
        weight: sealed_weight,
        hi: sealed_hi,
    })
}

fn known_field(
    id: StatementId,
    relation: RelationId,
    field: FieldId,
    relations: &[Relation],
) -> Result<&FieldDescriptor, SchemaError> {
    relations[relation.0 as usize]
        .fields
        .get(usize::from(field.0))
        .ok_or(StatementErrorKind::UnknownField { relation, field }.at(id))
}

fn encoded_literal(literal: &Value, desc: bumbledb_theory::schema::ValueType) -> Box<[u8]> {
    let mut bytes = Vec::with_capacity(16);
    crate::encoding::encode_literal(literal, desc, &mut bytes);
    bytes.into()
}

fn compiled_side(selection: &[(FieldId, LiteralSet)], relation: &Relation) -> CompiledSide {
    if relation.body.closed_rows().is_some() {
        CompiledSide::Closed(encodable_checks(selection, &relation.fields))
    } else {
        CompiledSide::Ordinary(compiled_checks(selection, &relation.fields))
    }
}

fn compiled_checks(
    selection: &[(FieldId, LiteralSet)],
    fields: &[FieldDescriptor],
) -> Box<[CompiledCheck]> {
    selection
        .iter()
        .map(|(field, literals)| {
            let desc = fields[usize::from(field.0)].value_type;
            match canonical_literals(literals) {
                LiteralSet::One(Value::String(text)) => CompiledCheck::Interned {
                    field: *field,
                    text: text.clone(),
                },
                LiteralSet::One(literal) => CompiledCheck::Encoded {
                    field: *field,
                    bytes: encoded_literal(&literal, desc),
                },

                LiteralSet::Many(values) if matches!(values[0], Value::String(_)) => {
                    CompiledCheck::InternedSet {
                        field: *field,
                        texts: values
                            .iter()
                            .map(|value| {
                                let Value::String(text) = value else {
                                    unreachable!("validated string binding is homogeneous")
                                };
                                text.clone()
                            })
                            .collect(),
                    }
                }
                LiteralSet::Many(values) => CompiledCheck::EncodedSet {
                    field: *field,
                    alternatives: values
                        .iter()
                        .map(|literal| encoded_literal(literal, desc))
                        .collect(),
                },
            }
        })
        .collect()
}

fn encodable_checks(
    selection: &[(FieldId, LiteralSet)],
    fields: &[FieldDescriptor],
) -> Box<[EncodableCheck]> {
    selection
        .iter()
        .map(|(field, literals)| {
            let desc = fields[usize::from(field.0)].value_type;
            match canonical_literals(literals) {
                LiteralSet::One(Value::String(_)) => {
                    unreachable!("closed relations refuse str columns")
                }
                LiteralSet::One(literal) => EncodableCheck::Encoded {
                    field: *field,
                    bytes: encoded_literal(&literal, desc),
                },
                LiteralSet::Many(values) if matches!(values.first(), Some(Value::String(_))) => {
                    unreachable!("closed relations refuse str columns")
                }
                LiteralSet::Many(values) => EncodableCheck::EncodedSet {
                    field: *field,
                    alternatives: values
                        .iter()
                        .map(|literal| encoded_literal(literal, desc))
                        .collect(),
                },
            }
        })
        .collect()
}

fn sealed_satisfies(checks: &[EncodableCheck], layout: &FactLayout, fact: &[u8]) -> bool {
    checks.iter().all(|check| check.matches(layout, fact))
}

fn decoded_word(layout: &FactLayout, field: FieldId, fact: &[u8]) -> u64 {
    u64::from_be_bytes(field_word_bytes(layout.encoded(fact), usize::from(field.0)))
}

fn known_relation(
    id: StatementId,
    relation: RelationId,
    relations: &[Relation],
) -> Result<&Relation, SchemaError> {
    relations
        .get(relation.0 as usize)
        .ok_or(StatementErrorKind::UnknownRelation { relation }.at(id))
}

fn validate_projection<'p>(
    id: StatementId,
    relation_id: RelationId,
    projection: &'p [FieldId],
    relation: &Relation,
) -> Result<Projection<'p>, SchemaError> {
    if projection.is_empty() {
        return Err(StatementErrorKind::EmptyProjection {
            relation: relation_id,
        }
        .at(id));
    }
    for field in projection {
        if usize::from(field.0) >= relation.fields.len() {
            return Err(StatementErrorKind::UnknownField {
                relation: relation_id,
                field: *field,
            }
            .at(id));
        }
    }
    let fields = FieldSet::new(projection).map_err(|field| {
        StatementErrorKind::DuplicateProjectionField {
            relation: relation_id,
            field,
        }
        .at(id)
    })?;
    Ok(Projection {
        ordered: projection,
        fields,
    })
}

/// The shared side-pair gate of the two two-sided forms — ONE definition site,
/// exactly as `resolve_target_key` is shared (the Lean model states one
/// acceptance rule: `lean/Bumbledb/Admission.lean: containmentForm` /
/// `capacityForm` take their sides through one structure). Form-specific
/// refusals (the closed-interval refusal, the capacity window vocabulary and
/// interval bans) stay with their callers.
fn validate_side_pair<'t>(
    id: StatementId,
    source: &Side,
    target: &'t Side,
    relations: &[Relation],
) -> Result<Projection<'t>, SchemaError> {
    validate_side_shape(id, source, relations)?;
    let target_projection = validate_side_shape(id, target, relations)?;

    if source.projection.len() != target.projection.len() {
        return Err(StatementErrorKind::ContainmentArityMismatch {
            mismatch: Mismatch {
                witnessed: source.projection.len(),
                required: target.projection.len(),
            },
        }
        .at(id));
    }

    let source_fields = &relations[source.relation.0 as usize].fields;
    let target_fields = &relations[target.relation.0 as usize].fields;
    for (position, (s, t)) in source
        .projection
        .iter()
        .zip(target.projection.iter())
        .enumerate()
    {
        if !positional_types_match(
            &source_fields[usize::from(s.0)].value_type,
            &target_fields[usize::from(t.0)].value_type,
        ) {
            return Err(StatementErrorKind::ContainmentTypeMismatch { position }.at(id));
        }
    }

    validate_side_selection(id, source, relations)?;
    validate_side_selection(id, target, relations)?;

    Ok(target_projection)
}

fn validate_side_shape<'s>(
    id: StatementId,
    side: &'s Side,
    relations: &[Relation],
) -> Result<Projection<'s>, SchemaError> {
    let relation = known_relation(id, side.relation, relations)?;
    let projection = validate_projection(id, side.relation, &side.projection, relation)?;
    for (idx, (field, literals)) in side.selection.iter().enumerate() {
        if usize::from(field.0) >= relation.fields.len() {
            return Err(StatementErrorKind::UnknownField {
                relation: side.relation,
                field: *field,
            }
            .at(id));
        }
        if side.selection[..idx].iter().any(|(f, _)| f == field) {
            return Err(StatementErrorKind::DuplicateSelectionField {
                relation: side.relation,
                field: *field,
            }
            .at(id));
        }
        if let LiteralSet::Many(values) = literals {
            if values.len() < 2 {
                return Err(StatementErrorKind::DegenerateSelectionSet {
                    relation: side.relation,
                    field: *field,
                    len: values.len(),
                }
                .at(id));
            }
            for (value_idx, value) in values.iter().enumerate() {
                if values[..value_idx]
                    .iter()
                    .any(|earlier| literal_cmp(earlier, value) == std::cmp::Ordering::Equal)
                {
                    return Err(StatementErrorKind::DuplicateSelectionLiteral {
                        relation: side.relation,
                        field: *field,
                    }
                    .at(id));
                }
            }
        }
    }
    Ok(projection)
}

fn validate_side_selection(
    id: StatementId,
    side: &Side,
    relations: &[Relation],
) -> Result<(), SchemaError> {
    let relation = &relations[side.relation.0 as usize];
    for (field, _) in &side.selection {
        if side.projection.contains(field) {
            return Err(StatementErrorKind::SelectedFieldProjected {
                relation: side.relation,
                field: *field,
            }
            .at(id));
        }
    }
    for (field, literals) in &side.selection {
        for literal in literals.literals() {
            validate_selection_literal(
                id,
                side.relation,
                *field,
                &relation.fields[usize::from(field.0)].value_type,
                literal,
            )?;
        }
    }
    Ok(())
}

fn validate_selection_literal(
    id: StatementId,
    relation: RelationId,
    field: FieldId,
    value_type: &ValueType,
    literal: &Value,
) -> Result<(), SchemaError> {
    value_matches(literal, value_type).map_err(|ValueMismatch::Type| {
        StatementErrorKind::SelectionLiteralTypeMismatch { relation, field }.at(id)
    })
}

fn resolve_target_key(
    id: StatementId,
    source: &Side,
    target: &Side,
    target_projection: &Projection<'_>,
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
    source_tail: Option<ValueType>,
) -> Result<Enforcement, SchemaError> {
    let target_relation = &relations[target.relation.0 as usize];

    // projection must be exactly the synthetic id — its OWN refusal, not

    // the refused field set, and the rule here is closedness, not key

    if let Some(rows) = target_relation.body.closed_rows() {
        if target.projection.len() != 1 || target.projection[0] != FieldId(0) {
            return Err(StatementErrorKind::ClosedTargetNotHandle {
                target: target.relation,
                target_name: target_relation.name.clone(),
                projection: target.projection.clone(),
                projection_names: projection_field_names(target_relation, &target.projection),
            }
            .at(id));
        }
        return Ok(Enforcement::Closed {
            members: compile_member_set(target_relation, target, rows),
        });
    }

    let target_fields = &target_relation.fields;
    let positions = interval_positions(target_fields, &target.projection);

    if positions.len() > 1 {
        return Err(missing_target_key(id, target, relations, descriptors, true));
    }
    let interval_position = positions.first().copied();

    let want = target_projection.fields();
    let Some((key_idx, key_projection)) =
        matching_functionality(target.relation, want, descriptors)
    else {
        return Err(missing_target_key(
            id,
            target,
            relations,
            descriptors,
            interval_position.is_some(),
        ));
    };

    let key_projection_in_order =
        source_key_projection(&source.projection, target_projection, key_projection);
    let target_key = functionality_key_id(descriptors, key_idx);

    if interval_position.is_some() {
        let FunctionalityEvidence::Pointwise(disjoint, target_tail) = validate_functionality(
            statement_id(key_idx),
            target.relation,
            key_projection,
            relations,
            descriptors,
        )?
        else {
            unreachable!("a set-equal interval projection resolves to a pointwise key")
        };
        let Some(source_tail) = source_tail else {
            unreachable!("positional type match: a coverage target implies an interval source");
        };
        Ok(Enforcement::IntervalCoverage {
            target_key,
            key_projection: key_projection_in_order,
            disjoint,
            source_tail,
            target_tail,
        })
    } else {
        Ok(Enforcement::ScalarProbe {
            target_key,
            key_projection: key_projection_in_order,
        })
    }
}

/// Coverage is unrepresentable — projections already refused interval
/// positions.
fn resolve_capacity_target(
    id: StatementId,
    source: &Side,
    target: &Side,
    target_projection: &Projection<'_>,
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
) -> Result<CapacityEnforcement, SchemaError> {
    let target_relation = &relations[target.relation.0 as usize];
    if let Some(rows) = target_relation.body.closed_rows() {
        if target.projection.len() != 1 || target.projection[0] != FieldId(0) {
            return Err(StatementErrorKind::ClosedTargetNotHandle {
                target: target.relation,
                target_name: target_relation.name.clone(),
                projection: target.projection.clone(),
                projection_names: projection_field_names(target_relation, &target.projection),
            }
            .at(id));
        }
        return Ok(CapacityEnforcement::Closed {
            members: compile_member_set(target_relation, target, rows),
        });
    }

    let Some((key_idx, key_projection)) =
        matching_functionality(target.relation, target_projection.fields(), descriptors)
    else {
        return Err(missing_target_key(
            id,
            target,
            relations,
            descriptors,
            false,
        ));
    };
    Ok(CapacityEnforcement::ScalarProbe {
        target_key: functionality_key_id(descriptors, key_idx),
        key_projection: source_key_projection(
            &source.projection,
            target_projection,
            key_projection,
        ),
    })
}

fn matching_functionality<'a>(
    relation: RelationId,
    want: &FieldSet,
    descriptors: &'a [StatementDescriptor],
) -> Option<(usize, &'a [FieldId])> {
    descriptors
        .iter()
        .enumerate()
        .find_map(|(index, descriptor)| match descriptor {
            StatementDescriptor::Functionality {
                relation: r,
                projection,
            } if *r == relation && FieldSet::new(projection).is_ok_and(|set| &set == want) => {
                Some((index, projection.as_ref()))
            }
            StatementDescriptor::Functionality { .. }
            | StatementDescriptor::Containment { .. }
            | StatementDescriptor::Capacity { .. } => None,
        })
}

fn source_key_projection(
    source_projection: &[FieldId],
    target_projection: &Projection<'_>,
    key_projection: &[FieldId],
) -> Box<[FieldId]> {
    key_projection
        .iter()
        .map(|key_field| {
            let pos = target_projection
                .ordered()
                .iter()
                .position(|field| field == key_field)
                .expect("set-equal projection contains every key field");
            source_projection[pos]
        })
        .collect()
}

fn functionality_key_id(descriptors: &[StatementDescriptor], key_idx: usize) -> KeyId {
    KeyId(
        u16::try_from(
            descriptors[..key_idx]
                .iter()
                .filter(|descriptor| {
                    matches!(descriptor, StatementDescriptor::Functionality { .. })
                })
                .count(),
        )
        .expect("statement count fits u16"),
    )
}

/// The projection was validated (`validate_projection`) before any rejection
/// citing it, so the index is total.
fn projection_field_names(relation: &Relation, projection: &[FieldId]) -> Box<[Box<str>]> {
    projection
        .iter()
        .map(|field| relation.fields[usize::from(field.0)].name.clone())
        .collect()
}

fn target_key_candidates(
    target: RelationId,
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
) -> Box<[TargetKeyCandidate]> {
    let mut next_key = 0usize;
    let mut available = Vec::new();
    for descriptor in descriptors {
        if let StatementDescriptor::Functionality {
            relation,
            projection,
        } = descriptor
        {
            let key = KeyId(u16::try_from(next_key).expect("statement count fits u16"));
            next_key += 1;
            if *relation == target {
                available.push(TargetKeyCandidate {
                    key,
                    projection: projection.clone(),
                    projection_names: projection_field_names(
                        &relations[target.0 as usize],
                        projection,
                    ),
                });
            }
        }
    }
    available.into_boxed_slice()
}

fn missing_target_key(
    statement: StatementId,
    side: &Side,
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
    pointwise: bool,
) -> SchemaError {
    let target = side.relation;
    let target_relation = &relations[target.0 as usize];
    let target_name = target_relation.name.clone();
    let projection = side.projection.clone();
    let projection_names = projection_field_names(target_relation, &projection);
    let available = target_key_candidates(target, relations, descriptors);
    if pointwise {
        StatementErrorKind::NoPointwiseTargetKey {
            target,
            target_name,
            projection,
            projection_names,
            available,
        }
        .at(statement)
    } else {
        StatementErrorKind::NoMatchingTargetKey {
            target,
            target_name,
            projection,
            projection_names,
            available,
        }
        .at(statement)
    }
}

/// The extension passed validation before statement resolution, so every
/// declaration index is below [`super::MAX_EXTENSION_ROWS`].
fn compile_member_set(target: &Relation, side: &Side, rows: &[super::SealedRow]) -> MemberSet {
    let psi = encodable_checks(&side.selection, &target.fields);
    let mut members = MemberSet::empty();
    for (idx, row) in rows.iter().enumerate() {
        if sealed_satisfies(&psi, &target.layout, &row.fact) {
            let index =
                AxiomIndex(u8::try_from(idx).expect("the validated extension cap is below 256"));
            members.insert(index);
        }
    }
    members
}

/// An interval field spans two word columns, a `bytes<N>` field its `⌈N/8⌉` —
/// never counted below one: `bytes<0>` is invalid, but its width rejection runs
/// only after the u16 field ids are minted, so the cap must be a true lower
/// bound on any legal repair of the declaration.
fn derived_columns(decl: &RelationDescriptor) -> usize {
    usize::from(decl.extension.is_some())
        + decl
            .fields
            .iter()
            .map(|field| match field.value_type {
                ValueType::Interval { .. } | ValueType::FixedInterval { .. } => 2,
                ValueType::FixedBytes { len } => crate::encoding::fixed_bytes_words(len).max(1),
                _ => 1,
            })
            .sum::<usize>()
}

fn validate_relation(
    rel_id: RelationId,
    decl: RelationDescriptor,
) -> Result<Relation, SchemaError> {
    let RelationDescriptor {
        name,
        fields: declared,
        extension,
    } = decl;

    let mut fields = Vec::with_capacity(declared.len() + usize::from(extension.is_some()));
    if extension.is_some() {
        fields.push(FieldDescriptor {
            name: "id".into(),
            value_type: ValueType::U64,
            generation: Generation::None,
        });
    }
    fields.extend(declared);

    for (idx, field) in fields.iter().enumerate() {
        let field_id = FieldId(u16::try_from(idx).expect("field count fits u16"));
        if fields[..idx].iter().any(|f| f.name == field.name) {
            return Err(SchemaError::DuplicateFieldName {
                relation: rel_id,
                name: field.name.clone(),
            });
        }
        if let ValueType::FixedBytes { len } = field.value_type {
            // bytes<N> width gate: N ∈ 1..=64 (64 bytes = 8 words).
            if len == 0 || usize::from(len) > crate::encoding::MAX_FIXED_BYTES {
                return Err(SchemaError::FixedBytesWidthOutOfRange {
                    relation: rel_id,
                    field: field_id,
                    len,
                });
            }
        }
        if let ValueType::FixedInterval { width, .. } = field.value_type {
            // interval<E, w> width gate: w ≥ 1 and w ≤ u64::MAX − 1.
            if width == 0 || width == u64::MAX {
                return Err(SchemaError::IntervalWidthOutOfRange {
                    relation: rel_id,
                    field: field_id,
                    width,
                });
            }
        }
        if field.generation == Generation::Fresh && field.value_type != ValueType::U64 {
            return Err(SchemaError::FreshOnNonU64 {
                relation: rel_id,
                field: field_id,
            });
        }

        // intrinsic-vs-policy law). `str` is refused — the handle IS the

        // dictionary writes at open; `fresh` is refused — identity is the

        // no refusal anymore: a reference to a closed relation is a plain

        if extension.is_some() {
            if field.value_type == ValueType::String {
                return Err(SchemaError::StrOnClosedRelation {
                    relation: rel_id,
                    field: field_id,
                });
            }
            if field.generation == Generation::Fresh {
                return Err(SchemaError::FreshOnClosedRelation {
                    relation: rel_id,
                    field: field_id,
                });
            }
        }
    }

    let layout = FactLayout::new(&fields.iter().map(|f| f.value_type).collect::<Vec<_>>());

    let body = match extension {
        None => RelationBody::Ordinary { fresh: None },
        Some(rows) => RelationBody::Closed {
            extension: validate_extension(rel_id, &fields, &layout, &rows)?,
        },
    };

    Ok(Relation {
        name,
        fields: fields.into_boxed_slice(),
        layout,
        keys: Box::new([]),
        outgoing: Box::new([]),
        capacity_sources: Box::new([]),
        capacity_targets: Box::new([]),
        body,
    })
}

/// The FIRST `Fresh`-generation field of a relation — used only while sealing
/// keys, before the ordinary arm's `fresh: Option<KeyId>` is minted.
fn first_fresh_field(relation: &Relation) -> Option<FieldId> {
    relation
        .fields()
        .iter()
        .position(|f| f.generation == Generation::Fresh)
        .map(|idx| FieldId(u16::try_from(idx).expect("field count fits u16")))
}

/// The extension roster: ground axioms validated through the one shared
/// [`value_matches`] check and canonically encoded ONCE — each sealed row
/// carries its full fact bytes (synthetic id ‖ intrinsic values), never
/// re-encoded after validate (the staging law applied to the feature itself).
fn validate_extension(
    rel_id: RelationId,
    fields: &[FieldDescriptor],
    layout: &FactLayout,
    rows: &[super::Row],
) -> Result<Box<[super::SealedRow]>, SchemaError> {
    if rows.is_empty() {
        return Err(SchemaError::EmptyExtension { relation: rel_id });
    }
    if rows.len() > super::MAX_EXTENSION_ROWS {
        return Err(SchemaError::ExtensionTooManyRows {
            relation: rel_id,
            count: rows.len(),
        });
    }
    let columns = fields.len() - 1;
    let mut sealed = Vec::with_capacity(rows.len());
    for (row_idx, row) in rows.iter().enumerate() {
        if rows[..row_idx].iter().any(|r| r.handle == row.handle) {
            return Err(SchemaError::DuplicateExtensionHandle {
                relation: rel_id,
                handle: row.handle.clone(),
            });
        }
        if row.values.len() != columns {
            return Err(SchemaError::ExtensionArityMismatch {
                relation: rel_id,
                row: RowIndex(row_idx),
                mismatch: Mismatch {
                    witnessed: row.values.len(),
                    required: columns,
                },
            });
        }
        let mut fact = Vec::with_capacity(layout.fact_width());
        fact.extend_from_slice(&crate::encoding::encode_u64(
            u64::try_from(row_idx).expect("row count fits u64"),
        ));
        for (value, (field_idx, field)) in row.values.iter().zip(fields.iter().enumerate().skip(1))
        {
            let field_id = FieldId(u16::try_from(field_idx).expect("field count fits u16"));
            value_matches(value, &field.value_type).map_err(|ValueMismatch::Type| {
                SchemaError::ExtensionValueTypeMismatch {
                    relation: rel_id,
                    row: RowIndex(row_idx),
                    field: field_id,
                }
            })?;

            let is_ray = match value {
                Value::IntervalU64(interval) => interval.is_ray(),
                Value::IntervalI64(interval) => interval.is_ray(),
                _ => false,
            };
            if is_ray {
                return Err(SchemaError::ExtensionIntervalRay {
                    relation: rel_id,
                    row: RowIndex(row_idx),
                    field: field_id,
                });
            }
            // Total here: String and enums (refused columns) and AllenMask
            // (no field type) all fail `value_matches` before reaching the

            crate::encoding::encode_literal(value, field.value_type, &mut fact);
        }
        debug_assert_eq!(fact.len(), layout.fact_width());
        sealed.push(super::SealedRow {
            handle: row.handle.clone(),
            fact: fact.into_boxed_slice(),
        });
    }
    Ok(sealed.into_boxed_slice())
}
