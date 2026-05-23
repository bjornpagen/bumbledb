use std::fmt;

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

    pub(crate) fn name(&self) -> &str {
        match self {
            ConstraintDescriptor::Unique { name, .. }
            | ConstraintDescriptor::ForeignKey { name, .. } => name,
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
    pub(crate) fn new(field: &FieldDescriptor, role: AccessComponentRole) -> Self {
        Self {
            field_name: field.name.clone(),
            value_type: field.value_type.clone(),
            encoded_width: field.value_type.encoded_width(),
            role,
        }
    }
}
