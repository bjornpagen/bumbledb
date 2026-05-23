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
