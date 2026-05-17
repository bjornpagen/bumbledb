//! Typed schema descriptors and current index layout generation.

use std::collections::BTreeSet;
use std::fmt;

const INDEX_KEY_OVERHEAD_BYTES: usize = 1 + 2 + 2;

/// Schema-layer result type.
pub type Result<T> = std::result::Result<T, SchemaError>;

/// Schema descriptor errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SchemaError {
    /// A relation referred to an unknown field.
    #[error("relation {relation} references unknown field {field}")]
    UnknownField { relation: String, field: String },

    /// A generated index key would exceed LMDB's max key size.
    #[error("index key too large for {relation}.{index}: {actual} bytes exceeds max {max} bytes")]
    KeyLayoutTooLarge {
        relation: String,
        index: String,
        actual: usize,
        max: usize,
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

    /// Computes all current-state index layouts and validates key lengths.
    pub fn current_index_layouts(&self, max_key_size: usize) -> Result<Vec<CurrentIndexLayout>> {
        let mut layouts = Vec::new();

        for (relation_id, relation) in self.relations.iter().enumerate() {
            let relation_id = relation_id as u16;
            let candidates = relation.index_candidates();

            for (index_id, candidate) in candidates.into_iter().enumerate() {
                let index_id = index_id as u16;
                let components = relation.covering_components(&candidate.fields)?;
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

        candidates
    }

    fn covering_components(&self, leading_fields: &[String]) -> Result<Vec<IndexComponent>> {
        let mut components = Vec::with_capacity(self.fields.len());
        let mut seen = BTreeSet::new();

        for field_name in leading_fields {
            let field = self
                .field(field_name)
                .ok_or_else(|| SchemaError::UnknownField {
                    relation: self.name.clone(),
                    field: field_name.clone(),
                })?;

            seen.insert(field.name.clone());
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
    }

    #[test]
    fn computes_current_index_layouts() {
        let layouts = ledger_schema().current_index_layouts(511).unwrap();

        let account_primary = find_layout(&layouts, "Account", "primary");
        assert_eq!(account_primary.leading_fields, ["id"]);
        assert_eq!(field_names(account_primary), ["id", "holder", "currency"]);

        let posting_account = find_layout(&layouts, "Posting", "by_account");
        assert_eq!(posting_account.kind, IndexKind::Ref);
        assert_eq!(posting_account.leading_fields, ["account"]);
        assert_eq!(
            field_names(posting_account),
            ["account", "id", "entry", "instrument", "amount", "at"]
        );

        let posting_at = find_layout(&layouts, "Posting", "by_at");
        assert_eq!(posting_at.kind, IndexKind::Range);
        assert_eq!(posting_at.leading_fields, ["at"]);

        let holder_unique = find_layout(&layouts, "Holder", "unique_name");
        assert_eq!(holder_unique.kind, IndexKind::Unique);
        assert_eq!(holder_unique.leading_fields, ["name"]);

        assert!(
            layouts
                .iter()
                .all(|layout| !layout.needs_runtime_type_tags())
        );
    }

    #[test]
    fn string_and_bytes_fields_use_interned_placeholders() {
        let schema = ledger_schema();
        let layouts = schema.current_index_layouts(511).unwrap();
        let holder_unique = find_layout(&layouts, "Holder", "unique_name");
        let name = holder_unique
            .components
            .iter()
            .find(|component| component.field_name == "name")
            .unwrap();
        assert!(name.value_type.is_interned_placeholder());
        assert_eq!(name.encoded_width, 8);

        let source_primary = find_layout(&layouts, "SourceDocument", "primary");
        let payload = source_primary
            .components
            .iter()
            .find(|component| component.field_name == "payload")
            .unwrap();
        assert!(payload.value_type.is_interned_placeholder());
        assert_eq!(payload.encoded_width, 8);
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

        let error = schema.current_index_layouts(511).unwrap_err();
        assert!(matches!(error, SchemaError::KeyLayoutTooLarge { .. }));
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
                )),
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

    fn find_layout<'a>(
        layouts: &'a [CurrentIndexLayout],
        relation: &str,
        index: &str,
    ) -> &'a CurrentIndexLayout {
        layouts
            .iter()
            .find(|layout| layout.relation_name == relation && layout.index_name == index)
            .unwrap_or_else(|| panic!("missing layout {relation}.{index}"))
    }

    fn field_names(layout: &CurrentIndexLayout) -> Vec<&str> {
        layout
            .components
            .iter()
            .map(|component| component.field_name.as_str())
            .collect()
    }
}
