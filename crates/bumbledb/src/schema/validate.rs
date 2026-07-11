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
    value_matches, FactLayout, FieldDescriptor, FieldId, Generation, Relation, RelationDescriptor,
    RelationId, Resolved, Schema, SchemaDescriptor, Side, Statement, StatementDescriptor,
    StatementId, ValueMismatch, ValueType,
};
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

        let statements = descriptors
            .into_iter()
            .zip(resolutions)
            .zip(mirrors)
            .map(|((descriptor, resolved), mirror)| Statement {
                descriptor,
                resolved,
                mirror,
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

    Ok(Resolved::Functionality { interval_position })
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

    resolve_target_key(id, target, relations, descriptors)
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
        ValueMismatch::EnumOrdinal(ordinal) => SchemaError::SelectionEnumOrdinalOutOfRange {
            statement: id,
            relation,
            field,
            ordinal,
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
    let target_fields = &relations[target.relation.0 as usize].fields;
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
        interval_position,
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
        if let ValueType::Enum { variants } = &field.value_type {
            if variants.is_empty() {
                return Err(SchemaError::EnumWithoutVariants {
                    relation: rel_id,
                    field: field_id,
                });
            }
            if variants.len() > 256 {
                return Err(SchemaError::EnumTooManyVariants {
                    relation: rel_id,
                    field: field_id,
                    count: variants.len(),
                });
            }
            for (v_idx, variant) in variants.iter().enumerate() {
                if variants[..v_idx].contains(variant) {
                    return Err(SchemaError::DuplicateEnumVariant {
                        relation: rel_id,
                        field: field_id,
                        variant: variant.clone(),
                    });
                }
            }
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
        // handle, and axioms are never minted.
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
                // `str` columns are refused above, so Utf8 is unreachable;
                // an out-of-range enum ordinal does not inhabit the type.
                ValueMismatch::Type | ValueMismatch::EnumOrdinal(_) | ValueMismatch::Utf8 => {
                    SchemaError::ExtensionValueTypeMismatch {
                        relation: rel_id,
                        row: row_idx,
                        field: field_id,
                    }
                }
            })?;
            // Total here: String (refused column) and AllenMask (no field
            // type) both fail `value_matches` before reaching the encoder.
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
