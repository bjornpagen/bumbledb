//! Typed schema descriptors and current index layout generation.

#![allow(clippy::result_large_err)]

use std::collections::BTreeSet;
use std::fmt;

const INDEX_KEY_OVERHEAD_BYTES: usize = 1 + 2 + 2;
const FACT_ID_BYTES: usize = 16;

/// Schema-layer result type.
pub type Result<T> = std::result::Result<T, SchemaError>;

/// Schema descriptor errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SchemaError {
    /// A schema-level name was empty.
    #[error("schema name must not be empty")]
    EmptySchemaName,

    /// A relation name was empty.
    #[error("relation name must not be empty")]
    EmptyRelationName,

    /// A relation name was declared more than once.
    #[error("duplicate relation {relation}")]
    DuplicateRelation { relation: String },

    /// A field name was empty.
    #[error("field name must not be empty in relation {relation}")]
    EmptyFieldName { relation: String },

    /// A relation declared the same field more than once.
    #[error("duplicate field {relation}.{field}")]
    DuplicateField { relation: String, field: String },

    /// A relation referred to an unknown field.
    #[error("relation {relation} references unknown field {field}")]
    UnknownField { relation: String, field: String },

    /// An enum domain name was empty.
    #[error("enum name must not be empty")]
    EmptyEnumName,

    /// An enum domain name was declared more than once.
    #[error("duplicate enum {enum_name}")]
    DuplicateEnum { enum_name: String },

    /// An enum variant name was empty.
    #[error("variant name must not be empty in enum {enum_name}")]
    EmptyEnumVariantName { enum_name: String },

    /// An enum variant name was declared more than once.
    #[error("duplicate enum variant {enum_name}.{variant}")]
    DuplicateEnumVariant { enum_name: String, variant: String },

    /// An enum variant code was declared more than once.
    #[error("duplicate enum code {code} in enum {enum_name}")]
    DuplicateEnumCode { enum_name: String, code: u8 },

    /// A field referred to an unknown enum domain.
    #[error("relation {relation}.{field} references unknown enum {enum_name}")]
    UnknownEnum {
        relation: String,
        field: String,
        enum_name: String,
    },

    /// A constraint name was empty.
    #[error("constraint name must not be empty in relation {relation}")]
    EmptyConstraintName { relation: String },

    /// A constraint name was declared more than once within a relation.
    #[error("duplicate constraint {relation}.{constraint}")]
    DuplicateConstraint {
        relation: String,
        constraint: String,
    },

    /// A constraint declaration was invalid.
    #[error("invalid constraint {relation}.{constraint}: {reason}")]
    InvalidConstraint {
        relation: String,
        constraint: String,
        reason: String,
    },

    /// A foreign-key constraint targeted an unknown named constraint.
    #[error(
        "constraint {relation}.{constraint} targets unknown constraint {target_relation}.{target_constraint}"
    )]
    UnknownTargetConstraint {
        relation: String,
        constraint: String,
        target_relation: String,
        target_constraint: String,
    },

    /// A foreign-key constraint did not target a unique constraint.
    #[error(
        "constraint {relation}.{constraint} targets non-unique constraint {target_relation}.{target_constraint}"
    )]
    ForeignKeyTargetNotUnique {
        relation: String,
        constraint: String,
        target_relation: String,
        target_constraint: String,
    },

    /// A foreign-key source field type did not match its target field type.
    #[error(
        "constraint {relation}.{constraint} field {source_field} type {source_type} is incompatible with {target_field} type {target_type}"
    )]
    ForeignKeyTypeMismatch {
        relation: String,
        constraint: String,
        source_field: String,
        target_field: String,
        source_type: String,
        target_type: String,
    },

    /// An explicit index name was empty.
    #[error("index name must not be empty in relation {relation}")]
    EmptyIndexName { relation: String },

    /// An index name was declared more than once within a relation.
    #[error("duplicate index {relation}.{index}")]
    DuplicateIndex { relation: String, index: String },

    /// An explicit index collided with a generated index name.
    #[error("explicit index {relation}.{index} uses reserved generated index name")]
    ReservedIndexName { relation: String, index: String },

    /// An index declaration was invalid.
    #[error("invalid index {relation}.{index}: {reason}")]
    InvalidIndex {
        relation: String,
        index: String,
        reason: String,
    },

    /// A generated index key would exceed LMDB's max key size.
    #[error("index key too large for {relation}.{index}: {actual} bytes exceeds max {max} bytes")]
    KeyLayoutTooLarge {
        relation: String,
        index: String,
        actual: usize,
        max: usize,
    },

    /// An index declared the same leading field more than once.
    #[error("index {relation}.{index} declares duplicate leading field {field}")]
    DuplicateIndexField {
        relation: String,
        index: String,
        field: String,
    },
}

/// Whole compiled schema descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaDescriptor {
    /// Database/schema name.
    pub name: String,
    /// Closed enum domains in declaration order.
    pub enums: Vec<EnumDescriptor>,
    /// Relations in declaration order.
    pub relations: Vec<RelationDescriptor>,
}

impl SchemaDescriptor {
    /// Creates a new schema descriptor.
    pub fn new(name: impl Into<String>, relations: Vec<RelationDescriptor>) -> Self {
        Self {
            name: name.into(),
            enums: Vec::new(),
            relations,
        }
    }

    /// Adds a closed enum domain.
    pub fn with_enum(mut self, enum_descriptor: EnumDescriptor) -> Self {
        self.enums.push(enum_descriptor);
        self
    }

    /// Computes the deterministic schema fingerprint.
    pub fn fingerprint(&self) -> SchemaFingerprint {
        SchemaFingerprint(*blake3::hash(&self.canonical_bytes()).as_bytes())
    }

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

    /// Returns an enum domain by name.
    pub fn enum_descriptor(&self, name: &str) -> Option<&EnumDescriptor> {
        self.enums
            .iter()
            .find(|enum_descriptor| enum_descriptor.name == name)
    }

    /// Returns true if an enum domain contains an encoded code.
    pub fn enum_contains_code(&self, name: &str, code: u8) -> bool {
        self.enum_descriptor(name)
            .is_some_and(|enum_descriptor| enum_descriptor.contains_code(code))
    }

    /// Computes all current-state index layouts and validates key lengths.
    pub fn access_layouts(&self, max_key_size: usize) -> Result<Vec<AccessLayout>> {
        let mut layouts = Vec::new();

        for (relation_id, relation) in self.relations.iter().enumerate() {
            let relation_id = relation_id as u16;
            let candidates = relation.access_candidates();

            for (index_id, candidate) in candidates.into_iter().enumerate() {
                let index_id = index_id as u16;
                let components = relation.access_components(&candidate.name, &candidate.fields)?;
                let encoded_len = INDEX_KEY_OVERHEAD_BYTES
                    + components
                        .iter()
                        .map(|component| component.encoded_width)
                        .sum::<usize>()
                    + FACT_ID_BYTES;

                if encoded_len > max_key_size {
                    return Err(SchemaError::KeyLayoutTooLarge {
                        relation: relation.name.clone(),
                        index: candidate.name,
                        actual: encoded_len,
                        max: max_key_size,
                    });
                }

                layouts.push(AccessLayout {
                    relation_name: relation.name.clone(),
                    relation_id,
                    index_name: candidate.name,
                    index_id,
                    kind: candidate.kind,
                    leading_fields: candidate.fields,
                    components,
                    encoded_len,
                });
            }
        }

        Ok(layouts)
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_str(&mut out, "bumbledb.schema.v4.set-native-layout");
        push_str(&mut out, &self.name);
        push_u32(&mut out, self.enums.len() as u32);
        for enum_descriptor in &self.enums {
            enum_descriptor.push_canonical(&mut out);
        }
        push_u32(&mut out, self.relations.len() as u32);
        for relation in &self.relations {
            relation.push_canonical(&mut out);
        }
        out
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

/// A 256-bit schema fingerprint.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SchemaFingerprint(pub [u8; 32]);

impl fmt::Debug for SchemaFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for SchemaFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Closed enum domain descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumDescriptor {
    /// Enum domain name.
    pub name: String,
    /// Allowed variants in declaration order.
    pub variants: Vec<EnumVariantDescriptor>,
}

impl EnumDescriptor {
    /// Creates an enum domain from named variants.
    pub fn new(
        name: impl Into<String>,
        variants: impl IntoIterator<Item = EnumVariantDescriptor>,
    ) -> Self {
        Self {
            name: name.into(),
            variants: variants.into_iter().collect(),
        }
    }

    /// Creates an enum domain from numeric codes with generated variant names.
    pub fn codes(name: impl Into<String>, codes: impl IntoIterator<Item = u8>) -> Self {
        Self {
            name: name.into(),
            variants: codes
                .into_iter()
                .map(|code| EnumVariantDescriptor::new(format!("code_{code}"), code))
                .collect(),
        }
    }

    /// Returns true if this enum contains a variant code.
    pub fn contains_code(&self, code: u8) -> bool {
        self.variants.iter().any(|variant| variant.code == code)
    }

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

    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_str(out, &self.name);
        push_u32(out, self.variants.len() as u32);
        for variant in &self.variants {
            variant.push_canonical(out);
        }
    }
}

/// Closed enum variant descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumVariantDescriptor {
    /// Variant label.
    pub name: String,
    /// Stable encoded code.
    pub code: u8,
}

impl EnumVariantDescriptor {
    /// Creates an enum variant.
    pub fn new(name: impl Into<String>, code: u8) -> Self {
        Self {
            name: name.into(),
            code,
        }
    }

    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_str(out, &self.name);
        push_u8(out, self.code);
    }
}

/// Relation descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationDescriptor {
    /// Relation name.
    pub name: String,
    /// Fields in declaration order.
    pub fields: Vec<FieldDescriptor>,
    /// Explicit constraints.
    pub constraints: Vec<ConstraintDescriptor>,
    /// Explicit physical indexes.
    pub indexes: Vec<IndexDescriptor>,
}

impl RelationDescriptor {
    /// Creates a new relation descriptor.
    pub fn new(name: impl Into<String>, fields: Vec<FieldDescriptor>) -> Self {
        Self {
            name: name.into(),
            fields,
            constraints: Vec::new(),
            indexes: Vec::new(),
        }
    }

    /// Adds an explicit constraint.
    pub fn with_constraint(mut self, constraint: ConstraintDescriptor) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Adds a named unique constraint.
    pub fn with_unique(
        mut self,
        name: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.constraints
            .push(ConstraintDescriptor::unique(name, fields));
        self
    }

    /// Adds an explicit physical index.
    pub fn with_index(mut self, index: IndexDescriptor) -> Self {
        self.indexes.push(index);
        self
    }

    /// Returns a field by name.
    pub fn field(&self, name: &str) -> Option<&FieldDescriptor> {
        self.fields.iter().find(|field| field.name == name)
    }

    fn access_candidates(&self) -> Vec<IndexCandidate> {
        let mut candidates = Vec::new();

        candidates.push(IndexCandidate {
            name: "fact_set".to_owned(),
            kind: IndexKind::FactSet,
            fields: self.fields.iter().map(|field| field.name.clone()).collect(),
        });

        for constraint in &self.constraints {
            if let ConstraintDescriptor::Unique { name, fields } = constraint {
                candidates.push(IndexCandidate {
                    name: format!("unique_{name}"),
                    kind: IndexKind::Unique,
                    fields: fields.clone(),
                });
            }
        }

        for constraint in &self.constraints {
            if let ConstraintDescriptor::ForeignKey { name, fields, .. } = constraint {
                candidates.push(IndexCandidate {
                    name: format!("by_fk_{name}"),
                    kind: IndexKind::ForeignKey,
                    fields: fields.clone(),
                });
            }
        }

        for field in &self.fields {
            if field.indexing.range {
                candidates.push(IndexCandidate {
                    name: format!("by_{}", field.name),
                    kind: IndexKind::Range,
                    fields: vec![field.name.clone()],
                });
            }
        }

        for index in &self.indexes {
            candidates.push(IndexCandidate {
                name: index.name.clone(),
                kind: index.kind,
                fields: index.fields.clone(),
            });
        }

        candidates
    }

    fn access_components(
        &self,
        index_name: &str,
        leading_fields: &[String],
    ) -> Result<Vec<AccessComponent>> {
        let mut components = Vec::with_capacity(leading_fields.len());
        let mut seen = BTreeSet::new();

        for field_name in leading_fields {
            let field = self
                .field(field_name)
                .ok_or_else(|| SchemaError::UnknownField {
                    relation: self.name.clone(),
                    field: field_name.clone(),
                })?;

            if !seen.insert(field.name.clone()) {
                return Err(SchemaError::DuplicateIndexField {
                    relation: self.name.clone(),
                    index: index_name.to_owned(),
                    field: field.name.clone(),
                });
            }
            components.push(AccessComponent::new(field, AccessComponentRole::Leading));
        }

        Ok(components)
    }

    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_str(out, &self.name);

        push_u32(out, self.fields.len() as u32);
        for field in &self.fields {
            field.push_canonical(out);
        }

        push_u32(out, self.constraints.len() as u32);
        for constraint in &self.constraints {
            constraint.push_canonical(out);
        }

        push_u32(out, self.indexes.len() as u32);
        for index in &self.indexes {
            index.push_canonical(out);
        }
    }
}

/// Field descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDescriptor {
    /// Field name.
    pub name: String,
    /// Logical field type.
    pub value_type: ValueType,
    /// Field-level index annotations.
    pub indexing: FieldIndexing,
}

impl FieldDescriptor {
    /// Creates a field descriptor.
    pub fn new(name: impl Into<String>, value_type: ValueType) -> Self {
        Self {
            name: name.into(),
            value_type,
            indexing: FieldIndexing::default(),
        }
    }

    /// Marks this field as range-indexed.
    pub fn range_indexed(mut self) -> Self {
        self.indexing.range = true;
        self
    }

    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_str(out, &self.name);
        self.value_type.push_canonical(out);
        push_u8(out, u8::from(self.indexing.range));
    }
}

/// Field-level index annotations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FieldIndexing {
    /// Whether this field gets a scalar range index.
    pub range: bool,
}

/// Logical value type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueType {
    /// Boolean.
    Bool,
    /// Unsigned 64-bit integer.
    U64,
    /// Signed 64-bit integer.
    I64,
    /// UTC timestamp in microseconds.
    TimestampMicros,
    /// Fixed-scale decimal.
    Decimal { scale: u32 },
    /// Closed enum domain stored as a stable numeric code.
    Enum { name: String },
    /// Interned UTF-8 string.
    String,
    /// Interned bytes.
    Bytes,
    /// Nominal database-allocated serial value.
    Serial {
        type_name: String,
        owning_relation: String,
    },
}

impl ValueType {
    /// Returns the fixed encoded width of this type in index keys.
    pub fn encoded_width(&self) -> usize {
        match self {
            ValueType::Bool => 1,
            ValueType::Enum { .. } => 1,
            ValueType::U64
            | ValueType::I64
            | ValueType::TimestampMicros
            | ValueType::String
            | ValueType::Bytes
            | ValueType::Serial { .. } => 8,
            ValueType::Decimal { .. } => 16,
        }
    }

    /// Returns true if values of this type are represented by dictionary IDs in hot keys.
    pub fn is_interned_placeholder(&self) -> bool {
        matches!(self, ValueType::String | ValueType::Bytes)
    }

    /// Returns true if this type can appear in primary/unique/index keys.
    pub fn is_key_eligible(&self) -> bool {
        true
    }

    /// Returns true if this type has meaningful ordered range semantics.
    pub fn is_orderable(&self) -> bool {
        matches!(
            self,
            ValueType::U64
                | ValueType::I64
                | ValueType::TimestampMicros
                | ValueType::Decimal { .. }
                | ValueType::Serial { .. }
        )
    }

    /// Returns true if range indexes are allowed for this type.
    pub fn supports_range_index(&self) -> bool {
        self.is_orderable()
    }

    fn push_canonical(&self, out: &mut Vec<u8>) {
        match self {
            ValueType::Bool => push_u8(out, 1),
            ValueType::U64 => push_u8(out, 2),
            ValueType::I64 => push_u8(out, 3),
            ValueType::TimestampMicros => push_u8(out, 4),
            ValueType::Decimal { scale } => {
                push_u8(out, 5);
                push_u32(out, *scale);
            }
            ValueType::Enum { name } => {
                push_u8(out, 7);
                push_str(out, name);
            }
            ValueType::String => push_u8(out, 8),
            ValueType::Bytes => push_u8(out, 9),
            ValueType::Serial {
                type_name,
                owning_relation,
            } => {
                push_u8(out, 10);
                push_str(out, type_name);
                push_str(out, owning_relation);
            }
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Explicit constraint descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstraintDescriptor {
    /// Unique key constraint.
    Unique { name: String, fields: Vec<String> },
    /// Foreign key constraint.
    ForeignKey {
        name: String,
        fields: Vec<String>,
        target_relation: String,
        target_constraint: String,
        on_delete: ForeignKeyAction,
    },
}

/// Foreign-key referential action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForeignKeyAction {
    /// Reject source-breaking target changes.
    Restrict,
}

impl ConstraintDescriptor {
    /// Creates a unique constraint.
    pub fn unique(
        name: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::Unique {
            name: name.into(),
            fields: fields.into_iter().map(Into::into).collect(),
        }
    }

    /// Creates a foreign-key constraint targeting a named unique constraint.
    pub fn foreign_key(
        name: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
        target_relation: impl Into<String>,
        target_constraint: impl Into<String>,
    ) -> Self {
        Self::ForeignKey {
            name: name.into(),
            fields: fields.into_iter().map(Into::into).collect(),
            target_relation: target_relation.into(),
            target_constraint: target_constraint.into(),
            on_delete: ForeignKeyAction::Restrict,
        }
    }

    fn name(&self) -> &str {
        match self {
            ConstraintDescriptor::Unique { name, .. }
            | ConstraintDescriptor::ForeignKey { name, .. } => name,
        }
    }

    fn push_canonical(&self, out: &mut Vec<u8>) {
        match self {
            ConstraintDescriptor::Unique { name, fields } => {
                push_u8(out, 1);
                push_str(out, name);
                push_string_list(out, fields);
            }
            ConstraintDescriptor::ForeignKey {
                name,
                fields,
                target_relation,
                target_constraint,
                on_delete,
            } => {
                push_u8(out, 2);
                push_str(out, name);
                push_string_list(out, fields);
                push_str(out, target_relation);
                push_str(out, target_constraint);
                on_delete.push_canonical(out);
            }
        }
    }
}

impl ForeignKeyAction {
    fn push_canonical(self, out: &mut Vec<u8>) {
        match self {
            ForeignKeyAction::Restrict => push_u8(out, 1),
        }
    }
}

/// Explicit physical index descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexDescriptor {
    /// Stable index name within the relation.
    pub name: String,
    /// Index access kind.
    pub kind: IndexKind,
    /// Leading fields in encoded key order.
    pub fields: Vec<String>,
}

impl IndexDescriptor {
    /// Creates an explicit physical index descriptor.
    pub fn new(
        name: impl Into<String>,
        kind: IndexKind,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            fields: fields.into_iter().map(Into::into).collect(),
        }
    }

    /// Creates an equality index over scalar leading fields.
    pub fn equality(
        name: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::new(name, IndexKind::Equality, fields)
    }

    /// Creates a permutation index for alternate trie traversal order.
    pub fn permutation(
        name: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::new(name, IndexKind::Permutation, fields)
    }

    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_str(out, &self.name);
        self.kind.push_canonical(out);
        push_string_list(out, &self.fields);
    }
}

/// Current index kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexKind {
    /// Canonical fact-set access path.
    FactSet,
    /// Unique leading index.
    Unique,
    /// Foreign-key leading index.
    ForeignKey,
    /// Range leading index.
    Range,
    /// Equality leading index.
    Equality,
    /// Explicit alternate component-order index.
    Permutation,
}

impl IndexKind {
    fn push_canonical(self, out: &mut Vec<u8>) {
        push_u8(
            out,
            match self {
                IndexKind::FactSet => 1,
                IndexKind::Unique => 2,
                IndexKind::ForeignKey => 3,
                IndexKind::Range => 4,
                IndexKind::Equality => 5,
                IndexKind::Permutation => 6,
            },
        );
    }
}

/// Generated current-state index layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessLayout {
    /// Relation name.
    pub relation_name: String,
    /// Stable declaration-order relation ID placeholder.
    pub relation_id: u16,
    /// Index name.
    pub index_name: String,
    /// Declaration-order index ID placeholder within relation.
    pub index_id: u16,
    /// Index kind.
    pub kind: IndexKind,
    /// Leading fields used for prefix access.
    pub leading_fields: Vec<String>,
    /// Encoded key components in access order.
    pub components: Vec<AccessComponent>,
    /// Total encoded key length including namespace/relation/index overhead.
    pub encoded_len: usize,
}

impl AccessLayout {
    /// Typed relation indexes do not need runtime type tags in hot keys.
    pub fn needs_runtime_type_tags(&self) -> bool {
        false
    }
}

/// Index component role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessComponentRole {
    /// Leading prefix component.
    Leading,
}

/// A field component inside an index key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessComponent {
    /// Field name.
    pub field_name: String,
    /// Logical field type.
    pub value_type: ValueType,
    /// Fixed encoded width.
    pub encoded_width: usize,
    /// Component role.
    pub role: AccessComponentRole,
}

impl AccessComponent {
    fn new(field: &FieldDescriptor, role: AccessComponentRole) -> Self {
        Self {
            field_name: field.name.clone(),
            value_type: field.value_type.clone(),
            encoded_width: field.value_type.encoded_width(),
            role,
        }
    }
}

#[derive(Clone, Debug)]
struct IndexCandidate {
    name: String,
    kind: IndexKind,
    fields: Vec<String>,
}

fn generated_index_names(relation: &RelationDescriptor) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    names.insert("fact_set".to_owned());
    for field in &relation.fields {
        if field.indexing.range {
            names.insert(format!("by_{}", field.name));
        }
    }
    for constraint in &relation.constraints {
        match constraint {
            ConstraintDescriptor::Unique { name, .. } => {
                names.insert(format!("unique_{name}"));
            }
            ConstraintDescriptor::ForeignKey { name, .. } => {
                names.insert(format!("by_fk_{name}"));
            }
        }
    }
    names
}

fn foreign_key_types_compatible(source: &ValueType, target: &ValueType) -> bool {
    source == target
}

fn push_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_str(out: &mut Vec<u8>, value: &str) {
    push_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

fn push_string_list(out: &mut Vec<u8>, values: &[String]) {
    push_u32(out, values.len() as u32);
    for value in values {
        push_str(out, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_ids_are_logically_distinct() {
        let account = serial_type("AccountId", "Account");
        let instrument = serial_type("InstrumentId", "Instrument");

        assert_ne!(account, instrument);
        assert_eq!(account.encoded_width(), instrument.encoded_width());
    }

    #[test]
    fn schema_fingerprint_is_deterministic_and_sensitive() {
        let schema = ledger_schema();
        assert_eq!(schema.fingerprint(), ledger_schema().fingerprint());

        let mut changed_relation = ledger_schema();
        changed_relation.relations[0].name = "Accounts".to_owned();
        assert_ne!(schema.fingerprint(), changed_relation.fingerprint());

        let mut changed_field_name = ledger_schema();
        changed_field_name.relations[0].fields[1].name = "owner".to_owned();
        assert_ne!(schema.fingerprint(), changed_field_name.fingerprint());

        let mut changed_field_type = ledger_schema();
        changed_field_type.relations[1].fields[4].value_type = ValueType::I64;
        assert_ne!(schema.fingerprint(), changed_field_type.fingerprint());

        let mut changed_index = ledger_schema();
        changed_index.relations[1].fields[5].indexing.range = false;
        assert_ne!(schema.fingerprint(), changed_index.fingerprint());

        let mut changed_constraint = ledger_schema();
        changed_constraint.relations[0].constraints.clear();
        assert_ne!(schema.fingerprint(), changed_constraint.fingerprint());

        let mut changed_explicit_index = ledger_schema();
        changed_explicit_index.relations[0].indexes.clear();
        assert_ne!(schema.fingerprint(), changed_explicit_index.fingerprint());
    }

    #[test]
    fn computes_access_layouts() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let layouts = ledger_schema().access_layouts(511)?;

        let account_fact_set = find_layout(&layouts, "Account", "fact_set")?;
        assert_eq!(account_fact_set.kind, IndexKind::FactSet);
        assert_eq!(
            account_fact_set.leading_fields,
            ["id", "holder", "currency"]
        );
        assert_eq!(field_names(account_fact_set), ["id", "holder", "currency"]);

        let posting_at = find_layout(&layouts, "Posting", "by_at")?;
        assert_eq!(posting_at.kind, IndexKind::Range);
        assert_eq!(posting_at.leading_fields, ["at"]);

        let holder_unique = find_layout(&layouts, "Holder", "unique_name")?;
        assert_eq!(holder_unique.kind, IndexKind::Unique);
        assert_eq!(holder_unique.leading_fields, ["name"]);
        assert_eq!(field_names(holder_unique), ["name"]);

        let account_holder_fk = find_layout(&layouts, "Account", "by_fk_holder")?;
        assert_eq!(account_holder_fk.kind, IndexKind::ForeignKey);
        assert_eq!(account_holder_fk.leading_fields, ["holder"]);
        assert_eq!(field_names(account_holder_fk), ["holder"]);

        let account_currency = find_layout(&layouts, "Account", "by_currency")?;
        assert_eq!(account_currency.kind, IndexKind::Equality);
        assert_eq!(account_currency.leading_fields, ["currency", "id"]);
        assert_eq!(field_names(account_currency), ["currency", "id"]);

        for layout in &layouts {
            assert_eq!(field_names(layout), layout.leading_fields);
            assert_eq!(
                layout.encoded_len,
                INDEX_KEY_OVERHEAD_BYTES
                    + layout
                        .components
                        .iter()
                        .map(|component| component.encoded_width)
                        .sum::<usize>()
                    + FACT_ID_BYTES
            );
        }

        assert!(
            layouts
                .iter()
                .all(|layout| !layout.needs_runtime_type_tags())
        );
        Ok(())
    }

    #[test]
    fn fact_set_layout_is_first_even_when_declared_later()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = SchemaDescriptor::new(
            "Ordering",
            vec![
                RelationDescriptor::new(
                    "Account",
                    vec![
                        FieldDescriptor::new("id", serial_type("AccountId", "Account")),
                        FieldDescriptor::new("code", ValueType::U64),
                    ],
                )
                .with_constraint(ConstraintDescriptor::unique("code", ["code"]))
                .with_unique("id", ["id"]),
            ],
        );

        let layouts = schema.access_layouts(511)?;
        assert_eq!(layouts[0].index_name, "fact_set");
        assert_eq!(layouts[0].kind, IndexKind::FactSet);
        assert_eq!(layouts[1].index_name, "unique_code");
        Ok(())
    }

    #[test]
    fn string_and_bytes_fields_use_interned_placeholders()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = ledger_schema();
        let layouts = schema.access_layouts(511)?;
        let holder_unique = find_layout(&layouts, "Holder", "unique_name")?;
        let name = holder_unique
            .components
            .iter()
            .find(|component| component.field_name == "name")
            .ok_or_else(|| std::io::Error::other("missing Holder.name component"))?;
        assert!(name.value_type.is_interned_placeholder());
        assert_eq!(name.encoded_width, 8);

        let source_fact_set = find_layout(&layouts, "SourceDocument", "fact_set")?;
        let payload = source_fact_set
            .components
            .iter()
            .find(|component| component.field_name == "payload")
            .ok_or_else(|| std::io::Error::other("missing SourceDocument.payload component"))?;
        assert!(payload.value_type.is_interned_placeholder());
        assert_eq!(payload.encoded_width, 8);

        let account_fact_set = find_layout(&layouts, "Account", "fact_set")?;
        let currency = account_fact_set
            .components
            .iter()
            .find(|component| component.field_name == "currency")
            .ok_or_else(|| std::io::Error::other("missing Account.currency component"))?;
        assert_eq!(currency.encoded_width, 1);
        Ok(())
    }

    #[test]
    fn rejects_oversized_index_layouts() {
        let schema = SchemaDescriptor::new(
            "TooWide",
            vec![
                RelationDescriptor::new(
                    "Wide",
                    (0..80)
                        .map(|index| {
                            FieldDescriptor::new(
                                format!("f{index}"),
                                ValueType::Decimal { scale: 0 },
                            )
                        })
                        .collect(),
                )
                .with_unique("id", ["f0"]),
            ],
        );

        assert!(matches!(
            schema.access_layouts(511),
            Err(SchemaError::KeyLayoutTooLarge { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_explicit_index_fields() {
        let schema = SchemaDescriptor::new(
            "DuplicateIndexFields",
            vec![
                RelationDescriptor::new(
                    "Account",
                    vec![
                        FieldDescriptor::new("id", serial_type("AccountId", "Account")),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_index(IndexDescriptor::equality(
                    "bad_currency",
                    ["currency", "currency"],
                )),
            ],
        );

        assert!(matches!(
            schema.access_layouts(511),
            Err(SchemaError::DuplicateIndexField { field, .. }) if field == "currency"
        ));
    }

    #[test]
    fn validates_well_formed_schema() {
        assert_eq!(valid_schema().validate(), Ok(()));
    }

    #[test]
    fn validation_rejects_duplicate_relations() {
        let mut schema = valid_schema();
        schema.relations.push(schema.relations[0].clone());
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::DuplicateRelation { relation }) if relation == "Parent"
        ));
    }

    #[test]
    fn validation_rejects_duplicate_fields() {
        let mut schema = valid_schema();
        let duplicate = schema.relations[0].fields[0].clone();
        schema.relations[0].fields.push(duplicate);
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::DuplicateField { relation, field }) if relation == "Parent" && field == "id"
        ));
    }

    #[test]
    fn validation_allows_relation_without_named_unique() {
        let mut schema = valid_schema();
        schema.relations[0]
            .constraints
            .retain(|constraint| !matches!(constraint, ConstraintDescriptor::Unique { .. }));
        schema.relations[1].constraints.clear();
        assert!(schema.validate().is_ok());
    }

    #[test]
    fn validation_accepts_multiple_named_unique_constraints() {
        let mut schema = valid_schema();
        schema.relations[0]
            .constraints
            .push(ConstraintDescriptor::unique("id_code", ["id", "code"]));
        assert!(schema.validate().is_ok());
    }

    #[test]
    fn validation_rejects_unknown_foreign_key_target() {
        let mut schema = valid_schema();
        schema.relations[1]
            .constraints
            .push(ConstraintDescriptor::foreign_key(
                "missing_parent",
                ["parent"],
                "Missing",
                "id",
            ));
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::InvalidConstraint { relation, constraint, .. })
                if relation == "Child" && constraint == "missing_parent"
        ));
    }

    #[test]
    fn validation_rejects_duplicate_constraint_names() {
        let mut schema = valid_schema();
        schema.relations[0]
            .constraints
            .push(ConstraintDescriptor::unique("code", ["code"]));
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::DuplicateConstraint { relation, constraint })
                if relation == "Parent" && constraint == "code"
        ));
    }

    #[test]
    fn validation_rejects_empty_unique_fields() {
        let mut schema = valid_schema();
        schema.relations[0].constraints[1] = ConstraintDescriptor::unique("code", [] as [&str; 0]);
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::InvalidConstraint { relation, constraint, .. })
                if relation == "Parent" && constraint == "code"
        ));
    }

    #[test]
    fn validation_rejects_duplicate_index_names() {
        let mut schema = valid_schema();
        schema.relations[0]
            .indexes
            .push(IndexDescriptor::equality("by_code_exact", ["code"]));
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::DuplicateIndex { relation, index })
                if relation == "Parent" && index == "by_code_exact"
        ));
    }

    #[test]
    fn validation_rejects_reserved_generated_index_names() {
        let mut schema = valid_schema();
        schema.relations[0]
            .indexes
            .push(IndexDescriptor::equality("unique_code", ["code", "id"]));
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::ReservedIndexName { relation, index })
                if relation == "Parent" && index == "unique_code"
        ));
    }

    #[test]
    fn validation_rejects_non_orderable_range_index() {
        let mut schema = valid_schema();
        schema.relations[0].fields[1] =
            FieldDescriptor::new("code", ValueType::String).range_indexed();
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::InvalidIndex { relation, index, .. })
                if relation == "Parent" && index == "by_code"
        ));
    }

    #[test]
    fn validation_rejects_duplicate_enum_names() {
        let schema = valid_schema()
            .with_enum(EnumDescriptor::codes("Status", [1]))
            .with_enum(EnumDescriptor::codes("Status", [2]));
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::DuplicateEnum { enum_name }) if enum_name == "Status"
        ));
    }

    #[test]
    fn validation_rejects_duplicate_enum_variants_and_codes() {
        let duplicate_variant = valid_schema().with_enum(EnumDescriptor::new(
            "Status",
            [
                EnumVariantDescriptor::new("Open", 1),
                EnumVariantDescriptor::new("Open", 2),
            ],
        ));
        assert!(matches!(
            duplicate_variant.validate(),
            Err(SchemaError::DuplicateEnumVariant { enum_name, variant })
                if enum_name == "Status" && variant == "Open"
        ));

        let duplicate_code = valid_schema().with_enum(EnumDescriptor::new(
            "Status",
            [
                EnumVariantDescriptor::new("Open", 1),
                EnumVariantDescriptor::new("Closed", 1),
            ],
        ));
        assert!(matches!(
            duplicate_code.validate(),
            Err(SchemaError::DuplicateEnumCode { enum_name, code })
                if enum_name == "Status" && code == 1
        ));
    }

    #[test]
    fn validation_rejects_unknown_enum_domains() {
        let mut schema = valid_schema();
        schema.relations[0].fields[1] = FieldDescriptor::new(
            "code",
            ValueType::Enum {
                name: "Missing".to_owned(),
            },
        );
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::UnknownEnum { relation, field, enum_name })
                if relation == "Parent" && field == "code" && enum_name == "Missing"
        ));
    }

    #[test]
    fn validation_accepts_compound_foreign_key() {
        assert_eq!(compound_fk_schema().validate(), Ok(()));
    }

    #[test]
    fn validation_accepts_single_enum_foreign_key() {
        assert_eq!(enum_fk_schema().validate(), Ok(()));
    }

    #[test]
    fn validation_accepts_compound_enum_foreign_key() {
        assert_eq!(compound_enum_fk_schema().validate(), Ok(()));
    }

    #[test]
    fn validation_accepts_compound_serial_enum_foreign_key() {
        assert_eq!(compound_serial_enum_fk_schema().validate(), Ok(()));
    }

    #[test]
    fn validation_rejects_foreign_key_arity_mismatch() {
        let mut schema = compound_fk_schema();
        schema.relations[1].constraints[0] =
            ConstraintDescriptor::foreign_key("parent", ["parent_a"], "Parent", "by_ab");
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::InvalidConstraint { relation, constraint, .. })
                if relation == "Child" && constraint == "parent"
        ));
    }

    #[test]
    fn validation_rejects_unknown_target_constraint() {
        let mut schema = compound_fk_schema();
        schema.relations[1].constraints[0] = ConstraintDescriptor::foreign_key(
            "parent",
            ["parent_a", "parent_b"],
            "Parent",
            "missing",
        );
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::UnknownTargetConstraint { relation, constraint, .. })
                if relation == "Child" && constraint == "parent"
        ));
    }

    #[test]
    fn fingerprint_changes_when_unique_fields_change() {
        let schema = valid_schema();
        let mut changed = valid_schema();
        changed.relations[0].constraints[0] =
            ConstraintDescriptor::unique("id_code", ["id", "code"]);
        assert_ne!(schema.fingerprint(), changed.fingerprint());
    }

    #[test]
    fn fingerprint_changes_when_fk_target_constraint_changes() {
        let schema = compound_fk_schema();
        let mut changed = compound_fk_schema();
        changed.relations[0]
            .constraints
            .push(ConstraintDescriptor::unique("by_ba", ["b", "a"]));
        changed.relations[1].constraints[0] = ConstraintDescriptor::foreign_key(
            "parent",
            ["parent_b", "parent_a"],
            "Parent",
            "by_ba",
        );
        assert_ne!(schema.fingerprint(), changed.fingerprint());
    }

    #[test]
    fn validation_rejects_foreign_key_type_mismatch() {
        let mut schema = compound_fk_schema();
        schema.relations[1].fields[1] = FieldDescriptor::new("parent_a", ValueType::String);
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::ForeignKeyTypeMismatch { relation, constraint, .. })
                if relation == "Child" && constraint == "parent"
        ));
    }

    #[test]
    fn validation_rejects_enum_foreign_key_domain_mismatch() {
        let mut schema = enum_fk_schema();
        schema.relations[1].fields[1] = FieldDescriptor::new(
            "currency",
            ValueType::Enum {
                name: "Country".to_owned(),
            },
        );
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::ForeignKeyTypeMismatch {
                relation,
                constraint,
                source_field,
                target_field,
                source_type,
                target_type,
            }) if relation == "Account"
                && constraint == "currency"
                && source_field == "currency"
                && target_field == "Currency.code"
                && source_type.contains("Country")
                && target_type.contains("Currency")
        ));
    }

    #[test]
    fn validation_rejects_compound_foreign_key_field_order_mismatch() {
        let mut schema = compound_enum_fk_schema();
        schema.relations[1].constraints[0] = ConstraintDescriptor::foreign_key(
            "policy",
            ["currency", "country"],
            "Policy",
            "by_country_currency",
        );
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::ForeignKeyTypeMismatch { relation, constraint, .. })
                if relation == "Account" && constraint == "policy"
        ));
    }

    fn ledger_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "LedgerDb",
            vec![
                RelationDescriptor::new(
                    "Account",
                    vec![
                        FieldDescriptor::new("id", serial_type("AccountId", "Account")),
                        FieldDescriptor::new("holder", serial_type("HolderId", "Holder")),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::unique(
                    "holder_currency",
                    ["holder", "currency"],
                ))
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "holder",
                    ["holder"],
                    "Holder",
                    "id",
                ))
                .with_index(IndexDescriptor::equality("by_currency", ["currency", "id"])),
                RelationDescriptor::new(
                    "Posting",
                    vec![
                        FieldDescriptor::new("id", serial_type("PostingId", "Posting")),
                        FieldDescriptor::new(
                            "entry",
                            serial_type("JournalEntryId", "JournalEntry"),
                        ),
                        FieldDescriptor::new("account", serial_type("AccountId", "Account")),
                        FieldDescriptor::new(
                            "instrument",
                            serial_type("InstrumentId", "Instrument"),
                        ),
                        FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                        FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "account",
                    ["account"],
                    "Account",
                    "id",
                )),
                RelationDescriptor::new(
                    "Holder",
                    vec![
                        FieldDescriptor::new("id", serial_type("HolderId", "Holder")),
                        FieldDescriptor::new("name", ValueType::String),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::unique("name", ["name"])),
                RelationDescriptor::new(
                    "SourceDocument",
                    vec![
                        FieldDescriptor::new(
                            "id",
                            serial_type("SourceDocumentId", "SourceDocument"),
                        ),
                        FieldDescriptor::new("payload", ValueType::Bytes),
                    ],
                )
                .with_unique("id", ["id"]),
                RelationDescriptor::new(
                    "OrgParent",
                    vec![
                        FieldDescriptor::new("child", serial_type("OrgId", "Org")),
                        FieldDescriptor::new("parent", serial_type("OrgId", "Org")),
                    ],
                )
                .with_unique("child_parent", ["child", "parent"]),
            ],
        )
        .with_enum(EnumDescriptor::codes("Currency", [1, 2]))
    }

    fn valid_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "ValidationDb",
            vec![
                RelationDescriptor::new(
                    "Parent",
                    vec![
                        FieldDescriptor::new("id", serial_type("ParentId", "Parent")),
                        FieldDescriptor::new("code", ValueType::U64),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::unique("code", ["code"]))
                .with_index(IndexDescriptor::equality("by_code_exact", ["code", "id"])),
                RelationDescriptor::new(
                    "Child",
                    vec![
                        FieldDescriptor::new("id", serial_type("ChildId", "Child")),
                        FieldDescriptor::new("parent", serial_type("ParentId", "Parent")),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "parent",
                    ["parent"],
                    "Parent",
                    "id",
                )),
            ],
        )
    }

    fn compound_fk_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "CompoundFkDb",
            vec![
                RelationDescriptor::new(
                    "Parent",
                    vec![
                        FieldDescriptor::new("a", ValueType::U64),
                        FieldDescriptor::new("b", ValueType::U64),
                    ],
                )
                .with_unique("by_ab", ["a", "b"]),
                RelationDescriptor::new(
                    "Child",
                    vec![
                        FieldDescriptor::new("id", ValueType::U64),
                        FieldDescriptor::new("parent_a", ValueType::U64),
                        FieldDescriptor::new("parent_b", ValueType::U64),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "parent",
                    ["parent_a", "parent_b"],
                    "Parent",
                    "by_ab",
                )),
            ],
        )
    }

    fn enum_fk_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "EnumFkDb",
            vec![
                RelationDescriptor::new(
                    "Currency",
                    vec![FieldDescriptor::new(
                        "code",
                        ValueType::Enum {
                            name: "Currency".to_owned(),
                        },
                    )],
                )
                .with_unique("by_code", ["code"]),
                RelationDescriptor::new(
                    "Account",
                    vec![
                        FieldDescriptor::new("id", serial_type("AccountId", "Account")),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "currency",
                    ["currency"],
                    "Currency",
                    "by_code",
                )),
            ],
        )
        .with_enum(EnumDescriptor::codes("Currency", [1, 2]))
        .with_enum(EnumDescriptor::codes("Country", [1, 2]))
    }

    fn compound_enum_fk_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "CompoundEnumFkDb",
            vec![
                RelationDescriptor::new(
                    "Policy",
                    vec![
                        FieldDescriptor::new(
                            "country",
                            ValueType::Enum {
                                name: "Country".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("by_country_currency", ["country", "currency"]),
                RelationDescriptor::new(
                    "Account",
                    vec![
                        FieldDescriptor::new("id", serial_type("AccountId", "Account")),
                        FieldDescriptor::new(
                            "country",
                            ValueType::Enum {
                                name: "Country".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "policy",
                    ["country", "currency"],
                    "Policy",
                    "by_country_currency",
                )),
            ],
        )
        .with_enum(EnumDescriptor::codes("Country", [1, 2]))
        .with_enum(EnumDescriptor::codes("Currency", [1, 2]))
    }

    fn compound_serial_enum_fk_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "CompoundSerialEnumFkDb",
            vec![
                RelationDescriptor::new(
                    "AccountCurrency",
                    vec![
                        FieldDescriptor::new("account", serial_type("AccountId", "Account")),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("by_account_currency", ["account", "currency"]),
                RelationDescriptor::new(
                    "Posting",
                    vec![
                        FieldDescriptor::new("id", serial_type("PostingId", "Posting")),
                        FieldDescriptor::new("account", serial_type("AccountId", "Account")),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                )
                .with_unique("id", ["id"])
                .with_constraint(ConstraintDescriptor::foreign_key(
                    "account_currency",
                    ["account", "currency"],
                    "AccountCurrency",
                    "by_account_currency",
                )),
            ],
        )
        .with_enum(EnumDescriptor::codes("Currency", [1, 2]))
    }

    fn serial_type(type_name: &str, owning_relation: &str) -> ValueType {
        ValueType::Serial {
            type_name: type_name.to_owned(),
            owning_relation: owning_relation.to_owned(),
        }
    }

    fn find_layout<'a>(
        layouts: &'a [AccessLayout],
        relation: &str,
        index: &str,
    ) -> std::result::Result<&'a AccessLayout, Box<dyn std::error::Error>> {
        layouts
            .iter()
            .find(|layout| layout.relation_name == relation && layout.index_name == index)
            .ok_or_else(|| std::io::Error::other(format!("missing layout {relation}.{index}")))
            .map_err(Into::into)
    }

    fn field_names(layout: &AccessLayout) -> Vec<&str> {
        layout
            .components
            .iter()
            .map(|component| component.field_name.as_str())
            .collect()
    }
}
