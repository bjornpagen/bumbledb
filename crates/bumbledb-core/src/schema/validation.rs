use std::collections::BTreeSet;

use super::layout::generated_index_names;
use super::{
    ConstraintDescriptor, EnumDescriptor, FieldDescriptor, ForeignKeyAction, IndexKind,
    RelationDescriptor, Result, SchemaDescriptor, SchemaError, ValueType,
};

impl SchemaDescriptor {
    /// Validates the logical schema before storage layout generation.
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(SchemaError::EmptySchemaName);
        }

        self.validate_enums()?;

        let mut relation_names = BTreeSet::new();
        for relation in &self.relations {
            if relation.name.is_empty() {
                return Err(SchemaError::EmptyRelationName);
            }
            if !relation_names.insert(relation.name.clone()) {
                return Err(SchemaError::DuplicateRelation {
                    relation: relation.name.clone(),
                });
            }
        }

        for relation in &self.relations {
            self.validate_relation(relation)?;
        }

        Ok(())
    }

    fn validate_enums(&self) -> Result<()> {
        let mut names = BTreeSet::new();
        for enum_descriptor in &self.enums {
            if enum_descriptor.name.is_empty() {
                return Err(SchemaError::EmptyEnumName);
            }
            if !names.insert(enum_descriptor.name.clone()) {
                return Err(SchemaError::DuplicateEnum {
                    enum_name: enum_descriptor.name.clone(),
                });
            }
            enum_descriptor.validate()?;
        }
        Ok(())
    }

    fn validate_relation(&self, relation: &RelationDescriptor) -> Result<()> {
        let mut field_names = BTreeSet::new();
        for field in &relation.fields {
            if field.name.is_empty() {
                return Err(SchemaError::EmptyFieldName {
                    relation: relation.name.clone(),
                });
            }
            if !field_names.insert(field.name.clone()) {
                return Err(SchemaError::DuplicateField {
                    relation: relation.name.clone(),
                    field: field.name.clone(),
                });
            }
            if field.indexing.range && !field.value_type.supports_range_index() {
                return Err(SchemaError::InvalidIndex {
                    relation: relation.name.clone(),
                    index: format!("by_{}", field.name),
                    reason: format!("field {} has non-range-indexable type", field.name),
                });
            }
            self.validate_field_type(relation, field)?;
        }

        self.validate_constraints(relation)?;
        self.validate_indexes(relation)?;

        Ok(())
    }

    fn validate_field_type(
        &self,
        relation: &RelationDescriptor,
        field: &FieldDescriptor,
    ) -> Result<()> {
        if let ValueType::Enum { name } = &field.value_type
            && self.enum_descriptor(name).is_none()
        {
            return Err(SchemaError::UnknownEnum {
                relation: relation.name.clone(),
                field: field.name.clone(),
                enum_name: name.clone(),
            });
        }
        Ok(())
    }

    fn validate_constraints(&self, relation: &RelationDescriptor) -> Result<()> {
        let mut names = BTreeSet::new();
        let mut unique_field_sets = BTreeSet::new();
        for constraint in &relation.constraints {
            let constraint_name = constraint.name();
            if constraint_name.is_empty() {
                return Err(SchemaError::EmptyConstraintName {
                    relation: relation.name.clone(),
                });
            }
            if !names.insert(constraint_name.to_owned()) {
                return Err(SchemaError::DuplicateConstraint {
                    relation: relation.name.clone(),
                    constraint: constraint_name.to_owned(),
                });
            }
            match constraint {
                ConstraintDescriptor::Unique { name, fields } => {
                    if fields.is_empty() {
                        return Err(SchemaError::InvalidConstraint {
                            relation: relation.name.clone(),
                            constraint: name.clone(),
                            reason: "unique field list must not be empty".to_owned(),
                        });
                    }
                    let mut seen_fields = BTreeSet::new();
                    for field_name in fields {
                        let field = relation.field(field_name).ok_or_else(|| {
                            SchemaError::UnknownField {
                                relation: relation.name.clone(),
                                field: field_name.clone(),
                            }
                        })?;
                        if !seen_fields.insert(field_name.clone()) {
                            return Err(SchemaError::InvalidConstraint {
                                relation: relation.name.clone(),
                                constraint: name.clone(),
                                reason: format!("duplicate field {field_name}"),
                            });
                        }
                        if !field.value_type.is_key_eligible() {
                            return Err(SchemaError::InvalidConstraint {
                                relation: relation.name.clone(),
                                constraint: name.clone(),
                                reason: format!("field {field_name} is not key-eligible"),
                            });
                        }
                    }
                    if !unique_field_sets.insert(fields.clone()) {
                        return Err(SchemaError::InvalidConstraint {
                            relation: relation.name.clone(),
                            constraint: name.clone(),
                            reason: "duplicate unique field set".to_owned(),
                        });
                    }
                }
                ConstraintDescriptor::ForeignKey {
                    name,
                    fields,
                    target_relation,
                    target_constraint,
                    on_delete,
                } => {
                    if *on_delete != ForeignKeyAction::Restrict {
                        return Err(SchemaError::InvalidConstraint {
                            relation: relation.name.clone(),
                            constraint: name.clone(),
                            reason: "only restrict foreign-key actions are supported".to_owned(),
                        });
                    }
                    self.validate_foreign_key_constraint(
                        relation,
                        name,
                        fields,
                        target_relation,
                        target_constraint,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn validate_foreign_key_constraint(
        &self,
        relation: &RelationDescriptor,
        name: &str,
        fields: &[String],
        target_relation: &str,
        target_constraint: &str,
    ) -> Result<()> {
        if fields.is_empty() {
            return Err(SchemaError::InvalidConstraint {
                relation: relation.name.clone(),
                constraint: name.to_owned(),
                reason: "foreign-key field list must not be empty".to_owned(),
            });
        }
        let target = self
            .relations
            .iter()
            .find(|candidate| candidate.name == target_relation)
            .ok_or_else(|| SchemaError::InvalidConstraint {
                relation: relation.name.clone(),
                constraint: name.to_owned(),
                reason: format!("unknown target relation {target_relation}"),
            })?;
        let target_unique = target
            .constraints
            .iter()
            .find(|constraint| constraint.name() == target_constraint)
            .ok_or_else(|| SchemaError::UnknownTargetConstraint {
                relation: relation.name.clone(),
                constraint: name.to_owned(),
                target_relation: target_relation.to_owned(),
                target_constraint: target_constraint.to_owned(),
            })?;
        let ConstraintDescriptor::Unique {
            fields: target_fields,
            ..
        } = target_unique
        else {
            return Err(SchemaError::ForeignKeyTargetNotUnique {
                relation: relation.name.clone(),
                constraint: name.to_owned(),
                target_relation: target_relation.to_owned(),
                target_constraint: target_constraint.to_owned(),
            });
        };
        if fields.len() != target_fields.len() {
            return Err(SchemaError::InvalidConstraint {
                relation: relation.name.clone(),
                constraint: name.to_owned(),
                reason: "foreign-key source and target field counts must match".to_owned(),
            });
        }

        let mut source_seen = BTreeSet::new();
        let mut target_seen = BTreeSet::new();
        for (source_field_name, target_field_name) in fields.iter().zip(target_fields) {
            if !source_seen.insert(source_field_name.clone()) {
                return Err(SchemaError::InvalidConstraint {
                    relation: relation.name.clone(),
                    constraint: name.to_owned(),
                    reason: format!("duplicate source field {source_field_name}"),
                });
            }
            if !target_seen.insert(target_field_name.clone()) {
                return Err(SchemaError::InvalidConstraint {
                    relation: relation.name.clone(),
                    constraint: name.to_owned(),
                    reason: format!("duplicate target field {target_field_name}"),
                });
            }
            let source_field =
                relation
                    .field(source_field_name)
                    .ok_or_else(|| SchemaError::UnknownField {
                        relation: relation.name.clone(),
                        field: source_field_name.clone(),
                    })?;
            let target_field =
                target
                    .field(target_field_name)
                    .ok_or_else(|| SchemaError::UnknownField {
                        relation: target.name.clone(),
                        field: target_field_name.clone(),
                    })?;
            if !foreign_key_types_compatible(&source_field.value_type, &target_field.value_type) {
                return Err(SchemaError::ForeignKeyTypeMismatch {
                    relation: relation.name.clone(),
                    constraint: name.to_owned(),
                    source_field: source_field_name.clone(),
                    target_field: format!("{target_relation}.{target_field_name}"),
                    source_type: source_field.value_type.to_string(),
                    target_type: target_field.value_type.to_string(),
                });
            }
        }
        Ok(())
    }

    fn validate_indexes(&self, relation: &RelationDescriptor) -> Result<()> {
        let generated_names = generated_index_names(relation);
        let mut names = BTreeSet::new();
        for index in &relation.indexes {
            if index.name.is_empty() {
                return Err(SchemaError::EmptyIndexName {
                    relation: relation.name.clone(),
                });
            }
            if !names.insert(index.name.clone()) {
                return Err(SchemaError::DuplicateIndex {
                    relation: relation.name.clone(),
                    index: index.name.clone(),
                });
            }
            if generated_names.contains(&index.name) {
                return Err(SchemaError::ReservedIndexName {
                    relation: relation.name.clone(),
                    index: index.name.clone(),
                });
            }
            if index.fields.is_empty() {
                return Err(SchemaError::InvalidIndex {
                    relation: relation.name.clone(),
                    index: index.name.clone(),
                    reason: "leading field list must not be empty".to_owned(),
                });
            }
            let mut seen_fields = BTreeSet::new();
            for field_name in &index.fields {
                let field =
                    relation
                        .field(field_name)
                        .ok_or_else(|| SchemaError::UnknownField {
                            relation: relation.name.clone(),
                            field: field_name.clone(),
                        })?;
                if !seen_fields.insert(field_name.clone()) {
                    return Err(SchemaError::DuplicateIndexField {
                        relation: relation.name.clone(),
                        index: index.name.clone(),
                        field: field_name.clone(),
                    });
                }
                if !field.value_type.is_key_eligible() {
                    return Err(SchemaError::InvalidIndex {
                        relation: relation.name.clone(),
                        index: index.name.clone(),
                        reason: format!("field {field_name} is not key-eligible"),
                    });
                }
            }
            if index.kind == IndexKind::Range
                && index.fields.first().is_none_or(|field_name| {
                    relation
                        .field(field_name)
                        .is_none_or(|field| !field.value_type.supports_range_index())
                })
            {
                return Err(SchemaError::InvalidIndex {
                    relation: relation.name.clone(),
                    index: index.name.clone(),
                    reason: "range index leading field must be orderable".to_owned(),
                });
            }
        }
        Ok(())
    }
}

impl EnumDescriptor {
    fn validate(&self) -> Result<()> {
        let mut names = BTreeSet::new();
        let mut codes = BTreeSet::new();
        for variant in &self.variants {
            if variant.name.is_empty() {
                return Err(SchemaError::EmptyEnumVariantName {
                    enum_name: self.name.clone(),
                });
            }
            if !names.insert(variant.name.clone()) {
                return Err(SchemaError::DuplicateEnumVariant {
                    enum_name: self.name.clone(),
                    variant: variant.name.clone(),
                });
            }
            if !codes.insert(variant.code) {
                return Err(SchemaError::DuplicateEnumCode {
                    enum_name: self.name.clone(),
                    code: variant.code,
                });
            }
        }
        Ok(())
    }
}

fn foreign_key_types_compatible(source: &ValueType, target: &ValueType) -> bool {
    source == target
}
