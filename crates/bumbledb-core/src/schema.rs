//! Typed schema descriptors and current index layout generation.

use std::collections::BTreeSet;
use std::fmt;

const INDEX_KEY_OVERHEAD_BYTES: usize = 1 + 2 + 2;

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

    /// A primary key had no fields.
    #[error("relation {relation} primary key must not be empty")]
    EmptyPrimaryKey { relation: String },

    /// A primary key declared the same field more than once.
    #[error("relation {relation} primary key declares duplicate field {field}")]
    DuplicatePrimaryKeyField { relation: String, field: String },

    /// Generated ID metadata was invalid.
    #[error("invalid generated id for {relation}.{field}: {reason}")]
    InvalidGeneratedId {
        relation: String,
        field: String,
        reason: String,
    },

    /// A relation kind was used with an invalid schema shape.
    #[error("invalid relation kind for {relation}: {reason}")]
    InvalidRelationKind { relation: String, reason: String },

    /// A foreign-key reference named an unknown target relation.
    #[error("relation {relation}.{field} references unknown target relation {target_relation}")]
    UnknownRefTarget {
        relation: String,
        field: String,
        target_relation: String,
    },

    /// A foreign-key reference did not match its target primary-key type.
    #[error(
        "relation {relation}.{field} reference type is incompatible with {target_relation}.{target_field}"
    )]
    RefTypeMismatch {
        relation: String,
        field: String,
        target_relation: String,
        target_field: String,
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
    /// Relations in declaration order.
    pub relations: Vec<RelationDescriptor>,
}

impl SchemaDescriptor {
    /// Creates a new schema descriptor.
    pub fn new(name: impl Into<String>, relations: Vec<RelationDescriptor>) -> Self {
        Self {
            name: name.into(),
            relations,
        }
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

    /// Computes all current-state index layouts and validates key lengths.
    pub fn current_index_layouts(&self, max_key_size: usize) -> Result<Vec<CurrentIndexLayout>> {
        let mut layouts = Vec::new();

        for (relation_id, relation) in self.relations.iter().enumerate() {
            let relation_id = relation_id as u16;
            let candidates = relation.index_candidates();

            for (index_id, candidate) in candidates.into_iter().enumerate() {
                let index_id = index_id as u16;
                let components =
                    relation.covering_components(&candidate.name, &candidate.fields)?;
                let encoded_len = INDEX_KEY_OVERHEAD_BYTES
                    + components
                        .iter()
                        .map(|component| component.encoded_width)
                        .sum::<usize>();

                if encoded_len > max_key_size {
                    return Err(SchemaError::KeyLayoutTooLarge {
                        relation: relation.name.clone(),
                        index: candidate.name,
                        actual: encoded_len,
                        max: max_key_size,
                    });
                }

                layouts.push(CurrentIndexLayout {
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
        push_str(&mut out, "bumbledb.schema.v1");
        push_str(&mut out, &self.name);
        push_u32(&mut out, self.relations.len() as u32);
        for relation in &self.relations {
            relation.push_canonical(&mut out);
        }
        out
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
            self.validate_ref_field(relation, field)?;
        }

        self.validate_primary_key(relation)?;
        self.validate_generated_id(relation)?;
        self.validate_constraints(relation)?;
        self.validate_indexes(relation)?;
        self.validate_relation_kind(relation)?;

        Ok(())
    }

    fn validate_primary_key(&self, relation: &RelationDescriptor) -> Result<()> {
        if relation.primary_key.fields.is_empty() {
            return Err(SchemaError::EmptyPrimaryKey {
                relation: relation.name.clone(),
            });
        }
        let mut seen = BTreeSet::new();
        for field_name in &relation.primary_key.fields {
            let field = relation
                .field(field_name)
                .ok_or_else(|| SchemaError::UnknownField {
                    relation: relation.name.clone(),
                    field: field_name.clone(),
                })?;
            if !seen.insert(field_name.clone()) {
                return Err(SchemaError::DuplicatePrimaryKeyField {
                    relation: relation.name.clone(),
                    field: field_name.clone(),
                });
            }
            if !field.value_type.is_key_eligible() {
                return Err(SchemaError::InvalidIndex {
                    relation: relation.name.clone(),
                    index: "primary".to_owned(),
                    reason: format!("field {field_name} is not key-eligible"),
                });
            }
        }
        Ok(())
    }

    fn validate_generated_id(&self, relation: &RelationDescriptor) -> Result<()> {
        let Some(generated_id) = &relation.generated_id else {
            return Ok(());
        };
        let field =
            relation
                .field(&generated_id.field)
                .ok_or_else(|| SchemaError::InvalidGeneratedId {
                    relation: relation.name.clone(),
                    field: generated_id.field.clone(),
                    reason: "field does not exist".to_owned(),
                })?;
        if relation.primary_key.fields.len() != 1
            || relation.primary_key.fields.first() != Some(&generated_id.field)
        {
            return Err(SchemaError::InvalidGeneratedId {
                relation: relation.name.clone(),
                field: generated_id.field.clone(),
                reason: "generated IDs require a single-field primary key on the generated field"
                    .to_owned(),
            });
        }
        match &field.value_type {
            ValueType::Id {
                relation: target, ..
            } if target == &relation.name => Ok(()),
            ValueType::Id { .. } => Err(SchemaError::InvalidGeneratedId {
                relation: relation.name.clone(),
                field: generated_id.field.clone(),
                reason: "generated ID field must use an ID type for its owning relation".to_owned(),
            }),
            _ => Err(SchemaError::InvalidGeneratedId {
                relation: relation.name.clone(),
                field: generated_id.field.clone(),
                reason: "generated ID field must have an ID type".to_owned(),
            }),
        }
    }

    fn validate_relation_kind(&self, relation: &RelationDescriptor) -> Result<()> {
        if matches!(relation.kind, RelationKind::Edge | RelationKind::Set)
            && relation.generated_id.is_some()
        {
            return Err(SchemaError::InvalidRelationKind {
                relation: relation.name.clone(),
                reason: "edge and set relations must not use generated IDs".to_owned(),
            });
        }
        Ok(())
    }

    fn validate_ref_field(
        &self,
        relation: &RelationDescriptor,
        field: &FieldDescriptor,
    ) -> Result<()> {
        let ValueType::Ref {
            name,
            target_relation,
        } = &field.value_type
        else {
            return Ok(());
        };

        let target = self
            .relations
            .iter()
            .find(|candidate| &candidate.name == target_relation)
            .ok_or_else(|| SchemaError::UnknownRefTarget {
                relation: relation.name.clone(),
                field: field.name.clone(),
                target_relation: target_relation.clone(),
            })?;
        if target.primary_key.fields.len() != 1 {
            return Ok(());
        }
        let target_field_name = &target.primary_key.fields[0];
        let target_field =
            target
                .field(target_field_name)
                .ok_or_else(|| SchemaError::UnknownField {
                    relation: target.name.clone(),
                    field: target_field_name.clone(),
                })?;
        match &target_field.value_type {
            ValueType::Id {
                name: id_name,
                relation: id_relation,
            } if id_name == name && id_relation == target_relation => Ok(()),
            _ => Err(SchemaError::RefTypeMismatch {
                relation: relation.name.clone(),
                field: field.name.clone(),
                target_relation: target.name.clone(),
                target_field: target_field.name.clone(),
            }),
        }
    }

    fn validate_constraints(&self, relation: &RelationDescriptor) -> Result<()> {
        let mut names = BTreeSet::new();
        let mut unique_field_sets = BTreeSet::new();
        for constraint in &relation.constraints {
            match constraint {
                ConstraintDescriptor::Unique { name, fields } => {
                    if name.is_empty() {
                        return Err(SchemaError::EmptyConstraintName {
                            relation: relation.name.clone(),
                        });
                    }
                    if !names.insert(name.clone()) {
                        return Err(SchemaError::DuplicateConstraint {
                            relation: relation.name.clone(),
                            constraint: name.clone(),
                        });
                    }
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

/// Relation descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationDescriptor {
    /// Relation name.
    pub name: String,
    /// Relation kind.
    pub kind: RelationKind,
    /// Fields in declaration order.
    pub fields: Vec<FieldDescriptor>,
    /// Primary identity fields.
    pub primary_key: PrimaryKeyDescriptor,
    /// Generated ID metadata for entity/event relations.
    pub generated_id: Option<GeneratedIdDescriptor>,
    /// Explicit constraints.
    pub constraints: Vec<ConstraintDescriptor>,
    /// Explicit physical indexes.
    pub indexes: Vec<IndexDescriptor>,
}

impl RelationDescriptor {
    /// Creates a new relation descriptor.
    pub fn new(
        name: impl Into<String>,
        kind: RelationKind,
        fields: Vec<FieldDescriptor>,
        primary_key: PrimaryKeyDescriptor,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            fields,
            primary_key,
            generated_id: None,
            constraints: Vec::new(),
            indexes: Vec::new(),
        }
    }

    /// Adds generated ID metadata.
    pub fn with_generated_id(mut self, generated_id: GeneratedIdDescriptor) -> Self {
        self.generated_id = Some(generated_id);
        self
    }

    /// Adds an explicit constraint.
    pub fn with_constraint(mut self, constraint: ConstraintDescriptor) -> Self {
        self.constraints.push(constraint);
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

    fn index_candidates(&self) -> Vec<IndexCandidate> {
        let mut candidates = vec![IndexCandidate {
            name: "primary".to_owned(),
            kind: IndexKind::Primary,
            fields: self.primary_key.fields.clone(),
        }];

        let mut seen = BTreeSet::new();
        seen.insert(candidates[0].fields.clone());

        for field in &self.fields {
            if matches!(field.value_type, ValueType::Ref { .. }) {
                let fields = vec![field.name.clone()];
                if seen.insert(fields.clone()) {
                    candidates.push(IndexCandidate {
                        name: format!("by_{}", field.name),
                        kind: IndexKind::Ref,
                        fields,
                    });
                }
            }

            if field.indexing.range {
                let fields = vec![field.name.clone()];
                if seen.insert(fields.clone()) {
                    candidates.push(IndexCandidate {
                        name: format!("by_{}", field.name),
                        kind: IndexKind::Range,
                        fields,
                    });
                }
            }
        }

        for constraint in &self.constraints {
            match constraint {
                ConstraintDescriptor::Unique { name, fields } => {
                    if seen.insert(fields.clone()) {
                        candidates.push(IndexCandidate {
                            name: format!("unique_{name}"),
                            kind: IndexKind::Unique,
                            fields: fields.clone(),
                        });
                    }
                }
            }
        }

        for index in &self.indexes {
            if seen.insert(index.fields.clone()) {
                candidates.push(IndexCandidate {
                    name: index.name.clone(),
                    kind: index.kind,
                    fields: index.fields.clone(),
                });
            }
        }

        candidates
    }

    fn covering_components(
        &self,
        index_name: &str,
        leading_fields: &[String],
    ) -> Result<Vec<IndexComponent>> {
        let mut components = Vec::with_capacity(self.fields.len());
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
            components.push(IndexComponent::new(field, ComponentRole::Leading));
        }

        for field in &self.fields {
            if seen.insert(field.name.clone()) {
                components.push(IndexComponent::new(field, ComponentRole::Covering));
            }
        }

        Ok(components)
    }

    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_str(out, &self.name);
        push_u8(out, self.kind as u8);

        push_u32(out, self.fields.len() as u32);
        for field in &self.fields {
            field.push_canonical(out);
        }

        self.primary_key.push_canonical(out);

        match &self.generated_id {
            Some(generated_id) => {
                push_u8(out, 1);
                generated_id.push_canonical(out);
            }
            None => push_u8(out, 0),
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

/// Relation role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationKind {
    /// Entity relation with generated or application-provided identity.
    Entity = 1,
    /// Event relation with generated or application-provided identity.
    Event = 2,
    /// Edge relation, usually composite-keyed.
    Edge = 3,
    /// Pure set relation, usually composite-keyed.
    Set = 4,
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
    /// Typed generated/application ID.
    Id { name: String, relation: String },
    /// Typed foreign-key reference.
    Ref {
        name: String,
        target_relation: String,
    },
    /// UTC timestamp in microseconds.
    TimestampMicros,
    /// Fixed-scale decimal.
    Decimal { scale: u32 },
    /// UUID.
    Uuid,
    /// Symbol domain stored as a numeric/interned ID.
    Symbol { name: String },
    /// Interned UTF-8 string.
    String,
    /// Interned bytes.
    Bytes,
}

impl ValueType {
    /// Returns the fixed encoded width of this type in index keys.
    pub fn encoded_width(&self) -> usize {
        match self {
            ValueType::Bool => 1,
            ValueType::U64
            | ValueType::I64
            | ValueType::Id { .. }
            | ValueType::Ref { .. }
            | ValueType::TimestampMicros
            | ValueType::Symbol { .. }
            | ValueType::String
            | ValueType::Bytes => 8,
            ValueType::Decimal { .. } | ValueType::Uuid => 16,
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
                | ValueType::Id { .. }
                | ValueType::Ref { .. }
                | ValueType::TimestampMicros
                | ValueType::Decimal { .. }
                | ValueType::Symbol { .. }
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
            ValueType::Id { name, relation } => {
                push_u8(out, 4);
                push_str(out, name);
                push_str(out, relation);
            }
            ValueType::Ref {
                name,
                target_relation,
            } => {
                push_u8(out, 5);
                push_str(out, name);
                push_str(out, target_relation);
            }
            ValueType::TimestampMicros => push_u8(out, 6),
            ValueType::Decimal { scale } => {
                push_u8(out, 7);
                push_u32(out, *scale);
            }
            ValueType::Uuid => push_u8(out, 8),
            ValueType::Symbol { name } => {
                push_u8(out, 9);
                push_str(out, name);
            }
            ValueType::String => push_u8(out, 10),
            ValueType::Bytes => push_u8(out, 11),
        }
    }
}

/// Primary key descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimaryKeyDescriptor {
    /// Primary key fields in key order.
    pub fields: Vec<String>,
}

impl PrimaryKeyDescriptor {
    /// Creates a primary key descriptor.
    pub fn new(fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            fields: fields.into_iter().map(Into::into).collect(),
        }
    }

    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_string_list(out, &self.fields);
    }
}

/// Generated ID metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedIdDescriptor {
    /// Field receiving generated IDs.
    pub field: String,
}

impl GeneratedIdDescriptor {
    /// Creates generated ID metadata for `field`.
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
        }
    }

    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_str(out, &self.field);
    }
}

/// Explicit constraint descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstraintDescriptor {
    /// Unique key constraint.
    Unique { name: String, fields: Vec<String> },
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

    fn push_canonical(&self, out: &mut Vec<u8>) {
        match self {
            ConstraintDescriptor::Unique { name, fields } => {
                push_u8(out, 1);
                push_str(out, name);
                push_string_list(out, fields);
            }
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
    /// Primary covering index.
    Primary,
    /// Reference leading covering index.
    Ref,
    /// Unique leading covering index.
    Unique,
    /// Range leading covering index.
    Range,
    /// Equality leading covering index.
    Equality,
    /// Explicit alternate component-order index.
    Permutation,
}

impl IndexKind {
    fn push_canonical(self, out: &mut Vec<u8>) {
        push_u8(
            out,
            match self {
                IndexKind::Primary => 1,
                IndexKind::Ref => 2,
                IndexKind::Unique => 3,
                IndexKind::Range => 4,
                IndexKind::Equality => 5,
                IndexKind::Permutation => 6,
            },
        );
    }
}

/// Generated current-state index layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentIndexLayout {
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
    /// Full covering components in encoded order.
    pub components: Vec<IndexComponent>,
    /// Total encoded key length including namespace/relation/index overhead.
    pub encoded_len: usize,
}

impl CurrentIndexLayout {
    /// Typed relation indexes do not need runtime type tags in hot keys.
    pub fn needs_runtime_type_tags(&self) -> bool {
        false
    }
}

/// Index component role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentRole {
    /// Leading prefix component.
    Leading,
    /// Covering payload component inside the key.
    Covering,
}

/// A field component inside an index key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexComponent {
    /// Field name.
    pub field_name: String,
    /// Logical field type.
    pub value_type: ValueType,
    /// Fixed encoded width.
    pub encoded_width: usize,
    /// Component role.
    pub role: ComponentRole,
}

impl IndexComponent {
    fn new(field: &FieldDescriptor, role: ComponentRole) -> Self {
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
    let mut names = BTreeSet::from(["primary".to_owned()]);
    for field in &relation.fields {
        if matches!(field.value_type, ValueType::Ref { .. }) || field.indexing.range {
            names.insert(format!("by_{}", field.name));
        }
    }
    for constraint in &relation.constraints {
        match constraint {
            ConstraintDescriptor::Unique { name, .. } => {
                names.insert(format!("unique_{name}"));
            }
        }
    }
    names
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
        let account = ValueType::Id {
            name: "AccountId".to_owned(),
            relation: "Account".to_owned(),
        };
        let instrument = ValueType::Id {
            name: "InstrumentId".to_owned(),
            relation: "Instrument".to_owned(),
        };

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
    fn computes_current_index_layouts() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let layouts = ledger_schema().current_index_layouts(511)?;

        let account_primary = find_layout(&layouts, "Account", "primary")?;
        assert_eq!(account_primary.leading_fields, ["id"]);
        assert_eq!(field_names(account_primary), ["id", "holder", "currency"]);

        let posting_account = find_layout(&layouts, "Posting", "by_account")?;
        assert_eq!(posting_account.kind, IndexKind::Ref);
        assert_eq!(posting_account.leading_fields, ["account"]);
        assert_eq!(
            field_names(posting_account),
            ["account", "id", "entry", "instrument", "amount", "at"]
        );

        let posting_at = find_layout(&layouts, "Posting", "by_at")?;
        assert_eq!(posting_at.kind, IndexKind::Range);
        assert_eq!(posting_at.leading_fields, ["at"]);

        let holder_unique = find_layout(&layouts, "Holder", "unique_name")?;
        assert_eq!(holder_unique.kind, IndexKind::Unique);
        assert_eq!(holder_unique.leading_fields, ["name"]);

        let account_currency = find_layout(&layouts, "Account", "by_currency")?;
        assert_eq!(account_currency.kind, IndexKind::Equality);
        assert_eq!(account_currency.leading_fields, ["currency", "id"]);
        assert_eq!(field_names(account_currency), ["currency", "id", "holder"]);

        assert!(
            layouts
                .iter()
                .all(|layout| !layout.needs_runtime_type_tags())
        );
        Ok(())
    }

    #[test]
    fn string_and_bytes_fields_use_interned_placeholders()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = ledger_schema();
        let layouts = schema.current_index_layouts(511)?;
        let holder_unique = find_layout(&layouts, "Holder", "unique_name")?;
        let name = holder_unique
            .components
            .iter()
            .find(|component| component.field_name == "name")
            .ok_or_else(|| std::io::Error::other("missing Holder.name component"))?;
        assert!(name.value_type.is_interned_placeholder());
        assert_eq!(name.encoded_width, 8);

        let source_primary = find_layout(&layouts, "SourceDocument", "primary")?;
        let payload = source_primary
            .components
            .iter()
            .find(|component| component.field_name == "payload")
            .ok_or_else(|| std::io::Error::other("missing SourceDocument.payload component"))?;
        assert!(payload.value_type.is_interned_placeholder());
        assert_eq!(payload.encoded_width, 8);
        Ok(())
    }

    #[test]
    fn rejects_oversized_index_layouts() {
        let schema = SchemaDescriptor::new(
            "TooWide",
            vec![RelationDescriptor::new(
                "Wide",
                RelationKind::Entity,
                (0..80)
                    .map(|index| FieldDescriptor::new(format!("f{index}"), ValueType::Uuid))
                    .collect(),
                PrimaryKeyDescriptor::new(["f0"]),
            )],
        );

        assert!(matches!(
            schema.current_index_layouts(511),
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
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "AccountId".to_owned(),
                                relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Symbol {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_index(IndexDescriptor::equality(
                    "bad_currency",
                    ["currency", "currency"],
                )),
            ],
        );

        assert!(matches!(
            schema.current_index_layouts(511),
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
    fn validation_rejects_empty_primary_key() {
        let mut schema = valid_schema();
        schema.relations[0].primary_key.fields.clear();
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::EmptyPrimaryKey { relation }) if relation == "Parent"
        ));
    }

    #[test]
    fn validation_rejects_duplicate_primary_key_fields() {
        let mut schema = valid_schema();
        schema.relations[1].primary_key.fields = vec!["id".to_owned(), "id".to_owned()];
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::DuplicatePrimaryKeyField { relation, field }) if relation == "Child" && field == "id"
        ));
    }

    #[test]
    fn validation_rejects_invalid_generated_id() {
        let mut schema = valid_schema();
        schema.relations[0].generated_id = Some(GeneratedIdDescriptor::new("missing"));
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::InvalidGeneratedId { relation, field, .. }) if relation == "Parent" && field == "missing"
        ));
    }

    #[test]
    fn validation_rejects_unknown_ref_target() {
        let mut schema = valid_schema();
        schema.relations[1].fields[1].value_type = ValueType::Ref {
            name: "MissingId".to_owned(),
            target_relation: "Missing".to_owned(),
        };
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::UnknownRefTarget { relation, field, target_relation })
                if relation == "Child" && field == "parent" && target_relation == "Missing"
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
        schema.relations[0].constraints[0] = ConstraintDescriptor::unique("code", [] as [&str; 0]);
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
        schema.relations[1]
            .indexes
            .push(IndexDescriptor::equality("by_parent", ["parent", "id"]));
        assert!(matches!(
            schema.validate(),
            Err(SchemaError::ReservedIndexName { relation, index })
                if relation == "Child" && index == "by_parent"
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

    fn ledger_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "LedgerDb",
            vec![
                RelationDescriptor::new(
                    "Account",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "AccountId".to_owned(),
                                relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "holder",
                            ValueType::Ref {
                                name: "HolderId".to_owned(),
                                target_relation: "Holder".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Symbol {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(GeneratedIdDescriptor::new("id"))
                .with_constraint(ConstraintDescriptor::unique(
                    "holder_currency",
                    ["holder", "currency"],
                ))
                .with_index(IndexDescriptor::equality("by_currency", ["currency", "id"])),
                RelationDescriptor::new(
                    "Posting",
                    RelationKind::Event,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "PostingId".to_owned(),
                                relation: "Posting".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "entry",
                            ValueType::Ref {
                                name: "JournalEntryId".to_owned(),
                                target_relation: "JournalEntry".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "account",
                            ValueType::Ref {
                                name: "AccountId".to_owned(),
                                target_relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "instrument",
                            ValueType::Ref {
                                name: "InstrumentId".to_owned(),
                                target_relation: "Instrument".to_owned(),
                            },
                        ),
                        FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                        FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(GeneratedIdDescriptor::new("id")),
                RelationDescriptor::new(
                    "Holder",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "HolderId".to_owned(),
                                relation: "Holder".to_owned(),
                            },
                        ),
                        FieldDescriptor::new("name", ValueType::String),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(GeneratedIdDescriptor::new("id"))
                .with_constraint(ConstraintDescriptor::unique("name", ["name"])),
                RelationDescriptor::new(
                    "SourceDocument",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "SourceDocumentId".to_owned(),
                                relation: "SourceDocument".to_owned(),
                            },
                        ),
                        FieldDescriptor::new("payload", ValueType::Bytes),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(GeneratedIdDescriptor::new("id")),
                RelationDescriptor::new(
                    "OrgParent",
                    RelationKind::Edge,
                    vec![
                        FieldDescriptor::new(
                            "child",
                            ValueType::Ref {
                                name: "OrgId".to_owned(),
                                target_relation: "Org".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "parent",
                            ValueType::Ref {
                                name: "OrgId".to_owned(),
                                target_relation: "Org".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["child", "parent"]),
                ),
            ],
        )
    }

    fn valid_schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "ValidationDb",
            vec![
                RelationDescriptor::new(
                    "Parent",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "ParentId".to_owned(),
                                relation: "Parent".to_owned(),
                            },
                        ),
                        FieldDescriptor::new("code", ValueType::U64),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(GeneratedIdDescriptor::new("id"))
                .with_constraint(ConstraintDescriptor::unique("code", ["code"]))
                .with_index(IndexDescriptor::equality("by_code_exact", ["code", "id"])),
                RelationDescriptor::new(
                    "Child",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "ChildId".to_owned(),
                                relation: "Child".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "parent",
                            ValueType::Ref {
                                name: "ParentId".to_owned(),
                                target_relation: "Parent".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(GeneratedIdDescriptor::new("id")),
            ],
        )
    }

    fn find_layout<'a>(
        layouts: &'a [CurrentIndexLayout],
        relation: &str,
        index: &str,
    ) -> std::result::Result<&'a CurrentIndexLayout, Box<dyn std::error::Error>> {
        layouts
            .iter()
            .find(|layout| layout.relation_name == relation && layout.index_name == index)
            .ok_or_else(|| std::io::Error::other(format!("missing layout {relation}.{index}")))
            .map_err(Into::into)
    }

    fn field_names(layout: &CurrentIndexLayout) -> Vec<&str> {
        layout
            .components
            .iter()
            .map(|component| component.field_name.as_str())
            .collect()
    }
}
