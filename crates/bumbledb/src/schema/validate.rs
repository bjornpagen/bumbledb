//! Declaration validation: the boundary that turns a [`SchemaDescriptor`]
//! into the sealed [`Schema`] witness.
//!
//! **PRD 03's site.** Statement validation — the roster and the acceptance
//! gate of `docs/architecture/30-dependencies.md` — is not implemented yet.
//! This placeholder keeps the field-level checks, materializes the statement
//! list ([`SchemaDescriptor::materialized_statements`] owns the ordering
//! rule), derives the per-relation statement indices, and seals. Every
//! [`Resolved`] it attaches is a placeholder; PRD 03 computes the real
//! enforcement data and nothing downstream reads it until then.

use super::{
    FactLayout, FieldDescriptor, FieldId, Generation, Relation, RelationDescriptor, RelationId,
    Resolved, Schema, SchemaDescriptor, Statement, StatementDescriptor, StatementId, ValueType,
};
use crate::error::SchemaError;

impl SchemaDescriptor {
    /// Validates the declaration into the sealed [`Schema`] witness.
    ///
    /// # Errors
    ///
    /// A distinct [`SchemaError`] per illegal shape; see the variant list.
    /// The statement roster (PRD 03) is not checked yet.
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

        // Per-relation statement indices, derived from the materialized list.
        let mut keys: Vec<Vec<StatementId>> = vec![Vec::new(); relations.len()];
        let mut outgoing: Vec<Vec<StatementId>> = vec![Vec::new(); relations.len()];
        let mut incoming: Vec<Vec<StatementId>> = vec![Vec::new(); relations.len()];
        for (idx, descriptor) in descriptors.iter().enumerate() {
            let id = StatementId(u16::try_from(idx).expect("statement count fits u16"));
            match descriptor {
                StatementDescriptor::Functionality { relation, .. } => {
                    keys[relation.0 as usize].push(id);
                }
                StatementDescriptor::Containment { source, target } => {
                    outgoing[source.relation.0 as usize].push(id);
                    incoming[target.relation.0 as usize].push(id);
                }
            }
        }
        for (((relation, keys), outgoing), incoming) in
            relations.iter_mut().zip(keys).zip(outgoing).zip(incoming)
        {
            relation.keys = keys.into_boxed_slice();
            relation.outgoing = outgoing.into_boxed_slice();
            relation.incoming = incoming.into_boxed_slice();
        }

        // PRD 03: the statement roster and the acceptance gate run here and
        // compute the real `Resolved` per statement. Placeholder resolution
        // until then — nothing downstream reads it.
        let statements = descriptors
            .into_iter()
            .map(|descriptor| {
                let resolved = match &descriptor {
                    StatementDescriptor::Functionality { .. } => Resolved::Functionality {
                        interval_position: None,
                    },
                    StatementDescriptor::Containment { .. } => Resolved::Containment {
                        target_key: StatementId(0),
                        key_permutation: Box::new([]),
                        interval_position: None,
                    },
                };
                Statement {
                    descriptor,
                    resolved,
                }
            })
            .collect();

        Ok(Schema {
            relations: relations.into_boxed_slice(),
            statements,
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

/// Validates one relation's fields and seals it; the caller fills the
/// statement indices from the materialized statement list.
fn validate_relation(
    rel_id: RelationId,
    decl: RelationDescriptor,
) -> Result<Relation, SchemaError> {
    let RelationDescriptor { name, fields } = decl;

    validate_fields(rel_id, &fields)?;

    let layout = FactLayout::new(
        &fields
            .iter()
            .map(|f| f.value_type.type_desc())
            .collect::<Vec<_>>(),
    );

    Ok(Relation {
        name,
        fields: fields.into_boxed_slice(),
        layout,
        keys: Box::new([]),
        outgoing: Box::new([]),
        incoming: Box::new([]),
    })
}
