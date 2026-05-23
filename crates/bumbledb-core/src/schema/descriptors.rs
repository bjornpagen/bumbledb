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
}

impl RelationDescriptor {
    /// Creates a new relation descriptor.
    pub fn new(name: impl Into<String>, fields: Vec<FieldDescriptor>) -> Self {
        Self {
            name: name.into(),
            fields,
            constraints: Vec::new(),
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
    /// DB-side generation policy.
    pub generation: FieldGeneration,
}

impl FieldDescriptor {
    /// Creates a field descriptor.
    pub fn new(name: impl Into<String>, value_type: ValueType) -> Self {
        Self {
            name: name.into(),
            value_type,
            generation: FieldGeneration::None,
        }
    }

    /// Creates a DB-generated serial field.
    pub fn generated_serial(
        name: impl Into<String>,
        type_name: impl Into<String>,
        owning_relation: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            value_type: ValueType::Serial {
                type_name: type_name.into(),
                owning_relation: owning_relation.into(),
            },
            generation: FieldGeneration::SerialSequence,
        }
    }
}

/// DB-side field generation policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FieldGeneration {
    /// The application or ETL supplies the value.
    #[default]
    None,
    /// LMDB-backed monotonic `u64` sequence for a serial field.
    SerialSequence,
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
    /// Closed enum domain stored as a stable numeric code.
    Enum { name: String },
    /// Interned UTF-8 string.
    String,
    /// Interned bytes.
    Bytes,
    /// Database-generated nominal `u64` sequence domain.
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
            | ValueType::String
            | ValueType::Bytes
            | ValueType::Serial { .. } => 8,
        }
    }

    /// Returns true if values of this type are represented by dictionary IDs in hot keys.
    pub fn is_interned_placeholder(&self) -> bool {
        matches!(self, ValueType::String | ValueType::Bytes)
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
