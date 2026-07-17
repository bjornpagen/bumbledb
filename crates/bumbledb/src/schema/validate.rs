//! Declaration validation: the boundary that turns a [`SchemaDescriptor`]
//! into the sealed [`Schema`] witness.
//!
//! Field checks first, then the statement roster and acceptance gate of
//! `docs/architecture/30-dependencies.md` — exhaustive, one distinct
//! [`SchemaError`] per roster line (the variant doc comments carry the
//! citations). Every accepted statement leaves as a typed arena witness;
//! downstream trusts its shape without re-checking.
//!
//! The roster's "FD with selection" and "non-key FD form" lines have no
//! checks here: [`StatementDescriptor::Functionality`] carries neither a
//! selection nor a Y side, so both shapes are unrepresentable rather than
//! rejected.

use super::{
    AxiomIndex, CardinalityStatement, CompiledCheck, CompiledSides, ContainmentId,
    ContainmentStatement, DisjointDeterminantProof, Enforcement, FactLayout, FieldDescriptor,
    FieldId, Generation, IntervalTail, KeyId, KeyStatement, LiteralSet, MemberSet, Relation,
    RelationDescriptor, RelationId, Schema, SchemaDescriptor, SchemaWarning, Side,
    StatementDescriptor, StatementId, StatementRef, ValueMismatch, ValueType, WindowId,
    value_matches,
};
use crate::encoding::{field_bytes, field_word_bytes};
use crate::error::{SchemaError, TargetKeyCandidate};
use crate::storage::keys::MAX_DETERMINANT_WIDTH;
use bumbledb_theory::Value;

/// The admission boundary as an extension trait: [`SchemaDescriptor`] is
/// theory data (hosted in `bumbledb-theory`), so the engine-side sealing
/// pass hangs off it here rather than as an inherent method.
pub trait ValidateDescriptor: Sized {
    /// Validates the declaration into the sealed [`Schema`] witness.
    ///
    /// # Errors
    ///
    /// A distinct [`SchemaError`] per illegal shape — the field checks and
    /// the full statement roster; see the variant list.
    fn validate(self) -> Result<Schema, SchemaError>;
}

impl ValidateDescriptor for SchemaDescriptor {
    /// # Panics
    ///
    /// Only on one programmer-invariant violation: more than 2³²
    /// relations (unreachable — the descriptors alone exceed memory).
    /// Field and statement counts need no panic path: the derived
    /// column count and the materialized statement count are typed
    /// rejections ([`SchemaError::RelationTooManyColumns`],
    /// [`SchemaError::TooManyStatements`]) checked before any u16 id is
    /// minted.
    #[expect(
        clippy::too_many_lines,
        reason = "the one materialized-order sealing pass — one arm per \
                  statement form, clearer kept together"
    )]
    fn validate(self) -> Result<Schema, SchemaError> {
        // The derived-column cap runs FIRST — before
        // `materialized_statements` or the field loop mints any u16 id
        // from a field index (a fresh field's auto-key carries its
        // sealed index). See `derived_columns` for the bound.
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
        // The statement-id space is u16 (`StatementId`, and the
        // per-kind Key/Containment/Window ids it bounds): a
        // materialized roster past it is a typed rejection before any
        // per-statement validation walks it — never the id-mint expect.
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

        // Duplicate relation names.
        for (idx, relation) in relations.iter().enumerate() {
            if relations[..idx].iter().any(|r| r.name == relation.name) {
                return Err(SchemaError::DuplicateRelationName {
                    name: relation.name.clone(),
                });
            }
        }

        // The statement roster becomes three sealed structures in this one
        // materialized-order pass: two homogeneous typed arenas and the
        // StatementId spine selecting between them. Duplicate checks look
        // backward; containment target-key resolution sees the immutable
        // descriptor list, so a key may still be declared after its probe.
        let mut normalized: Vec<StatementDescriptor> = Vec::with_capacity(descriptors.len());
        let key_count = descriptors
            .iter()
            .filter(|descriptor| matches!(descriptor, StatementDescriptor::Functionality { .. }))
            .count();
        let mut keys = Vec::with_capacity(key_count);
        let mut containments = Vec::new();
        let mut windows = Vec::new();
        let mut order = Vec::with_capacity(descriptors.len());
        let mut relation_keys: Vec<Vec<KeyId>> = vec![Vec::new(); relations.len()];
        let mut relation_outgoing: Vec<Vec<ContainmentId>> = vec![Vec::new(); relations.len()];
        let mut relation_window_sources: Vec<Vec<WindowId>> = vec![Vec::new(); relations.len()];
        let mut relation_window_targets: Vec<Vec<WindowId>> = vec![Vec::new(); relations.len()];
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
                        pointwise: matches!(evidence, FunctionalityEvidence::Pointwise(_)),
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
                        checks: CompiledSides {
                            source: compiled_checks(
                                &source.selection,
                                &relations[source.relation.0 as usize].fields,
                            ),
                            target: compiled_checks(
                                &target.selection,
                                &relations[target.relation.0 as usize].fields,
                            ),
                        },
                        mirror: mirror_of(&descriptors, idx),
                    });
                    StatementRef::Containment(containment_id)
                }
                StatementDescriptor::Cardinality {
                    source,
                    lo,
                    hi,
                    target,
                } => {
                    let enforcement = validate_cardinality(
                        id,
                        source,
                        *lo,
                        *hi,
                        target,
                        &relations,
                        &descriptors,
                    )?;
                    let window_id =
                        WindowId(u16::try_from(windows.len()).expect("statement count fits u16"));
                    relation_window_sources[source.relation.0 as usize].push(window_id);
                    relation_window_targets[target.relation.0 as usize].push(window_id);
                    windows.push(CardinalityStatement {
                        id,
                        source: canonical_side(source),
                        lo: *lo,
                        hi: *hi,
                        target: canonical_side(target),
                        enforcement,
                        checks: CompiledSides {
                            source: compiled_checks(
                                &source.selection,
                                &relations[source.relation.0 as usize].fields,
                            ),
                            target: compiled_checks(
                                &target.selection,
                                &relations[target.relation.0 as usize].fields,
                            ),
                        },
                    });
                    StatementRef::Cardinality(window_id)
                }
            };
            // Roster "duplicate statements": identical descriptors after
            // normalization (selections sorted by FieldId). Identical FDs
            // never reach this — `DuplicateFunctionality` (a set rule, so
            // a superset of this equality) fired above.
            let norm = normalize(descriptor);
            if let Some(earlier) = normalized.iter().position(|n| *n == norm) {
                return Err(SchemaError::DuplicateStatement {
                    statement: id,
                    earlier: statement_id(earlier),
                });
            }
            normalized.push(norm);
            order.push(sealed);
        }

        for (((relation, keys), outgoing), (window_sources, window_targets)) in relations
            .iter_mut()
            .zip(relation_keys)
            .zip(relation_outgoing)
            .zip(
                relation_window_sources
                    .into_iter()
                    .zip(relation_window_targets),
            )
        {
            relation.keys = keys.into_boxed_slice();
            relation.outgoing = outgoing.into_boxed_slice();
            relation.window_sources = window_sources.into_boxed_slice();
            relation.window_targets = window_targets.into_boxed_slice();
        }

        Ok(Schema {
            warnings: redundant_superkeys(&keys),
            relations: relations.into_boxed_slice(),
            keys: keys.into_boxed_slice(),
            containments: containments.into_boxed_slice(),
            windows: windows.into_boxed_slice(),
            order: order.into_boxed_slice(),
            dependents: dependents.into_iter().map(Vec::into_boxed_slice).collect(),
        })
    }
}

/// The `==` partner of the statement at `index`: the first *other*
/// containment in the materialized list whose sides are exactly the swapped
/// sides — the one swapped-sides comparison site. Sealing calls it over
/// **all** statements (the lowered pair need not be adjacent — legal for
/// hand-built descriptors); the declared-side diagnostic renderer calls it
/// too, because a rejected declaration never seals a field to read. On a
/// *sealed* list the partner is unique and the links symmetric: a second
/// candidate mirror would equal the first, and
/// [`SchemaError::DuplicateStatement`] rejects identical normalized
/// statements. On a rejected declaration first-match is best-effort
/// diagnostics.
pub(super) fn mirror_of(descriptors: &[StatementDescriptor], index: usize) -> Option<StatementId> {
    let StatementDescriptor::Containment { source, target } = &descriptors[index] else {
        return None;
    };
    descriptors
        .iter()
        .enumerate()
        .find(|(other, descriptor)| {
            *other != index
                && matches!(
                    descriptor,
                    StatementDescriptor::Containment {
                        source: mirror_source,
                        target: mirror_target,
                    } if mirror_source == target && mirror_target == source
                )
        })
        .map(|(other, _)| statement_id(other))
}

/// The materialized-order [`StatementId`] for a list index (the typed
/// [`SchemaError::TooManyStatements`] gate runs before any id is
/// minted, so the expect is a true invariant).
fn statement_id(index: usize) -> StatementId {
    StatementId(u16::try_from(index).expect("statement count fits u16"))
}

/// Canonical projection identity. Construction sorts once and refuses
/// duplicates, so equality cannot accidentally compare written order.
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

    fn is_strict_subset_of(&self, other: &Self) -> bool {
        self.0.len() < other.0.len()
            && self
                .0
                .iter()
                .all(|field| other.0.binary_search(field).is_ok())
    }
}

/// Non-fatal key diagnostics, derived only after every accepted key has a
/// stable [`KeyId`]. A strict superkey remains fully sealed and enforced;
/// the warning records its determinant-write amplification.
fn redundant_superkeys(keys: &[KeyStatement]) -> Box<[SchemaWarning]> {
    let field_sets: Vec<FieldSet> = keys
        .iter()
        .map(|key| FieldSet::new(&key.projection).expect("sealed key projection is a set"))
        .collect();
    let mut warnings = Vec::new();
    for (key_position, key) in keys.iter().enumerate() {
        for (smaller_index, smaller) in keys.iter().enumerate() {
            if key.relation == smaller.relation
                && field_sets[smaller_index].is_strict_subset_of(&field_sets[key_position])
            {
                warnings.push(SchemaWarning::RedundantSuperkey {
                    relation: key.relation,
                    key: KeyId(u16::try_from(key_position).expect("statement count fits u16")),
                    implied_by: KeyId(
                        u16::try_from(smaller_index).expect("statement count fits u16"),
                    ),
                });
            }
        }
    }
    warnings.into_boxed_slice()
}

/// A validated projection carries both statement order (execution and key
/// permutation) and its canonical set (identity and key resolution).
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
    Pointwise(DisjointDeterminantProof),
}

/// The projection positions holding interval-typed fields — the one scan
/// behind the FD interval gate and the containment pointwise gate.
/// Q1 — element-domain typing at interval positions: two interval types
/// of one element domain match positionally WHATEVER their widths (the
/// pointwise judgments quantify over points, which carry an element
/// domain and not a width — `lean/Bumbledb/Schema.lean: Value.points`;
/// the coverage walk is width-blind by construction,
/// `storage/commit/judgment.rs::check_coverage`). Every other position
/// demands exact structural equality — scalar typing is untouched, and
/// u64-vs-i64 interval pairs still mismatch
/// (`docs/architecture/30-dependencies.md` § Q1).
fn positional_types_match(a: &ValueType, b: &ValueType) -> bool {
    match (a, b) {
        (ValueType::Interval { element: ea, .. }, ValueType::Interval { element: eb, .. }) => {
            ea == eb
        }
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
                ValueType::Interval { .. }
            )
        })
        .map(|(pos, _)| pos)
        .collect()
}

/// A deterministic total order over literal values — the canonical order
/// of a disjunctive binding's set (`docs/architecture/30-dependencies.md`
/// § validation roster: literal sets seal sorted and duplicate-free).
/// Within one validated binding every literal shares the field's type, so
/// the cross-variant rank only makes the comparison total on plain data.
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
            Value::AllenMask(_) => 7,
        }
    }
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::U64(x), Value::U64(y)) => x.cmp(y),
        (Value::I64(x), Value::I64(y)) => x.cmp(y),
        (Value::String(x), Value::String(y)) | (Value::FixedBytes(x), Value::FixedBytes(y)) => {
            x.cmp(y)
        }
        (Value::IntervalU64(x), Value::IntervalU64(y)) => {
            (x.start(), x.end()).cmp(&(y.start(), y.end()))
        }
        (Value::IntervalI64(x), Value::IntervalI64(y)) => {
            (x.start(), x.end()).cmp(&(y.start(), y.end()))
        }
        (Value::AllenMask(x), Value::AllenMask(y)) => x.bits().cmp(&y.bits()),
        _ => rank(a).cmp(&rank(b)),
    }
}

/// One binding's literal set in canonical form: `Many` sorts by
/// [`literal_cmp`]. Duplicates were rejected by
/// [`validate_side_shape`] before any side seals, so sorting is the whole
/// canonicalization.
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

/// One side with every disjunctive binding in canonical (sorted) literal
/// order — what seals into the arena and what the fingerprint hashes, so
/// two spellings of one set are one statement.
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

/// The descriptor with each selection sorted by [`FieldId`] and each
/// binding's literal set canonicalized — σ is a set of bindings over sets
/// of literals, so neither written order is identity (roster "duplicate
/// statements (identical normalized sides and form)").
fn normalize(descriptor: &StatementDescriptor) -> StatementDescriptor {
    fn side(side: &Side) -> Side {
        let canonical = canonical_side(side);
        let mut selection = canonical.selection.to_vec();
        selection.sort_by_key(|(field, _)| *field);
        Side {
            relation: canonical.relation,
            projection: canonical.projection,
            selection: selection.into_boxed_slice(),
        }
    }
    match descriptor {
        StatementDescriptor::Functionality { .. } => descriptor.clone(),
        StatementDescriptor::Containment { source, target } => StatementDescriptor::Containment {
            source: side(source),
            target: side(target),
        },
        StatementDescriptor::Cardinality {
            source,
            lo,
            hi,
            target,
        } => StatementDescriptor::Cardinality {
            source: side(source),
            lo: *lo,
            hi: *hi,
            target: side(target),
        },
    }
}

/// Roster "FD …" lines: `R(X) -> R` under the acceptance gate. Returns the
/// sealed scalar shape or the proof minted by the accepted pointwise arm.
fn validate_functionality(
    id: StatementId,
    relation_id: RelationId,
    projection: &[FieldId],
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
) -> Result<FunctionalityEvidence, SchemaError> {
    let relation = known_relation(id, relation_id, relations)?;
    let projection = validate_projection(id, relation_id, projection, relation)?;

    // Roster ">1 interval position" and "interval not in final position":
    // the neighbor probe needs the scalar prefix as its group; two interval
    // positions would be 2-D exclusion, which the ordered determinant cannot
    // answer.
    let positions = interval_positions(&relation.fields, projection.ordered());
    if positions.len() > 1 {
        return Err(SchemaError::FunctionalityMultipleIntervals {
            statement: id,
            relation: relation_id,
            field: projection.ordered()[positions[1]],
        });
    }
    let interval_position = positions.first().copied();
    if let Some(pos) = interval_position
        && pos != projection.ordered().len() - 1
    {
        return Err(SchemaError::FunctionalityIntervalNotLast {
            statement: id,
            relation: relation_id,
            field: projection.ordered()[pos],
        });
    }

    // Roster "duplicate statements", FD form: one field *set* per relation
    // — a second FD over the same set (any order) asserts the same
    // judgment, so its determinant is pure write amplification, and rejecting it
    // is what makes containment target-key resolution unambiguous.
    let this_set = projection.fields();
    for (idx, earlier) in descriptors[..usize::from(id.0)].iter().enumerate() {
        if let StatementDescriptor::Functionality {
            relation: r,
            projection: p,
        } = earlier
            && *r == relation_id
            && FieldSet::new(p).is_ok_and(|set| &set == this_set)
        {
            return Err(SchemaError::DuplicateFunctionality {
                statement: id,
                earlier: statement_id(idx),
            });
        }
    }

    // Roster "determinant width overflow": Σ field widths (intervals count 16)
    // must fit `MAX_DETERMINANT_WIDTH` — rejected at declaration, never
    // discovered at write time.
    let width: usize = projection
        .ordered()
        .iter()
        .map(|field| {
            relation.fields[usize::from(field.0)]
                .value_type
                .type_desc()
                .width()
        })
        .sum();
    if width > MAX_DETERMINANT_WIDTH {
        return Err(SchemaError::DeterminantKeyTooWide {
            statement: id,
            width,
        });
    }

    // A key on a closed relation is judged here, once: the axioms ARE the
    // final state (no commit ever touches the relation), so a colliding
    // pair refutes the statement now or never. Scalar keys collide on
    // equal projected bytes; a pointwise key collides when the scalar
    // prefix agrees and the intervals share a point — the ordered-neighbor
    // probe's judgment, run over ≤256 sealed rows instead of a determinant.
    if let Some(rows) = relation.extension.as_deref() {
        let layout = &relation.layout;
        let scalar_len = projection.ordered().len() - usize::from(interval_position.is_some());
        for (row_idx, row) in rows.iter().enumerate() {
            for earlier in &rows[..row_idx] {
                let scalars_agree = projection.ordered()[..scalar_len].iter().all(|field| {
                    let idx = usize::from(field.0);
                    field_bytes(&row.fact, layout, idx) == field_bytes(&earlier.fact, layout, idx)
                });
                if !scalars_agree {
                    continue;
                }
                let collide = match interval_position {
                    None => true,
                    Some(pos) => {
                        let field = projection.ordered()[pos];
                        let idx = usize::from(field.0);
                        let tail = IntervalTail {
                            width: match relation.fields[idx].value_type {
                                ValueType::Interval { width, .. } => width,
                                _ => unreachable!("interval_positions found an interval field"),
                            },
                        };
                        // Half-open `[s, e)` intersection on the
                        // order-preserving words (a fixed-width field's
                        // end derives from the type's width). Sealed rows
                        // encode at validate, so a malformed tail is a
                        // programmer invariant, never data.
                        let (a_start, a_end) = tail
                            .words(field_bytes(&row.fact, layout, idx))
                            .expect("sealed rows hold canonical interval bytes");
                        let (b_start, b_end) = tail
                            .words(field_bytes(&earlier.fact, layout, idx))
                            .expect("sealed rows hold canonical interval bytes");
                        a_start < b_end && b_start < a_end
                    }
                };
                if collide {
                    return Err(SchemaError::ClosedStatementRefuted {
                        statement: id,
                        relation: relation_id,
                        row: row_idx,
                    });
                }
            }
        }
    }

    Ok(match interval_position {
        Some(_) => FunctionalityEvidence::Pointwise(DisjointDeterminantProof(())),
        None => FunctionalityEvidence::Scalar,
    })
}

/// Roster "IND …" lines: `A(X | φ) <= B(Y | ψ)` under the acceptance gate.
/// Returns the resolved target key, its permutation, and the shared
/// interval position.
fn validate_containment(
    id: StatementId,
    source: &Side,
    target: &Side,
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
) -> Result<Enforcement, SchemaError> {
    validate_side_shape(id, source, relations)?;
    let target_projection = validate_side_shape(id, target, relations)?;

    // Roster "arity mismatch between sides": |X| = |Y|.
    if source.projection.len() != target.projection.len() {
        return Err(SchemaError::ContainmentArityMismatch {
            statement: id,
            source: source.projection.len(),
            target: target.projection.len(),
        });
    }

    // Roster "positional structural-type mismatch" — element-domain at
    // interval positions (Q1: widths free, elements bound), exact
    // structural equality everywhere else, which also covers the
    // called-out interval-against-scalar case
    // (`docs/architecture/10-data-model.md` structural equality).
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
            return Err(SchemaError::ContainmentTypeMismatch {
                statement: id,
                position,
            });
        }
    }

    validate_side_selection(id, source, relations)?;
    validate_side_selection(id, target, relations)?;

    // Interval positions on closed containments: refused v0. A pointwise
    // judgment against a closed target would mix the coverage walk with
    // virtual storage, and a constant source's coverage demand has no
    // delete-time re-judgment path — either closed side refuses
    // (`docs/architecture/30-dependencies.md`; trigger: a census
    // sighting). One check covers both sides: the positional type match
    // above makes the sides' interval positions identical.
    let source_closed = relations[source.relation.0 as usize].extension.is_some();
    let target_closed = relations[target.relation.0 as usize].extension.is_some();
    if (source_closed || target_closed)
        && !interval_positions(target_fields, &target.projection).is_empty()
    {
        return Err(SchemaError::ClosedContainmentInterval {
            statement: id,
            relation: if target_closed {
                target.relation
            } else {
                source.relation
            },
        });
    }

    let resolved = resolve_target_key(id, target, &target_projection, relations, descriptors)?;

    // Both sides constant: the judgment is decidable here, and a theory
    // whose axioms refute its own statement has no model to commit — the
    // source extension's φ-rows must all sit inside the compiled member
    // set (a closed source under an *ordinary* target stays commit-judged:
    // the target can shrink).
    if let (Enforcement::Closed { members }, Some(rows)) = (
        &resolved,
        relations[source.relation.0 as usize].extension.as_deref(),
    ) {
        let layout = &relations[source.relation.0 as usize].layout;
        let phi = compiled_checks(
            &source.selection,
            &relations[source.relation.0 as usize].fields,
        );
        for (row_idx, row) in rows.iter().enumerate() {
            if !sealed_satisfies(&phi, layout, &row.fact) {
                continue;
            }
            let word = decoded_word(layout, source.projection[0], &row.fact);
            // An out-of-range word narrows to non-membership — the same
            // miss the commit path takes (`storage/commit/judgment.rs`)
            // and the `AxiomIndex` contract ("values beyond `u16` are
            // absent") applied at validate: the row escapes the member
            // set, so the statement is refuted, never a panic.
            if !AxiomIndex::try_from(word).is_ok_and(|index| members.contains(index)) {
                return Err(SchemaError::ClosedStatementRefuted {
                    statement: id,
                    relation: source.relation,
                    row: row_idx,
                });
            }
        }
    }

    Ok(resolved)
}

/// Roster "cardinality …" lines: `B(Y | ψ) <={lo..hi} A(X | φ)` under
/// the acceptance gate (`docs/architecture/30-dependencies.md`). The
/// premises are exactly the model's
/// (`lean/Bumbledb/Admission.lean: cardinalityForm`;
/// `lean/Bumbledb/Oracle.lean: cardinality_plan_decides` is the promised
/// plan): the shared side shapes, the containment target-key rule reused
/// verbatim, and the v0 interval refusal — a window counts FACTS per
/// parent, and an interval position would make the count ambiguous
/// between facts and points (`lean/Bumbledb/Cardinality.lean` § v0
/// refusals; *trigger* for lifting: a sighted counting-over-denotation
/// workload). Closed-side rules mirror containment's: a closed target
/// compiles the member-set plan through the same key rule, and a
/// statement between constants is decided here outright.
fn validate_cardinality(
    id: StatementId,
    source: &Side,
    lo: u64,
    hi: Option<u64>,
    target: &Side,
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
) -> Result<Enforcement, SchemaError> {
    // The window vocabulary is closed (the canonical-utterance law,
    // `docs/architecture/70-api.md` — the descriptor face of the macro's
    // ban table): an inverted window is satisfied by no count, `0..*`
    // provably says nothing (`lean/Bumbledb/Cardinality.lean:
    // cardinality_zero_star`), and `1..*` is the bare containment's
    // duplicate spelling (`lean/Bumbledb/Subsumption.lean:
    // window_floor_containment`). Rejecting all three here means a sealed
    // schema holds canonical windows only — the renderer never faces a
    // banned spelling.
    match hi {
        Some(hi) if hi < lo => {
            return Err(SchemaError::CardinalityInvertedWindow {
                statement: id,
                lo,
                hi,
            });
        }
        None if lo == 0 => {
            return Err(SchemaError::CardinalityVacuousWindow { statement: id });
        }
        None if lo == 1 => {
            return Err(SchemaError::CardinalityContainmentWindow { statement: id });
        }
        _ => {}
    }

    validate_side_shape(id, source, relations)?;
    let target_projection = validate_side_shape(id, target, relations)?;

    // Roster "arity mismatch between sides": |X| = |Y| — the child group
    // compares whole projected tuples.
    if source.projection.len() != target.projection.len() {
        return Err(SchemaError::ContainmentArityMismatch {
            statement: id,
            source: source.projection.len(),
            target: target.projection.len(),
        });
    }

    // Roster "positional structural-type mismatch" — as for containment
    // (Q1 element-domain at interval positions; moot for acceptance here,
    // since any interval position hits the window refusal just below).
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
            return Err(SchemaError::ContainmentTypeMismatch {
                statement: id,
                position,
            });
        }
    }

    validate_side_selection(id, source, relations)?;
    validate_side_selection(id, target, relations)?;

    // The v0 interval refusal: window projections carry no interval
    // position, either side (the positional type match above makes the
    // sides' interval positions identical, so one scan suffices).
    let positions = interval_positions(source_fields, &source.projection);
    if let Some(pos) = positions.first() {
        return Err(SchemaError::CardinalityIntervalPosition {
            statement: id,
            relation: source.relation,
            field: source.projection[*pos],
        });
    }

    // Probe-ability, the containment rule reused: Y resolves a declared
    // key of B (a closed target takes the member-set arm through the same
    // call — the closed-side mirror).
    let resolved = resolve_target_key(id, target, &target_projection, relations, descriptors)?;

    // Both sides constant: the count judgment is decidable here — per
    // ψ-selected parent axiom, the φ-selected child axioms sharing its
    // projected tuple must count inside the window
    // (`lean/Bumbledb/Schema.lean: den_closed_constant`). The cited row
    // is the parent axiom whose group fails.
    if let (Enforcement::Closed { .. }, Some(source_rows)) = (
        &resolved,
        relations[source.relation.0 as usize].extension.as_deref(),
    ) {
        let target_relation = &relations[target.relation.0 as usize];
        let target_rows = target_relation
            .extension
            .as_deref()
            .expect("the Closed enforcement arm resolves only against a closed target");
        let source_layout = &relations[source.relation.0 as usize].layout;
        let phi = compiled_checks(&source.selection, source_fields);
        let psi = compiled_checks(&target.selection, target_fields);
        for (row_idx, parent) in target_rows.iter().enumerate() {
            if !sealed_satisfies(&psi, &target_relation.layout, &parent.fact) {
                continue;
            }
            let count =
                source_rows
                    .iter()
                    .filter(|child| {
                        sealed_satisfies(&phi, source_layout, &child.fact)
                            && source.projection.iter().zip(target.projection.iter()).all(
                                |(s, t)| {
                                    field_bytes(&child.fact, source_layout, usize::from(s.0))
                                        == field_bytes(
                                            &parent.fact,
                                            &target_relation.layout,
                                            usize::from(t.0),
                                        )
                                },
                            )
                    })
                    .count();
            let count = u64::try_from(count).expect("extension row count fits u64");
            if count < lo || hi.is_some_and(|hi| count > hi) {
                return Err(SchemaError::ClosedStatementRefuted {
                    statement: id,
                    relation: target.relation,
                    row: row_idx,
                });
            }
        }
    }

    Ok(resolved)
}

/// One encodable literal's sealed canonical bytes, at its field's
/// encoding (a fixed-width interval binding seals its one-word start).
fn encoded_literal(literal: &Value, desc: bumbledb_theory::TypeDesc) -> Box<[u8]> {
    let mut bytes = Vec::with_capacity(16);
    crate::encoding::encode_literal(literal, desc, &mut bytes);
    bytes.into()
}

/// One side's σ compiled at validate: canonical bytes sealed for every
/// literal whose encoding is a pure function of the value; `str` literals
/// stay [`CompiledCheck::Interned`]/[`CompiledCheck::InternedSet`] (their
/// word is per-database dictionary state, resolved at commit). Singleton
/// bindings compile to the classic one-compare arms unchanged; disjunctive
/// bindings seal their alternatives in canonical order. The one compile
/// walk — the sealed [`ContainmentStatement::checks`] and the
/// closed-extension evaluations below both consume it.
fn compiled_checks(
    selection: &[(FieldId, LiteralSet)],
    fields: &[FieldDescriptor],
) -> Box<[CompiledCheck]> {
    selection
        .iter()
        .map(|(field, literals)| {
            let desc = fields[usize::from(field.0)].value_type.type_desc();
            match canonical_literals(literals) {
                LiteralSet::One(Value::String(raw)) => CompiledCheck::Interned {
                    field: *field,
                    text: std::str::from_utf8(&raw)
                        .expect("selection literals validated UTF-8")
                        .into(),
                },
                LiteralSet::One(literal) => CompiledCheck::Encoded {
                    field: *field,
                    bytes: encoded_literal(&literal, desc),
                },
                // A validated binding is type-homogeneous: a `str` field's set
                // is all strings, any other field's set all encodable.
                LiteralSet::Many(values) if matches!(values[0], Value::String(_)) => {
                    CompiledCheck::InternedSet {
                        field: *field,
                        texts: values
                            .iter()
                            .map(|value| {
                                let Value::String(raw) = value else {
                                    unreachable!("validated string binding is homogeneous")
                                };
                                std::str::from_utf8(raw)
                                    .expect("selection literals validated UTF-8")
                                    .into()
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

/// σ over one sealed row: per binding, a byte compare against the sealed
/// encoding (singleton) or membership among the sealed alternatives (set).
/// Closed relations refuse `str` columns, so the `Interned` arms are
/// absent for a validated closed side. Keeping the evaluator total makes
/// malformed internal data fail the predicate instead of opening a panic
/// path.
fn sealed_satisfies(checks: &[CompiledCheck], layout: &FactLayout, fact: &[u8]) -> bool {
    checks.iter().all(|check| match check {
        CompiledCheck::Encoded { field, bytes } => {
            field_bytes(fact, layout, usize::from(field.0)) == &bytes[..]
        }
        CompiledCheck::EncodedSet {
            field,
            alternatives,
        } => {
            let actual = field_bytes(fact, layout, usize::from(field.0));
            alternatives.iter().any(|bytes| actual == &bytes[..])
        }
        CompiledCheck::Interned { .. } | CompiledCheck::InternedSet { .. } => false,
    })
}

/// One u64 field decoded off a sealed row's canonical bytes (big-endian,
/// order-preserving — `docs/architecture/10-data-model.md`).
fn decoded_word(layout: &FactLayout, field: FieldId, fact: &[u8]) -> u64 {
    u64::from_be_bytes(field_word_bytes(fact, layout, usize::from(field.0)))
}

/// Roster "unknown relation … ids": the relation for a statement-named id.
fn known_relation(
    id: StatementId,
    relation: RelationId,
    relations: &[Relation],
) -> Result<&Relation, SchemaError> {
    relations
        .get(relation.0 as usize)
        .ok_or(SchemaError::StatementUnknownRelation {
            statement: id,
            relation,
        })
}

/// Roster "unknown … field ids" and "empty or duplicate-carrying
/// projections" for one projection.
fn validate_projection<'p>(
    id: StatementId,
    relation_id: RelationId,
    projection: &'p [FieldId],
    relation: &Relation,
) -> Result<Projection<'p>, SchemaError> {
    if projection.is_empty() {
        return Err(SchemaError::EmptyProjection {
            statement: id,
            relation: relation_id,
        });
    }
    for field in projection {
        if usize::from(field.0) >= relation.fields.len() {
            return Err(SchemaError::StatementUnknownField {
                statement: id,
                relation: relation_id,
                field: *field,
            });
        }
    }
    let fields =
        FieldSet::new(projection).map_err(|field| SchemaError::DuplicateProjectionField {
            statement: id,
            relation: relation_id,
            field,
        })?;
    Ok(Projection {
        ordered: projection,
        fields,
    })
}

/// One side's id and duplication shape: unknown relation/field ids, empty
/// or duplicate projection, duplicate selection binding (σ is a set), and
/// the literal-set canon — a `Many` binding carries at least two distinct
/// literals (an empty set selects nothing and the one-literal set is the
/// `One` spelling; both degenerate spellings are rejected so the singleton
/// arm stays the only singleton by representation).
fn validate_side_shape<'s>(
    id: StatementId,
    side: &'s Side,
    relations: &[Relation],
) -> Result<Projection<'s>, SchemaError> {
    let relation = known_relation(id, side.relation, relations)?;
    let projection = validate_projection(id, side.relation, &side.projection, relation)?;
    for (idx, (field, literals)) in side.selection.iter().enumerate() {
        if usize::from(field.0) >= relation.fields.len() {
            return Err(SchemaError::StatementUnknownField {
                statement: id,
                relation: side.relation,
                field: *field,
            });
        }
        if side.selection[..idx].iter().any(|(f, _)| f == field) {
            return Err(SchemaError::DuplicateSelectionField {
                statement: id,
                relation: side.relation,
                field: *field,
            });
        }
        if let LiteralSet::Many(values) = literals {
            if values.len() < 2 {
                return Err(SchemaError::DegenerateSelectionSet {
                    statement: id,
                    relation: side.relation,
                    field: *field,
                    len: values.len(),
                });
            }
            for (value_idx, value) in values.iter().enumerate() {
                if values[..value_idx]
                    .iter()
                    .any(|earlier| literal_cmp(earlier, value) == std::cmp::Ordering::Equal)
                {
                    return Err(SchemaError::DuplicateSelectionLiteral {
                        statement: id,
                        relation: side.relation,
                        field: *field,
                    });
                }
            }
        }
    }
    Ok(projection)
}

/// One side's selection semantics: roster "a selected field also projected"
/// (a constant column — write the statement you mean), then the literal
/// checks against each selected field's structural type.
fn validate_side_selection(
    id: StatementId,
    side: &Side,
    relations: &[Relation],
) -> Result<(), SchemaError> {
    let relation = &relations[side.relation.0 as usize];
    for (field, _) in &side.selection {
        if side.projection.contains(field) {
            return Err(SchemaError::SelectedFieldProjected {
                statement: id,
                relation: side.relation,
                field: *field,
            });
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

/// Roster "selection literal type mismatch (including out-of-range enum
/// ordinals and non-UTF-8 string literals)" — the one shared
/// [`value_matches`] check, so the σ rules cannot drift from the
/// query-literal and dynamic-fact boundaries. Interval literals already
/// carry the checked [`crate::Interval`] representation.
fn validate_selection_literal(
    id: StatementId,
    relation: RelationId,
    field: FieldId,
    value_type: &ValueType,
    literal: &Value,
) -> Result<(), SchemaError> {
    value_matches(literal, value_type).map_err(|mismatch| match mismatch {
        ValueMismatch::Type => SchemaError::SelectionLiteralTypeMismatch {
            statement: id,
            relation,
            field,
        },
        ValueMismatch::Utf8 => SchemaError::SelectionLiteralNotUtf8 {
            statement: id,
            relation,
            field,
        },
    })
}

/// Target-key resolution and the pointwise gate
/// (`docs/architecture/30-dependencies.md` § the acceptance gate): the
/// target projection, as a set, must equal the field set of some
/// `Functionality` statement on the target relation — probe-ability, one
/// determinant get answers "is this tuple present". Unambiguous because duplicate
/// field sets are rejected by [`SchemaError::DuplicateFunctionality`].
fn resolve_target_key(
    id: StatementId,
    target: &Side,
    target_projection: &Projection<'_>,
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
) -> Result<Enforcement, SchemaError> {
    let target_relation = &relations[target.relation.0 as usize];

    // The compiled-subset branch (`docs/architecture/30-dependencies.md`):
    // a closed target is stage-1-known, so there is no key search, no
    // permutation, and no determinant-width concern — the enforcement plan is
    // the answer set itself. The handle id is the one probe-able identity
    // of a closed relation (the auto-key `R(id) -> R`), so the target
    // projection must be exactly the synthetic id; ψ folds against the
    // sealed extension here and never exists at commit.
    if let Some(rows) = target_relation.extension.as_deref() {
        if target.projection.len() != 1 || target.projection[0] != FieldId(0) {
            return Err(missing_target_key(id, target, descriptors, false));
        }
        return Ok(Enforcement::Closed {
            members: compile_member_set(target_relation, target, rows),
        });
    }

    let target_fields = &target_relation.fields;
    let positions = interval_positions(target_fields, &target.projection);

    // Pointwise gate, "exactly one interval position": no key can carry
    // two intervals (the FD gate rejects them), so with two or more there
    // is no pointwise key to resolve — reject without searching.
    if positions.len() > 1 {
        return Err(missing_target_key(id, target, descriptors, true));
    }
    let interval_position = positions.first().copied();

    let want = target_projection.fields();
    let found = descriptors
        .iter()
        .enumerate()
        .find_map(|(index, descriptor)| match descriptor {
            StatementDescriptor::Functionality {
                relation,
                projection,
            } if *relation == target.relation
                && FieldSet::new(projection).is_ok_and(|set| &set == want) =>
            {
                Some((index, projection.as_ref()))
            }
            StatementDescriptor::Functionality { .. }
            | StatementDescriptor::Containment { .. }
            | StatementDescriptor::Cardinality { .. } => None,
        });

    // Roster "IND whose target projection matches no key of the target
    // (or, with an interval position, no pointwise key carrying it)".
    let Some((key_idx, key_projection)) = found else {
        return Err(missing_target_key(
            id,
            target,
            descriptors,
            interval_position.is_some(),
        ));
    };
    // Set equality means the resolved key carries the interval field, and
    // the key's own FD gate forces it last — the key *is* pointwise; the
    // gate's "key carries its interval" demand is discharged by
    // construction, not re-checked.

    let key_permutation = target_projection
        .ordered()
        .iter()
        .map(|field| {
            let determinant_pos = key_projection
                .iter()
                .position(|k| k == field)
                .expect("set-equal key contains every projected field");
            u16::try_from(determinant_pos).expect("field count fits u16")
        })
        .collect();

    let target_key = KeyId(
        u16::try_from(
            descriptors[..key_idx]
                .iter()
                .filter(|descriptor| {
                    matches!(descriptor, StatementDescriptor::Functionality { .. })
                })
                .count(),
        )
        .expect("statement count fits u16"),
    );

    if interval_position.is_some() {
        let FunctionalityEvidence::Pointwise(disjoint) = validate_functionality(
            statement_id(key_idx),
            target.relation,
            key_projection,
            relations,
            descriptors,
        )?
        else {
            unreachable!("a set-equal interval projection resolves to a pointwise key")
        };
        Ok(Enforcement::IntervalCoverage {
            target_key,
            key_permutation,
            disjoint,
        })
    } else {
        Ok(Enforcement::ScalarProbe {
            target_key,
            key_permutation,
        })
    }
}

/// Owned evidence for an exact-target-key rejection. Key ids follow the
/// functionality-only typed arena order, exactly as successful sealing.
fn target_key_candidates(
    target: RelationId,
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
                });
            }
        }
    }
    available.into_boxed_slice()
}

fn missing_target_key(
    statement: StatementId,
    side: &Side,
    descriptors: &[StatementDescriptor],
    pointwise: bool,
) -> SchemaError {
    let target = side.relation;
    let projection = side.projection.clone();
    let available = target_key_candidates(target, descriptors);
    if pointwise {
        SchemaError::NoPointwiseTargetKey {
            statement,
            target,
            projection,
            available,
        }
    } else {
        SchemaError::NoMatchingTargetKey {
            statement,
            target,
            projection,
            available,
        }
    }
}

/// Compiles ψ over one sealed extension into its typed axiom set. The
/// extension passed validation before statement resolution, so every
/// declaration index is below [`super::MAX_EXTENSION_ROWS`].
fn compile_member_set(target: &Relation, side: &Side, rows: &[super::SealedRow]) -> MemberSet {
    let psi = compiled_checks(&side.selection, &target.fields);
    let mut members = MemberSet::empty();
    for (idx, row) in rows.iter().enumerate() {
        if sealed_satisfies(&psi, &target.layout, &row.fact) {
            let index = AxiomIndex(
                u16::try_from(idx).expect("the validated extension cap is below u16::MAX"),
            );
            members.insert(index);
        }
    }
    members
}

/// One relation's derived column count, for the pre-pass cap in
/// [`SchemaDescriptor::validate`]: the image's column index is u16
/// (`crate::image::ColumnSpan`, `column_spans`), so the count is capped
/// as a typed rejection ([`SchemaError::RelationTooManyColumns`]) —
/// never discovered at image-build time. An interval field spans two
/// word columns, a `bytes<N>` field its `⌈N/8⌉` — never counted below
/// one: `bytes<0>` is invalid, but its width rejection runs only after
/// the u16 field ids are minted, so the cap must be a true lower bound
/// on any legal repair of the declaration. A closed relation's
/// synthetic id contributes its word column. With every field at least
/// one column, the cap also keeps every `FieldId` within u16.
fn derived_columns(decl: &RelationDescriptor) -> usize {
    usize::from(decl.extension.is_some())
        + decl
            .fields
            .iter()
            .map(|field| match field.value_type {
                ValueType::Interval { .. } => 2,
                ValueType::FixedBytes { len } => crate::encoding::fixed_bytes_words(len).max(1),
                _ => 1,
            })
            .sum::<usize>()
}

/// One relation: field checks (duplicate names, enum shape, fresh typing,
/// the closed-relation column roster), the extension roster for a closed
/// relation, then the sealed [`Relation`]; the caller fills the statement
/// indices from the materialized statement list.
fn validate_relation(
    rel_id: RelationId,
    decl: RelationDescriptor,
) -> Result<Relation, SchemaError> {
    let RelationDescriptor {
        name,
        fields: declared,
        extension,
    } = decl;

    // A closed relation's sealed field list opens with the synthetic
    // (`id`, U64) field — the handle's declaration index — so determinants,
    // statements, and queries address it uniformly at `FieldId(0)`. The
    // macro (the emission) never lets the user declare it; a hand-built
    // descriptor declaring its own `id` collides here
    // ([`SchemaError::DuplicateFieldName`]).
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
            // The bytes<N> width gate: N ∈ 1..=64 — 64 bytes = 8 words =
            // two cache lines of key material; 0 denotes nothing
            // (`docs/architecture/10-data-model.md`).
            if len == 0 || usize::from(len) > crate::encoding::MAX_FIXED_BYTES {
                return Err(SchemaError::FixedBytesWidthOutOfRange {
                    relation: rel_id,
                    field: field_id,
                    len,
                });
            }
        }
        if let ValueType::Interval {
            width: Some(width), ..
        } = field.value_type
        {
            // The interval<E, w> width gate: w ≥ 1 (zero points denote
            // nothing) and w ≤ u64::MAX − 1 (at w = u64::MAX no start
            // satisfies the Q2 bound in either element domain — an empty
            // type is a relation no fact can ever inhabit).
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
        // The closed-relation column roster: intrinsic columns are value
        // types only (`docs/architecture/10-data-model.md`, the
        // intrinsic-vs-policy law). `str` is refused — the handle IS the
        // label, and interned columns on a virtual relation would force
        // dictionary writes at open; `fresh` is refused — identity is the
        // handle, and axioms are never minted. (A vocabulary column needs
        // no refusal anymore: a reference to a closed relation is a plain
        // u64 column plus a declared containment, and no other vocabulary
        // type exists.)
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

    let layout = FactLayout::new(
        &fields
            .iter()
            .map(|f| f.value_type.type_desc())
            .collect::<Vec<_>>(),
    );

    let extension = match extension {
        None => None,
        Some(rows) => Some(validate_extension(rel_id, &fields, &layout, &rows)?),
    };

    Ok(Relation {
        name,
        fields: fields.into_boxed_slice(),
        layout,
        extension,
        keys: Box::new([]),
        outgoing: Box::new([]),
        window_sources: Box::new([]),
        window_targets: Box::new([]),
    })
}

/// The extension roster (`docs/architecture/10-data-model.md` § closed
/// relations): ground axioms validated through the one shared
/// [`value_matches`] check and canonically encoded ONCE — each sealed row
/// carries its full fact bytes (synthetic id ‖ intrinsic values), never
/// re-encoded after validate (the staging law applied to the feature
/// itself). `fields` is the sealed list, synthetic id first.
fn validate_extension(
    rel_id: RelationId,
    fields: &[FieldDescriptor],
    layout: &FactLayout,
    rows: &[super::Row],
) -> Result<Box<[super::SealedRow]>, SchemaError> {
    // A closed relation with no rows is a vocabulary of nothing — write
    // no relation.
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
                row: row_idx,
                expected: columns,
                supplied: row.values.len(),
            });
        }
        let mut fact = Vec::with_capacity(layout.fact_width());
        fact.extend_from_slice(&crate::encoding::encode_u64(
            u64::try_from(row_idx).expect("row count fits u64"),
        ));
        for (value, (field_idx, field)) in row.values.iter().zip(fields.iter().enumerate().skip(1))
        {
            let field_id = FieldId(u16::try_from(field_idx).expect("field count fits u16"));
            value_matches(value, &field.value_type).map_err(|mismatch| match mismatch {
                // `str` columns are refused above, so Utf8 cannot arise;
                // the match stays total and maps malformed internal data
                // to the ordinary type-mismatch error.
                ValueMismatch::Type | ValueMismatch::Utf8 => {
                    SchemaError::ExtensionValueTypeMismatch {
                        relation: rel_id,
                        row: row_idx,
                        field: field_id,
                    }
                }
            })?;
            // The ray refusal (`docs/architecture/10-data-model.md`): `[s, ∞)`
            // says the theory's constant is still running, and a
            // still-running span is policy, not an intrinsic property —
            // the witnessed write that eventually closes it needs an
            // ordinary relation. Rays stay honest values everywhere else.
            let is_ray = match value {
                Value::IntervalU64(interval) => interval.is_ray(),
                Value::IntervalI64(interval) => interval.is_ray(),
                _ => false,
            };
            if is_ray {
                return Err(SchemaError::ExtensionIntervalRay {
                    relation: rel_id,
                    row: row_idx,
                    field: field_id,
                });
            }
            // Total here: String and enums (refused columns) and AllenMask
            // (no field type) all fail `value_matches` before reaching the
            // encoder.
            crate::encoding::encode_literal(value, field.value_type.type_desc(), &mut fact);
        }
        debug_assert_eq!(fact.len(), layout.fact_width());
        sealed.push(super::SealedRow {
            handle: row.handle.clone(),
            fact: fact.into_boxed_slice(),
        });
    }
    Ok(sealed.into_boxed_slice())
}
