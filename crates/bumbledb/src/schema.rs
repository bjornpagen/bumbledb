//! Schema descriptors, declaration validation, and the fingerprint (PRDs 02-03).
//!
//! Construction is the validation boundary (parse, don't validate): the only
//! way to obtain a [`Schema`] is [`SchemaDescriptor::validate`], and everything
//! downstream trusts the sealed witness without re-checking.

pub mod fingerprint;

use crate::encoding::{FactLayout, TypeDesc};

/// Dense relation id: the relation's index in schema declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelationId(pub u32);

/// Dense field id: the field's index in its relation's declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldId(pub u16);

/// Dense constraint id: the constraint's index in its relation's constraint
/// list — auto-materialized serial uniques first (in field declaration
/// order), then declared constraints in declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConstraintId(pub u16);

/// A structural value type: the description *is* the identity — structural
/// equality of the description is type equality, and there is no name field
/// anywhere (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueType {
    Bool,
    /// Identity is the ordered variant-name list: two fields declaring the
    /// same list are the same type, whatever the schema calls them.
    Enum {
        variants: Box<[Box<str>]>,
    },
    U64,
    I64,
    String,
    Bytes,
}

impl ValueType {
    /// The encoding-level description this type lowers to.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: an enum with more than 256
    /// variants, which schema validation makes unconstructible.
    #[must_use]
    pub fn type_desc(&self) -> TypeDesc {
        match self {
            Self::Bool => TypeDesc::Bool,
            Self::Enum { variants } => TypeDesc::Enum {
                variant_count: u16::try_from(variants.len())
                    .expect("validated schema: <=256 enum variants"),
            },
            Self::U64 => TypeDesc::U64,
            Self::I64 => TypeDesc::I64,
            Self::String => TypeDesc::String,
            Self::Bytes => TypeDesc::Bytes,
        }
    }
}

/// Field generation: a storage behavior, not a type
/// (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Generation {
    /// Ordinary field: the application supplies every value.
    None,
    /// The database mints values: monotonic per (relation, field), never
    /// re-issuing a value observable in a committed state. Must be `U64`.
    Serial,
}

/// One field: name + structural type + generation attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: Box<str>,
    pub value_type: ValueType,
    pub generation: Generation,
}

/// One declared constraint. Field lists are ordered (the order defines the
/// guard key and the FK target shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintDescriptor {
    Unique {
        name: Box<str>,
        fields: Box<[FieldId]>,
    },
    ForeignKey {
        name: Box<str>,
        fields: Box<[FieldId]>,
        target_relation: RelationId,
        /// Must name a `Unique` constraint of the target relation. Note the
        /// id numbering rule on [`ConstraintId`]: auto-materialized serial
        /// uniques come first.
        target_constraint: ConstraintId,
    },
}

impl ConstraintDescriptor {
    /// The constraint's name (scoped per relation).
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Unique { name, .. } | Self::ForeignKey { name, .. } => name,
        }
    }

    /// The constraint's ordered field list.
    #[must_use]
    pub fn fields(&self) -> &[FieldId] {
        match self {
            Self::Unique { fields, .. } | Self::ForeignKey { fields, .. } => fields,
        }
    }
}

/// One declared relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDescriptor {
    pub name: Box<str>,
    pub fields: Vec<FieldDescriptor>,
    pub constraints: Vec<ConstraintDescriptor>,
}

/// The schema as declared: input to validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDescriptor {
    pub relations: Vec<RelationDescriptor>,
}

/// A declaration error. Every illegal schema shape has a distinct variant;
/// an invalid schema is unconstructible, not flagged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    DuplicateRelationName {
        name: Box<str>,
    },
    DuplicateFieldName {
        relation: RelationId,
        name: Box<str>,
    },
    DuplicateConstraintName {
        relation: RelationId,
        name: Box<str>,
    },
    EnumWithoutVariants {
        relation: RelationId,
        field: FieldId,
    },
    EnumTooManyVariants {
        relation: RelationId,
        field: FieldId,
        count: usize,
    },
    DuplicateEnumVariant {
        relation: RelationId,
        field: FieldId,
        variant: Box<str>,
    },
    SerialOnNonU64 {
        relation: RelationId,
        field: FieldId,
    },
    UnknownConstraintField {
        relation: RelationId,
        constraint: ConstraintId,
        field: FieldId,
    },
    UniqueWithoutFields {
        relation: RelationId,
        constraint: ConstraintId,
    },
    UniqueDuplicateField {
        relation: RelationId,
        constraint: ConstraintId,
        field: FieldId,
    },
    UnknownFkTargetRelation {
        relation: RelationId,
        constraint: ConstraintId,
        target: RelationId,
    },
    UnknownFkTargetConstraint {
        relation: RelationId,
        constraint: ConstraintId,
        target: ConstraintId,
    },
    FkTargetNotUnique {
        relation: RelationId,
        constraint: ConstraintId,
        target: ConstraintId,
    },
    FkArityMismatch {
        relation: RelationId,
        constraint: ConstraintId,
    },
    FkFieldTypeMismatch {
        relation: RelationId,
        constraint: ConstraintId,
        position: usize,
    },
}

/// One relation of a validated schema.
#[derive(Debug)]
pub struct Relation {
    name: Box<str>,
    fields: Box<[FieldDescriptor]>,
    /// Auto-materialized serial uniques first, then declared constraints.
    constraints: Box<[ConstraintDescriptor]>,
    layout: FactLayout,
    /// Ids of this relation's `Unique` constraints.
    unique_constraints: Box<[ConstraintId]>,
    /// Unique constraints of *this* relation targeted by some FK anywhere in
    /// the schema — the delete-side Restrict scan set (PRD 08's reader).
    fk_targeted: Box<[ConstraintId]>,
}

impl Relation {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn fields(&self) -> &[FieldDescriptor] {
        &self.fields
    }

    #[must_use]
    pub fn field(&self, id: FieldId) -> &FieldDescriptor {
        &self.fields[usize::from(id.0)]
    }

    #[must_use]
    pub fn constraints(&self) -> &[ConstraintDescriptor] {
        &self.constraints
    }

    #[must_use]
    pub fn constraint(&self, id: ConstraintId) -> &ConstraintDescriptor {
        &self.constraints[usize::from(id.0)]
    }

    /// The relation's fact byte layout (fields in declaration order).
    #[must_use]
    pub const fn layout(&self) -> &FactLayout {
        &self.layout
    }

    /// Ids of this relation's `Unique` constraints (auto-materialized and
    /// declared alike).
    #[must_use]
    pub fn unique_constraints(&self) -> &[ConstraintId] {
        &self.unique_constraints
    }

    /// Unique constraints of this relation that some FK targets.
    #[must_use]
    pub fn fk_targeted(&self) -> &[ConstraintId] {
        &self.fk_targeted
    }
}

/// The sealed schema witness. Unconstructible except through
/// [`SchemaDescriptor::validate`]; downstream code trusts its invariants.
#[derive(Debug)]
pub struct Schema {
    relations: Box<[Relation]>,
}

impl Schema {
    #[must_use]
    pub fn relations(&self) -> &[Relation] {
        &self.relations
    }

    #[must_use]
    pub fn relation(&self, id: RelationId) -> &Relation {
        &self.relations[id.0 as usize]
    }
}

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
        if let ConstraintDescriptor::Unique { fields: cf, .. } = constraint {
            if cf.is_empty() {
                return Err(SchemaError::UniqueWithoutFields {
                    relation: rel_id,
                    constraint: con_id,
                });
            }
            for (f_idx, field) in cf.iter().enumerate() {
                if cf[..f_idx].contains(field) {
                    return Err(SchemaError::UniqueDuplicateField {
                        relation: rel_id,
                        constraint: con_id,
                        field: *field,
                    });
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
        FieldDescriptor {
            name: name.into(),
            value_type,
            generation: Generation::None,
        }
    }

    fn serial_field(name: &str) -> FieldDescriptor {
        FieldDescriptor {
            name: name.into(),
            value_type: ValueType::U64,
            generation: Generation::Serial,
        }
    }

    fn enum_type(variants: &[&str]) -> ValueType {
        ValueType::Enum {
            variants: variants.iter().map(|v| Box::from(*v)).collect(),
        }
    }

    /// Account(id serial, holder u64 -> Holder.id, status enum) + Holder(id serial, name string).
    fn ledger_slice() -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "Holder".into(),
                    fields: vec![serial_field("id"), field("name", ValueType::String)],
                    constraints: vec![],
                },
                RelationDescriptor {
                    name: "Account".into(),
                    fields: vec![
                        serial_field("id"),
                        field("holder", ValueType::U64),
                        field("status", enum_type(&["Active", "Closed"])),
                    ],
                    constraints: vec![ConstraintDescriptor::ForeignKey {
                        name: "account_holder".into(),
                        fields: Box::new([FieldId(1)]),
                        target_relation: RelationId(0),
                        // Holder's auto-unique on its serial `id` field.
                        target_constraint: ConstraintId(0),
                    }],
                },
            ],
        }
    }

    #[test]
    fn valid_schema_constructs_with_auto_uniques() {
        let schema = ledger_slice().validate().expect("valid schema");
        let holder = schema.relation(RelationId(0));
        // The serial field auto-materialized an ordinary, visible unique.
        assert_eq!(holder.constraints().len(), 1);
        assert_eq!(
            holder.constraint(ConstraintId(0)),
            &ConstraintDescriptor::Unique {
                name: "id".into(),
                fields: Box::new([FieldId(0)]),
            }
        );
        assert_eq!(holder.unique_constraints(), &[ConstraintId(0)]);
        // ...and it is FK-targeted by Account's FK (the Restrict scan set).
        assert_eq!(holder.fk_targeted(), &[ConstraintId(0)]);

        let account = schema.relation(RelationId(1));
        assert_eq!(account.constraints().len(), 2); // auto-unique + declared FK
        assert_eq!(account.fk_targeted(), &[]);
        // Layout: id 8 + holder 8 + status 1, dense.
        assert_eq!(account.layout().fact_width(), 17);
    }

    #[test]
    fn structural_enum_equality_is_the_identity() {
        // Same ordered variant list, different declaring contexts: equal type.
        assert_eq!(enum_type(&["A", "B"]), enum_type(&["A", "B"]));
        // Different order: different type (ordinal encoding differs).
        assert_ne!(enum_type(&["A", "B"]), enum_type(&["B", "A"]));
    }

    #[test]
    fn fk_may_target_structurally_equal_enum_key() {
        // FK compatibility is positional structural equality — an enum key
        // unifies iff the variant lists match exactly.
        let schema = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "T".into(),
                    fields: vec![field("kind", enum_type(&["X", "Y"]))],
                    constraints: vec![ConstraintDescriptor::Unique {
                        name: "kind".into(),
                        fields: Box::new([FieldId(0)]),
                    }],
                },
                RelationDescriptor {
                    name: "S".into(),
                    fields: vec![field("kind", enum_type(&["X", "Y"]))],
                    constraints: vec![ConstraintDescriptor::ForeignKey {
                        name: "s_kind".into(),
                        fields: Box::new([FieldId(0)]),
                        target_relation: RelationId(0),
                        target_constraint: ConstraintId(0),
                    }],
                },
            ],
        };
        schema.validate().expect("structural enums unify");
    }

    #[test]
    fn nullary_relation_constructs() {
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "Flag".into(),
                fields: vec![],
                constraints: vec![],
            }],
        }
        .validate()
        .expect("nullary relations are legal");
        assert_eq!(schema.relation(RelationId(0)).layout().fact_width(), 0);
    }

    fn one_relation(
        fields: Vec<FieldDescriptor>,
        constraints: Vec<ConstraintDescriptor>,
    ) -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "R".into(),
                fields,
                constraints,
            }],
        }
    }

    #[test]
    fn rejects_duplicate_relation_name() {
        let decl = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "R".into(),
                    fields: vec![],
                    constraints: vec![],
                },
                RelationDescriptor {
                    name: "R".into(),
                    fields: vec![],
                    constraints: vec![],
                },
            ],
        };
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::DuplicateRelationName { name: "R".into() }
        );
    }

    #[test]
    fn rejects_duplicate_field_name() {
        let decl = one_relation(
            vec![field("x", ValueType::U64), field("x", ValueType::I64)],
            vec![],
        );
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::DuplicateFieldName {
                relation: RelationId(0),
                name: "x".into()
            }
        );
    }

    #[test]
    fn rejects_duplicate_constraint_name_including_auto_unique_collision() {
        // A declared constraint colliding with a serial auto-unique's name is
        // the same duplicate-name error — auto-uniques are ordinary.
        let decl = one_relation(
            vec![serial_field("id")],
            vec![ConstraintDescriptor::Unique {
                name: "id".into(),
                fields: Box::new([FieldId(0)]),
            }],
        );
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::DuplicateConstraintName {
                relation: RelationId(0),
                name: "id".into()
            }
        );
    }

    #[test]
    fn rejects_enum_without_variants() {
        let decl = one_relation(vec![field("e", enum_type(&[]))], vec![]);
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::EnumWithoutVariants {
                relation: RelationId(0),
                field: FieldId(0)
            }
        );
    }

    #[test]
    fn rejects_enum_with_more_than_256_variants() {
        let names: Vec<String> = (0..257).map(|i| format!("V{i}")).collect();
        let decl = one_relation(
            vec![field(
                "e",
                enum_type(&names.iter().map(String::as_str).collect::<Vec<_>>()),
            )],
            vec![],
        );
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::EnumTooManyVariants {
                relation: RelationId(0),
                field: FieldId(0),
                count: 257
            }
        );
    }

    #[test]
    fn accepts_enum_with_exactly_256_variants() {
        let names: Vec<String> = (0..256).map(|i| format!("V{i}")).collect();
        let decl = one_relation(
            vec![field(
                "e",
                enum_type(&names.iter().map(String::as_str).collect::<Vec<_>>()),
            )],
            vec![],
        );
        decl.validate().expect("256 variants fit one byte");
    }

    #[test]
    fn rejects_duplicate_enum_variant() {
        let decl = one_relation(vec![field("e", enum_type(&["A", "A"]))], vec![]);
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::DuplicateEnumVariant {
                relation: RelationId(0),
                field: FieldId(0),
                variant: "A".into()
            }
        );
    }

    #[test]
    fn rejects_serial_on_non_u64() {
        let decl = one_relation(
            vec![FieldDescriptor {
                name: "id".into(),
                value_type: ValueType::I64,
                generation: Generation::Serial,
            }],
            vec![],
        );
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::SerialOnNonU64 {
                relation: RelationId(0),
                field: FieldId(0)
            }
        );
    }

    #[test]
    fn rejects_unknown_constraint_field() {
        let decl = one_relation(
            vec![field("x", ValueType::U64)],
            vec![ConstraintDescriptor::Unique {
                name: "u".into(),
                fields: Box::new([FieldId(7)]),
            }],
        );
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::UnknownConstraintField {
                relation: RelationId(0),
                constraint: ConstraintId(0),
                field: FieldId(7)
            }
        );
    }

    #[test]
    fn rejects_unique_without_fields() {
        let decl = one_relation(
            vec![field("x", ValueType::U64)],
            vec![ConstraintDescriptor::Unique {
                name: "u".into(),
                fields: Box::new([]),
            }],
        );
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::UniqueWithoutFields {
                relation: RelationId(0),
                constraint: ConstraintId(0)
            }
        );
    }

    #[test]
    fn rejects_unique_with_duplicate_field() {
        let decl = one_relation(
            vec![field("x", ValueType::U64)],
            vec![ConstraintDescriptor::Unique {
                name: "u".into(),
                fields: Box::new([FieldId(0), FieldId(0)]),
            }],
        );
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::UniqueDuplicateField {
                relation: RelationId(0),
                constraint: ConstraintId(0),
                field: FieldId(0)
            }
        );
    }

    #[test]
    fn rejects_unknown_fk_target_relation() {
        let decl = one_relation(
            vec![field("x", ValueType::U64)],
            vec![ConstraintDescriptor::ForeignKey {
                name: "fk".into(),
                fields: Box::new([FieldId(0)]),
                target_relation: RelationId(9),
                target_constraint: ConstraintId(0),
            }],
        );
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::UnknownFkTargetRelation {
                relation: RelationId(0),
                constraint: ConstraintId(0),
                target: RelationId(9)
            }
        );
    }

    #[test]
    fn rejects_unknown_fk_target_constraint() {
        let decl = one_relation(
            vec![field("x", ValueType::U64)],
            vec![ConstraintDescriptor::ForeignKey {
                name: "fk".into(),
                fields: Box::new([FieldId(0)]),
                target_relation: RelationId(0),
                target_constraint: ConstraintId(9),
            }],
        );
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::UnknownFkTargetConstraint {
                relation: RelationId(0),
                constraint: ConstraintId(0),
                target: ConstraintId(9)
            }
        );
    }

    #[test]
    fn rejects_fk_targeting_a_foreign_key() {
        let decl = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "T".into(),
                    fields: vec![field("x", ValueType::U64)],
                    constraints: vec![
                        ConstraintDescriptor::Unique {
                            name: "x".into(),
                            fields: Box::new([FieldId(0)]),
                        },
                        ConstraintDescriptor::ForeignKey {
                            name: "self_fk".into(),
                            fields: Box::new([FieldId(0)]),
                            target_relation: RelationId(0),
                            target_constraint: ConstraintId(0),
                        },
                    ],
                },
                RelationDescriptor {
                    name: "S".into(),
                    fields: vec![field("y", ValueType::U64)],
                    constraints: vec![ConstraintDescriptor::ForeignKey {
                        name: "bad".into(),
                        fields: Box::new([FieldId(0)]),
                        target_relation: RelationId(0),
                        target_constraint: ConstraintId(1), // T's FK, not a unique
                    }],
                },
            ],
        };
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::FkTargetNotUnique {
                relation: RelationId(1),
                constraint: ConstraintId(0),
                target: ConstraintId(1)
            }
        );
    }

    #[test]
    fn rejects_fk_arity_mismatch() {
        let decl = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "T".into(),
                    fields: vec![field("a", ValueType::U64), field("b", ValueType::U64)],
                    constraints: vec![ConstraintDescriptor::Unique {
                        name: "ab".into(),
                        fields: Box::new([FieldId(0), FieldId(1)]),
                    }],
                },
                RelationDescriptor {
                    name: "S".into(),
                    fields: vec![field("a", ValueType::U64)],
                    constraints: vec![ConstraintDescriptor::ForeignKey {
                        name: "fk".into(),
                        fields: Box::new([FieldId(0)]),
                        target_relation: RelationId(0),
                        target_constraint: ConstraintId(0),
                    }],
                },
            ],
        };
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::FkArityMismatch {
                relation: RelationId(1),
                constraint: ConstraintId(0)
            }
        );
    }

    #[test]
    fn rejects_fk_positional_type_mismatch() {
        let decl = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "T".into(),
                    fields: vec![field("a", ValueType::U64)],
                    constraints: vec![ConstraintDescriptor::Unique {
                        name: "a".into(),
                        fields: Box::new([FieldId(0)]),
                    }],
                },
                RelationDescriptor {
                    name: "S".into(),
                    fields: vec![field("a", ValueType::I64)],
                    constraints: vec![ConstraintDescriptor::ForeignKey {
                        name: "fk".into(),
                        fields: Box::new([FieldId(0)]),
                        target_relation: RelationId(0),
                        target_constraint: ConstraintId(0),
                    }],
                },
            ],
        };
        assert_eq!(
            decl.validate().unwrap_err(),
            SchemaError::FkFieldTypeMismatch {
                relation: RelationId(1),
                constraint: ConstraintId(0),
                position: 0
            }
        );
    }

    // `Schema` is unconstructible outside this module: its fields and
    // `Relation`'s fields are private, and no public constructor exists —
    // the only path in is `SchemaDescriptor::validate`. (Compile-time
    // property; recorded here as the sealing contract.)
}
