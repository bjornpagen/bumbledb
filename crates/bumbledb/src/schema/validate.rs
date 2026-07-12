//! Declaration validation: the boundary that turns a [`SchemaDescriptor`]
//! into the sealed [`Schema`] witness.
//!
//! Field checks first, then the statement roster and acceptance gate of
//! `docs/architecture/30-dependencies.md` — exhaustive, one distinct
//! [`SchemaError`] per roster line (the variant doc comments carry the
//! citations). Every accepted statement leaves with its [`Resolved`]
//! enforcement plan computed; downstream trusts it without re-checking.
//!
//! The roster's "FD with selection" and "non-key FD form" lines have no
//! checks here: [`StatementDescriptor::Functionality`] carries neither a
//! selection nor a Y side, so both shapes are unrepresentable rather than
//! rejected.

use super::{
    closed_member, value_matches, CompiledCheck, CompiledSides, FactLayout, FieldDescriptor,
    FieldId, Generation, Relation, RelationDescriptor, RelationId, Resolved, Schema,
    SchemaDescriptor, Side, Statement, StatementDescriptor, StatementId, ValueMismatch, ValueType,
};
use crate::encoding::field_bytes;
use crate::error::SchemaError;
use crate::storage::keys::MAX_GUARD_WIDTH;
use crate::value::Value;

impl SchemaDescriptor {
    /// Validates the declaration into the sealed [`Schema`] witness.
    ///
    /// # Errors
    ///
    /// A distinct [`SchemaError`] per illegal shape — the field checks and
    /// the full statement roster; see the variant list.
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations: declaration counts exceeding
    /// the id widths (2³² relations, 2¹⁶ fields per relation, 2¹⁶
    /// statements).
    pub fn validate(self) -> Result<Schema, SchemaError> {
        let descriptors = self.materialized_statements();

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

        // The statement roster, in materialized order. Duplicate checks
        // look backward (earlier statements are already validated);
        // containment target-key resolution looks at the whole list (a key
        // may be declared after the containment that probes it).
        let mut normalized: Vec<StatementDescriptor> = Vec::with_capacity(descriptors.len());
        let mut resolutions: Vec<Resolved> = Vec::with_capacity(descriptors.len());
        for (idx, descriptor) in descriptors.iter().enumerate() {
            let id = statement_id(idx);
            let resolved = match descriptor {
                StatementDescriptor::Functionality {
                    relation,
                    projection,
                } => validate_functionality(id, *relation, projection, &relations, &descriptors)?,
                StatementDescriptor::Containment { source, target } => {
                    validate_containment(id, source, target, &relations, &descriptors)?
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
            resolutions.push(resolved);
        }

        // Per-relation statement indices, derived from the materialized
        // list — safe to index now, every relation id is validated.
        let mut keys: Vec<Vec<StatementId>> = vec![Vec::new(); relations.len()];
        let mut outgoing: Vec<Vec<StatementId>> = vec![Vec::new(); relations.len()];
        for (idx, descriptor) in descriptors.iter().enumerate() {
            let id = statement_id(idx);
            match descriptor {
                StatementDescriptor::Functionality { relation, .. } => {
                    keys[relation.0 as usize].push(id);
                }
                StatementDescriptor::Containment { source, .. } => {
                    outgoing[source.relation.0 as usize].push(id);
                }
            }
        }
        for ((relation, keys), outgoing) in relations.iter_mut().zip(keys).zip(outgoing) {
            relation.keys = keys.into_boxed_slice();
            relation.outgoing = outgoing.into_boxed_slice();
        }

        // `target_key -> dependents`: the target-side reverse-edge check
        // set (`docs/architecture/30-dependencies.md` § enforcement).
        let mut dependents: Vec<Vec<StatementId>> = vec![Vec::new(); resolutions.len()];
        for (idx, resolved) in resolutions.iter().enumerate() {
            if let Resolved::Containment { target_key, .. } = resolved {
                dependents[usize::from(target_key.0)].push(statement_id(idx));
            }
        }

        // The `==` pairing, sealed as a fact of the declaration
        // ([`Statement::mirror`]): n² over ≤ 2¹⁶ statements, in practice
        // tens — computed once here, read everywhere after.
        let mirrors: Vec<Option<StatementId>> = (0..descriptors.len())
            .map(|idx| mirror_of(&descriptors, idx))
            .collect();

        // σ literals compile once, here — the commit path consumes sealed
        // bytes and resolves only interned text (the staging law).
        let statements = descriptors
            .into_iter()
            .zip(resolutions)
            .zip(mirrors)
            .map(|((descriptor, resolved), mirror)| {
                let checks = match &descriptor {
                    StatementDescriptor::Containment { source, target } => Some(CompiledSides {
                        source: compiled_checks(&source.selection),
                        target: compiled_checks(&target.selection),
                    }),
                    StatementDescriptor::Functionality { .. } => None,
                };
                Statement {
                    descriptor,
                    resolved,
                    checks,
                    mirror,
                }
            })
            .collect();

        Ok(Schema {
            relations: relations.into_boxed_slice(),
            statements,
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

/// The materialized-order [`StatementId`] for a list index (validation
/// panics before 2¹⁶ statements are exceeded).
fn statement_id(index: usize) -> StatementId {
    StatementId(u16::try_from(index).expect("statement count fits u16"))
}

/// A projection as its sorted field set — FD identity is the field *set*
/// (the duplicate-FD rule and target-key resolution both match on it).
fn field_set(projection: &[FieldId]) -> Vec<FieldId> {
    let mut set = projection.to_vec();
    set.sort_unstable();
    set
}

/// The projection positions holding interval-typed fields — the one scan
/// behind the FD interval gate and the containment pointwise gate.
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

/// The descriptor with each selection sorted by [`FieldId`] — σ is a set of
/// bindings, so its written order is not identity (roster "duplicate
/// statements (identical normalized sides and form)").
fn normalize(descriptor: &StatementDescriptor) -> StatementDescriptor {
    fn side(side: &Side) -> Side {
        let mut selection = side.selection.to_vec();
        selection.sort_by_key(|(field, _)| *field);
        Side {
            relation: side.relation,
            projection: side.projection.clone(),
            selection: selection.into_boxed_slice(),
        }
    }
    match descriptor {
        StatementDescriptor::Functionality { .. } => descriptor.clone(),
        StatementDescriptor::Containment { source, target } => StatementDescriptor::Containment {
            source: side(source),
            target: side(target),
        },
    }
}

/// Roster "FD …" lines: `R(X) -> R` under the acceptance gate. Returns the
/// key's [`Resolved::Functionality`] (its interval position, if pointwise).
fn validate_functionality(
    id: StatementId,
    relation_id: RelationId,
    projection: &[FieldId],
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
) -> Result<Resolved, SchemaError> {
    let relation = known_relation(id, relation_id, relations)?;
    validate_projection(id, relation_id, projection, relation)?;

    // Roster ">1 interval position" and "interval not in final position":
    // the neighbor probe needs the scalar prefix as its group; two interval
    // positions would be 2-D exclusion, which the ordered guard cannot
    // answer.
    let positions = interval_positions(&relation.fields, projection);
    if positions.len() > 1 {
        return Err(SchemaError::FunctionalityMultipleIntervals {
            statement: id,
            relation: relation_id,
            field: projection[positions[1]],
        });
    }
    let interval_position = positions.first().copied();
    if let Some(pos) = interval_position {
        if pos != projection.len() - 1 {
            return Err(SchemaError::FunctionalityIntervalNotLast {
                statement: id,
                relation: relation_id,
                field: projection[pos],
            });
        }
    }

    // Roster "duplicate statements", FD form: one field *set* per relation
    // — a second FD over the same set (any order) asserts the same
    // judgment, so its guard is pure write amplification, and rejecting it
    // is what makes containment target-key resolution unambiguous.
    let this_set = field_set(projection);
    for (idx, earlier) in descriptors[..usize::from(id.0)].iter().enumerate() {
        if let StatementDescriptor::Functionality {
            relation: r,
            projection: p,
        } = earlier
        {
            if *r == relation_id && field_set(p) == this_set {
                return Err(SchemaError::DuplicateFunctionality {
                    statement: id,
                    earlier: statement_id(idx),
                });
            }
        }
    }

    // Roster "guard width overflow": Σ field widths (intervals count 16)
    // must fit `MAX_GUARD_WIDTH` — rejected at declaration, never
    // discovered at write time.
    let width: usize = projection
        .iter()
        .map(|field| {
            relation.fields[usize::from(field.0)]
                .value_type
                .type_desc()
                .width()
        })
        .sum();
    if width > MAX_GUARD_WIDTH {
        return Err(SchemaError::GuardKeyTooWide {
            statement: id,
            width,
        });
    }

    // A key on a closed relation is judged here, once: the axioms ARE the
    // final state (no commit ever touches the relation), so a colliding
    // pair refutes the statement now or never. Scalar keys collide on
    // equal projected bytes; a pointwise key collides when the scalar
    // prefix agrees and the intervals share a point — the ordered-neighbor
    // probe's judgment, run over ≤256 sealed rows instead of a guard.
    if let Some(rows) = relation.extension.as_deref() {
        let layout = &relation.layout;
        let scalar_len = projection.len() - usize::from(interval_position.is_some());
        for (row_idx, row) in rows.iter().enumerate() {
            for earlier in &rows[..row_idx] {
                let scalars_agree = projection[..scalar_len].iter().all(|field| {
                    let idx = usize::from(field.0);
                    field_bytes(&row.fact, layout, idx) == field_bytes(&earlier.fact, layout, idx)
                });
                if !scalars_agree {
                    continue;
                }
                let collide = match interval_position {
                    None => true,
                    Some(pos) => {
                        let idx = usize::from(projection[pos].0);
                        let a = field_bytes(&row.fact, layout, idx);
                        let b = field_bytes(&earlier.fact, layout, idx);
                        // Half-open `[s, e)` intersection on the 8-byte
                        // order-preserving halves.
                        a[..8] < b[8..] && b[..8] < a[8..]
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

    Ok(Resolved::Functionality {
        pointwise: interval_position.is_some(),
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
) -> Result<Resolved, SchemaError> {
    validate_side_shape(id, source, relations)?;
    validate_side_shape(id, target, relations)?;

    // Roster "arity mismatch between sides": |X| = |Y|.
    if source.projection.len() != target.projection.len() {
        return Err(SchemaError::ContainmentArityMismatch {
            statement: id,
            source: source.projection.len(),
            target: target.projection.len(),
        });
    }

    // Roster "positional structural-type mismatch" — derive-eq on
    // `ValueType`, which also covers the called-out interval-against-scalar
    // case (`docs/architecture/10-data-model.md` structural equality).
    let source_fields = &relations[source.relation.0 as usize].fields;
    let target_fields = &relations[target.relation.0 as usize].fields;
    for (position, (s, t)) in source
        .projection
        .iter()
        .zip(target.projection.iter())
        .enumerate()
    {
        if source_fields[usize::from(s.0)].value_type != target_fields[usize::from(t.0)].value_type
        {
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
    // (`docs/prd-comptime/04-compiled-subsets.md`; trigger: a census
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

    let resolved = resolve_target_key(id, target, relations, descriptors)?;

    // Both sides constant: the judgment is decidable here, and a theory
    // whose axioms refute its own statement has no model to commit — the
    // source extension's φ-rows must all sit inside the compiled member
    // set (a closed source under an *ordinary* target stays commit-judged:
    // the target can shrink).
    if let (Resolved::ClosedContainment { members }, Some(rows)) = (
        &resolved,
        relations[source.relation.0 as usize].extension.as_deref(),
    ) {
        let layout = &relations[source.relation.0 as usize].layout;
        let phi = compiled_checks(&source.selection);
        for (row_idx, row) in rows.iter().enumerate() {
            if !sealed_satisfies(&phi, layout, &row.fact) {
                continue;
            }
            let word = decoded_word(layout, source.projection[0], &row.fact);
            if !closed_member(members, word) {
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

/// One side's σ compiled at validate: canonical bytes sealed for every
/// literal whose encoding is a pure function of the value; `str` literals
/// stay [`CompiledCheck::Interned`] (their word is per-database dictionary
/// state, resolved at commit). The one compile walk — the sealed
/// [`Statement::checks`] and the closed-extension evaluations below both
/// consume it.
fn compiled_checks(selection: &[(FieldId, Value)]) -> Box<[CompiledCheck]> {
    selection
        .iter()
        .map(|(field, literal)| match literal {
            Value::String(raw) => CompiledCheck::Interned {
                field: *field,
                text: std::str::from_utf8(raw)
                    .expect("selection literals validated UTF-8")
                    .into(),
            },
            literal => {
                let mut bytes = Vec::with_capacity(16);
                crate::encoding::encode_literal(literal, &mut bytes);
                CompiledCheck::Encoded {
                    field: *field,
                    bytes: bytes.into(),
                }
            }
        })
        .collect()
}

/// σ over one sealed row: one byte compare per selected field. Total over
/// `Encoded` only — closed relations refuse `str` columns, so a closed
/// side's compiled checks never carry `Interned`.
fn sealed_satisfies(checks: &[CompiledCheck], layout: &FactLayout, fact: &[u8]) -> bool {
    checks.iter().all(|check| match check {
        CompiledCheck::Encoded { field, bytes } => {
            field_bytes(fact, layout, usize::from(field.0)) == &bytes[..]
        }
        CompiledCheck::Interned { .. } => {
            unreachable!("closed relations refuse str columns")
        }
    })
}

/// One u64 field decoded off a sealed row's canonical bytes (big-endian,
/// order-preserving — `docs/architecture/10-data-model.md`).
fn decoded_word(layout: &FactLayout, field: FieldId, fact: &[u8]) -> u64 {
    u64::from_be_bytes(
        field_bytes(fact, layout, usize::from(field.0))
            .try_into()
            .expect("u64 field is 8 bytes"),
    )
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
fn validate_projection(
    id: StatementId,
    relation_id: RelationId,
    projection: &[FieldId],
    relation: &Relation,
) -> Result<(), SchemaError> {
    if projection.is_empty() {
        return Err(SchemaError::EmptyProjection {
            statement: id,
            relation: relation_id,
        });
    }
    for (idx, field) in projection.iter().enumerate() {
        if usize::from(field.0) >= relation.fields.len() {
            return Err(SchemaError::StatementUnknownField {
                statement: id,
                relation: relation_id,
                field: *field,
            });
        }
        if projection[..idx].contains(field) {
            return Err(SchemaError::DuplicateProjectionField {
                statement: id,
                relation: relation_id,
                field: *field,
            });
        }
    }
    Ok(())
}

/// One side's id and duplication shape: unknown relation/field ids, empty
/// or duplicate projection, duplicate selection binding (σ is a set).
fn validate_side_shape(
    id: StatementId,
    side: &Side,
    relations: &[Relation],
) -> Result<(), SchemaError> {
    let relation = known_relation(id, side.relation, relations)?;
    validate_projection(id, side.relation, &side.projection, relation)?;
    for (idx, (field, _)) in side.selection.iter().enumerate() {
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
    }
    Ok(())
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
    for (field, literal) in &side.selection {
        validate_selection_literal(
            id,
            side.relation,
            *field,
            &relation.fields[usize::from(field.0)].value_type,
            literal,
        )?;
    }
    Ok(())
}

/// Roster "selection literal type mismatch (including out-of-range enum
/// ordinals and non-UTF-8 string literals)", plus the interval bound rule
/// `start < end` (an empty interval denotes no points, and a fact never
/// denotes nothing) — the one shared [`value_matches`] check, so the σ
/// rules cannot drift from the query-literal and dynamic-fact boundaries.
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
        ValueMismatch::IntervalEmpty => SchemaError::SelectionIntervalEmpty {
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
/// guard get answers "is this tuple present". Unambiguous because duplicate
/// field sets are rejected by [`SchemaError::DuplicateFunctionality`].
fn resolve_target_key(
    id: StatementId,
    target: &Side,
    relations: &[Relation],
    descriptors: &[StatementDescriptor],
) -> Result<Resolved, SchemaError> {
    let target_relation = &relations[target.relation.0 as usize];

    // The compiled-subset branch (`docs/prd-comptime/04-compiled-subsets.md`):
    // a closed target is stage-1-known, so there is no key search, no
    // permutation, and no guard-width concern — the enforcement plan is
    // the answer set itself. The handle id is the one probe-able identity
    // of a closed relation (the auto-key `R(id) -> R`), so the target
    // projection must be exactly the synthetic id; ψ folds against the
    // sealed extension here and never exists at commit.
    if let Some(rows) = target_relation.extension.as_deref() {
        if target.projection.len() != 1 || target.projection[0] != FieldId(0) {
            return Err(SchemaError::NoMatchingTargetKey {
                statement: id,
                relation: target.relation,
            });
        }
        let psi = compiled_checks(&target.selection);
        let mut members = [0u64; 4];
        for (idx, row) in rows.iter().enumerate() {
            if sealed_satisfies(&psi, &target_relation.layout, &row.fact) {
                members[idx / 64] |= 1 << (idx % 64);
            }
        }
        return Ok(Resolved::ClosedContainment { members });
    }

    let target_fields = &target_relation.fields;
    let positions = interval_positions(target_fields, &target.projection);

    // Pointwise gate, "exactly one interval position": no key can carry
    // two intervals (the FD gate rejects them), so with two or more there
    // is no pointwise key to resolve — reject without searching.
    if positions.len() > 1 {
        return Err(SchemaError::NoPointwiseTargetKey {
            statement: id,
            relation: target.relation,
        });
    }
    let interval_position = positions.first().copied();

    let want = field_set(&target.projection);
    let found = descriptors.iter().enumerate().find(|(_, descriptor)| {
        if let StatementDescriptor::Functionality {
            relation,
            projection,
        } = descriptor
        {
            *relation == target.relation && field_set(projection) == want
        } else {
            false
        }
    });

    // Roster "IND whose target projection matches no key of the target
    // (or, with an interval position, no pointwise key carrying it)".
    let Some((key_idx, key)) = found else {
        return Err(if interval_position.is_some() {
            SchemaError::NoPointwiseTargetKey {
                statement: id,
                relation: target.relation,
            }
        } else {
            SchemaError::NoMatchingTargetKey {
                statement: id,
                relation: target.relation,
            }
        });
    };
    // Set equality means the resolved key carries the interval field, and
    // the key's own FD gate forces it last — the key *is* pointwise; the
    // gate's "key carries its interval" demand is discharged by
    // construction, not re-checked.

    let StatementDescriptor::Functionality {
        projection: key_projection,
        ..
    } = key
    else {
        unreachable!("resolution matched a Functionality");
    };
    let key_permutation = target
        .projection
        .iter()
        .map(|field| {
            let guard_pos = key_projection
                .iter()
                .position(|k| k == field)
                .expect("set-equal key contains every projected field");
            u16::try_from(guard_pos).expect("field count fits u16")
        })
        .collect();

    Ok(Resolved::Containment {
        target_key: statement_id(key_idx),
        key_permutation,
        coverage: interval_position.is_some(),
    })
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
    // (`id`, U64) field — the handle's declaration index — so guards,
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
                // The constructor law holds for axioms too: a malformed
                // ground axiom is a schema error, not corruption.
                ValueMismatch::IntervalEmpty => SchemaError::ExtensionIntervalEmpty {
                    relation: rel_id,
                    row: row_idx,
                    field: field_id,
                },
                // `str` columns are refused above, so Utf8 is
                // unreachable — kept total, not clever.
                ValueMismatch::Type | ValueMismatch::Utf8 => {
                    SchemaError::ExtensionValueTypeMismatch {
                        relation: rel_id,
                        row: row_idx,
                        field: field_id,
                    }
                }
            })?;
            // The ray refusal (`docs/prd-comptime/README.md`): `[s, ∞)`
            // says the theory's constant is still running, and a
            // still-running span is policy, not an intrinsic property —
            // the witnessed write that eventually closes it needs an
            // ordinary relation. Rays stay honest values everywhere else.
            let is_ray = match value {
                Value::IntervalU64(_, end) => *end == crate::Interval::<u64>::MAX_END,
                Value::IntervalI64(_, end) => *end == crate::Interval::<i64>::MAX_END,
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
            crate::encoding::encode_literal(value, &mut fact);
        }
        debug_assert_eq!(fact.len(), layout.fact_width());
        sealed.push(super::SealedRow {
            handle: row.handle.clone(),
            fact: fact.into_boxed_slice(),
        });
    }
    Ok(sealed.into_boxed_slice())
}
