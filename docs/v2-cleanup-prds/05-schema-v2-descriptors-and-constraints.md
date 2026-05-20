# PRD 05: Schema v2 Descriptors and Constraints

## Goal

Remove the old relation descriptor model and replace it with explicit named constraints.

This PRD removes primary keys, generated ID descriptors, relation kinds, implicit ref foreign keys, and check placeholders.

## Current State

Current relation descriptor at `crates/bumbledb-core/src/schema.rs:863-880`:

```rust
pub struct RelationDescriptor {
    pub name: String,
    pub kind: RelationKind,
    pub fields: Vec<FieldDescriptor>,
    pub primary_key: PrimaryKeyDescriptor,
    pub generated_id: Option<GeneratedIdDescriptor>,
    pub constraints: Vec<ConstraintDescriptor>,
    pub indexes: Vec<IndexDescriptor>,
}
```

Current helpers to delete:

- `RelationDescriptor::new(name, kind, fields, primary_key)` at `schema.rs:883-899`
- `with_generated_id` at `schema.rs:901-905`
- `with_ref_foreign_keys` at `schema.rs:919-936`
- `SchemaDescriptor::with_ref_foreign_keys` at `schema.rs:189-197`

Current validation to delete or replace:

- `validate_primary_key` at `schema.rs:374-403`
- `validate_generated_id` at `schema.rs:405-442`
- `validate_relation_kind` at `schema.rs:444-454`
- `validate_ref_field` at `schema.rs:456-501`
- FK-primary requirement at `schema.rs:627-632`

## Target Descriptor Types

Replace `RelationDescriptor` with:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationDescriptor {
    pub name: String,
    pub fields: Vec<FieldDescriptor>,
    pub constraints: Vec<ConstraintDescriptor>,
    pub indexes: Vec<IndexDescriptor>,
}

impl RelationDescriptor {
    pub fn new(name: impl Into<String>, fields: Vec<FieldDescriptor>) -> Self {
        Self {
            name: name.into(),
            fields,
            constraints: Vec::new(),
            indexes: Vec::new(),
        }
    }

    pub fn with_constraint(mut self, constraint: ConstraintDescriptor) -> Self {
        self.constraints.push(constraint);
        self
    }

    pub fn with_covering_unique(
        mut self,
        name: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.constraints.push(ConstraintDescriptor::unique_covering(name, fields));
        self
    }

    pub fn with_index(mut self, index: IndexDescriptor) -> Self {
        self.indexes.push(index);
        self
    }
}
```

Replace `ConstraintDescriptor` with:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstraintDescriptor {
    Unique {
        name: String,
        fields: Vec<String>,
        covering: bool,
    },
    ForeignKey {
        name: String,
        fields: Vec<String>,
        target_relation: String,
        target_constraint: String,
        on_delete: ForeignKeyAction,
        on_update: ForeignKeyAction,
    },
}
```

Remove `Check`. It is currently rejected by validation and only adds dead branches.

Constructors:

```rust
impl ConstraintDescriptor {
    pub fn unique(
        name: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::Unique {
            name: name.into(),
            fields: fields.into_iter().map(Into::into).collect(),
            covering: false,
        }
    }

    pub fn unique_covering(
        name: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::Unique {
            name: name.into(),
            fields: fields.into_iter().map(Into::into).collect(),
            covering: true,
        }
    }

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
            on_update: ForeignKeyAction::Restrict,
        }
    }
}
```

## Validation Rules

### Relation Shape

Every relation must have:

- Non-empty relation name.
- Unique field names.
- Valid field types.
- Exactly one covering unique constraint.

### Unique Constraints

Every unique constraint must have:

- Non-empty name.
- Non-empty field list.
- No duplicate fields.
- All fields known to the relation.
- Only key-eligible fields.

Duplicate unique field sets are allowed only if there is a concrete reason. Default policy: reject duplicate field sets as branchy redundant schema.

### Covering Unique Constraint

Exactly one `Unique { covering: true, .. }` is required.

Add errors:

```rust
#[error("relation {relation} must declare exactly one covering unique constraint")]
MissingCoveringConstraint { relation: String },

#[error("relation {relation} declares multiple covering unique constraints: {constraints:?}")]
MultipleCoveringConstraints {
    relation: String,
    constraints: Vec<String>,
},
```

### Foreign Keys

Every FK must have:

- Non-empty source field list.
- Existing target relation.
- Existing target constraint.
- Target constraint must be `Unique`.
- Source arity equals target unique field arity.
- Source field types pairwise equal target field types.
- Only `Restrict` actions for now.

Add errors:

```rust
UnknownTargetConstraint {
    relation: String,
    constraint: String,
    target_relation: String,
    target_constraint: String,
},

ForeignKeyTargetNotUnique {
    relation: String,
    constraint: String,
    target_relation: String,
    target_constraint: String,
},

IdentityTypeMismatch {
    relation: String,
    constraint: String,
    source_field: String,
    target_field: String,
    source_type: String,
    target_type: String,
},
```

The `IdentityTypeMismatch` name may be generalized to `ForeignKeyTypeMismatch` if non-identity fields may be FK targets. Prefer generic if FK over enum/code-like keys is supported.

## Deleted Types and Errors

Remove these types:

- `PrimaryKeyDescriptor`
- `GeneratedIdDescriptor`
- `RelationKind`

Remove these `SchemaError` variants:

- `EmptyPrimaryKey`
- `DuplicatePrimaryKeyField`
- `InvalidGeneratedId`
- `InvalidRelationKind`
- `UnknownRefTarget`
- `RefTypeMismatch`

Remove all validation branches that reference them.

## Canonical Serialization

Bump schema version in `SchemaDescriptor::canonical_bytes` at `schema.rs:292-305`:

```rust
push_str(&mut out, "bumbledb.schema.v2");
```

Remove relation-kind, primary-key, and generated-id serialization from `RelationDescriptor::push_canonical` at `schema.rs:1080-1108`.

New relation canonical shape:

```rust
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
```

New unique canonical shape:

```rust
ConstraintDescriptor::Unique { name, fields, covering } => {
    push_u8(out, 1);
    push_str(out, name);
    push_string_list(out, fields);
    push_u8(out, u8::from(*covering));
}
```

New FK canonical shape:

```rust
ConstraintDescriptor::ForeignKey {
    name,
    fields,
    target_relation,
    target_constraint,
    on_delete,
    on_update,
} => {
    push_u8(out, 2);
    push_str(out, name);
    push_string_list(out, fields);
    push_str(out, target_relation);
    push_str(out, target_constraint);
    on_delete.push_canonical(out);
    on_update.push_canonical(out);
}
```

## Example Target Schema

```rust
RelationDescriptor::new(
    "Account",
    vec![
        FieldDescriptor::new("id", identity("AccountId", "Account")),
        FieldDescriptor::new("holder", identity("HolderId", "Holder")),
        FieldDescriptor::new("currency", ValueType::Enum { name: "Currency".to_owned() }),
    ],
)
.with_covering_unique("by_id", ["id"])
.with_constraint(ConstraintDescriptor::unique("holder_currency", ["holder", "currency"]))
.with_constraint(ConstraintDescriptor::foreign_key(
    "holder_fk",
    ["holder"],
    "Holder",
    "by_id",
))
```

## Tests Required

Add/update tests for:

- Relation with no covering unique fails.
- Relation with two covering uniques fails.
- Relation with exactly one covering unique passes.
- Unknown unique field fails.
- Duplicate unique field fails.
- Duplicate unique field set fails.
- FK with unknown target relation fails.
- FK with unknown target constraint fails.
- FK target constraint not unique fails, if non-unique constraints are ever added.
- FK arity mismatch fails.
- FK type mismatch fails.
- FK to named unique succeeds.
- Fingerprint changes when covering flag changes.
- Fingerprint changes when FK target constraint changes.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- Grep for `PrimaryKeyDescriptor` returns no real code references.
- Grep for `GeneratedIdDescriptor` returns no real code references.
- Grep for `RelationKind` returns no real code references.
- Grep for `with_ref_foreign_keys` returns no real code references.

## Completion Criteria

- Schema descriptors are explicit and minimal.
- There is no primary-key descriptor.
- There is no generated-ID descriptor.
- There is no relation-kind enum.
- There are no implicit FK helpers.
- Every relation validates through exactly one covering unique constraint.
