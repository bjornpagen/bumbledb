//! Declaration validation: the boundary that turns a [`SchemaDescriptor`]
//! into the sealed [`Schema`] witness.

use super::{
    ConstraintDescriptor, ConstraintId, FactLayout, FieldDescriptor, FieldId, Generation, Relation,
    RelationDescriptor, RelationId, Schema, SchemaDescriptor, ValueType,
};
use crate::error::SchemaError;
use crate::storage::keys::MAX_GUARD_WIDTH;

impl SchemaDescriptor {
    /// Validates the declaration into the sealed [`Schema`] witness,
    /// auto-materializing one `Unique` constraint per `Serial` field (named
    /// after the field, ordinary in every way).
    ///
    /// # Errors
    ///
    /// A distinct [`SchemaError`] per illegal shape; see the variant list.
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations: declaration counts exceeding
    /// the id widths (2³² relations, 2¹⁶ fields/constraints per relation).
    pub fn validate(self) -> Result<Schema, SchemaError> {
        // Pass 1: per-relation checks that need no cross-relation knowledge,
        // and auto-unique materialization.
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

        // Pass 2: FK resolution against the fully-materialized constraint
        // lists (targets may live in any relation, including later ones).
        let mut fk_targeted: Vec<Vec<ConstraintId>> = vec![Vec::new(); relations.len()];
        for (rel_idx, relation) in relations.iter().enumerate() {
            let rel_id = RelationId(u32::try_from(rel_idx).expect("relation count fits u32"));
            for (con_idx, constraint) in relation.constraints.iter().enumerate() {
                let con_id =
                    ConstraintId(u16::try_from(con_idx).expect("constraint count fits u16"));
                let ConstraintDescriptor::ForeignKey {
                    fields,
                    target_relation,
                    target_constraint,
                    ..
                } = constraint
                else {
                    continue;
                };
                let target_rel = relations.get(target_relation.0 as usize).ok_or(
                    SchemaError::UnknownFkTargetRelation {
                        relation: rel_id,
                        constraint: con_id,
                        target: *target_relation,
                    },
                )?;
                let target = target_rel
                    .constraints
                    .get(usize::from(target_constraint.0))
                    .ok_or(SchemaError::UnknownFkTargetConstraint {
                        relation: rel_id,
                        constraint: con_id,
                        target: *target_constraint,
                    })?;
                let ConstraintDescriptor::Unique {
                    fields: target_fields,
                    ..
                } = target
                else {
                    return Err(SchemaError::FkTargetNotUnique {
                        relation: rel_id,
                        constraint: con_id,
                        target: *target_constraint,
                    });
                };
                if fields.len() != target_fields.len() {
                    return Err(SchemaError::FkArityMismatch {
                        relation: rel_id,
                        constraint: con_id,
                    });
                }
                for (position, (source_field, target_field)) in
                    fields.iter().zip(target_fields.iter()).enumerate()
                {
                    let source_type = &relation.field(*source_field).value_type;
                    let target_type = &target_rel.field(*target_field).value_type;
                    // Positional structural-type equality: one derive-powered
                    // comparison IS the compatibility rule.
                    if source_type != target_type {
                        return Err(SchemaError::FkFieldTypeMismatch {
                            relation: rel_id,
                            constraint: con_id,
                            position,
                        });
                    }
                }
                fk_targeted[target_relation.0 as usize].push(*target_constraint);
            }
        }

        for (relation, mut targeted) in relations.iter_mut().zip(fk_targeted) {
            targeted.sort_unstable();
            targeted.dedup();
            relation.fk_targeted = targeted.into_boxed_slice();
        }

        Ok(Schema {
            relations: relations.into_boxed_slice(),
        })
    }
}

/// Field checks: duplicate names, enum shape, serial typing.
fn validate_fields(rel_id: RelationId, fields: &[FieldDescriptor]) -> Result<(), SchemaError> {
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
        if field.generation == Generation::Serial && field.value_type != ValueType::U64 {
            return Err(SchemaError::SerialOnNonU64 {
                relation: rel_id,
                field: field_id,
            });
        }
    }
    Ok(())
}

/// Validates one relation's fields and constraints, materializing serial
/// auto-uniques. FK targets are resolved by the caller in a second pass.
fn validate_relation(
    rel_id: RelationId,
    decl: RelationDescriptor,
) -> Result<Relation, SchemaError> {
    let RelationDescriptor {
        name,
        fields,
        constraints: declared,
    } = decl;

    validate_fields(rel_id, &fields)?;

    // Auto-materialize a unique constraint per serial field, named after the
    // field, ahead of declared constraints (the ConstraintId numbering rule).
    let mut constraints: Vec<ConstraintDescriptor> = fields
        .iter()
        .enumerate()
        .filter(|(_, f)| f.generation == Generation::Serial)
        .map(|(idx, f)| ConstraintDescriptor::Unique {
            name: f.name.clone(),
            fields: Box::new([FieldId(u16::try_from(idx).expect("field count fits u16"))]),
        })
        .collect();
    constraints.extend(declared);

    // Constraint checks: name scoping, field validity, unique shape.
    for (idx, constraint) in constraints.iter().enumerate() {
        let con_id = ConstraintId(u16::try_from(idx).expect("constraint count fits u16"));
        if constraints[..idx]
            .iter()
            .any(|c| c.name() == constraint.name())
        {
            return Err(SchemaError::DuplicateConstraintName {
                relation: rel_id,
                name: constraint.name().into(),
            });
        }
        for &field in constraint.fields() {
            if usize::from(field.0) >= fields.len() {
                return Err(SchemaError::UnknownConstraintField {
                    relation: rel_id,
                    constraint: con_id,
                    field,
                });
            }
        }
        // A duplicated field within any constraint's list is a typo, not a
        // meaning: `unique(a, a)` guards nothing extra and `fk(a, a -> ..)`
        // references keys with equal components — reject both.
        let field_list = constraint.fields();
        for (f_idx, field) in field_list.iter().enumerate() {
            if field_list[..f_idx].contains(field) {
                return Err(SchemaError::ConstraintDuplicateField {
                    relation: rel_id,
                    constraint: con_id,
                    field: *field,
                });
            }
        }
        if let ConstraintDescriptor::Unique { fields: cf, .. } = constraint {
            if cf.is_empty() {
                return Err(SchemaError::UniqueWithoutFields {
                    relation: rel_id,
                    constraint: con_id,
                });
            }
            // Two unique constraints over one ordered field list are pure
            // write amplification: every insert/delete maintains two `U`
            // guards that can never disagree. Names are per-relation
            // scoped already; field sets are too.
            for (other_idx, other) in constraints[..idx].iter().enumerate() {
                if let ConstraintDescriptor::Unique { fields: of, .. } = other {
                    if of == cf {
                        let _ = other_idx;
                        return Err(SchemaError::DuplicateConstraintFields {
                            relation: rel_id,
                            constraint: con_id,
                        });
                    }
                }
            }
            // Guard keys are fixed-width (every type encodes 1 or 8 bytes),
            // so a constraint that would overflow LMDB's key ceiling once
            // embedded in a Restrict key is rejected here, at declaration —
            // never discovered at write time (the 40-storage doc's construction hook).
            let width: usize = cf
                .iter()
                .map(|f| fields[usize::from(f.0)].value_type.type_desc().width())
                .sum();
            if width > MAX_GUARD_WIDTH {
                return Err(SchemaError::GuardKeyTooWide {
                    relation: rel_id,
                    constraint: con_id,
                    width,
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
    let unique_constraints = constraints
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, ConstraintDescriptor::Unique { .. }))
        .map(|(idx, _)| ConstraintId(u16::try_from(idx).expect("constraint count fits u16")))
        .collect();

    Ok(Relation {
        name,
        fields: fields.into_boxed_slice(),
        constraints: constraints.into_boxed_slice(),
        layout,
        unique_constraints,
        fk_targeted: Box::new([]),
    })
}
