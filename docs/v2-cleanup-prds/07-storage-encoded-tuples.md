# PRD 07: Storage Encoded Tuples

## Goal

Replace current-row payload storage and `EncodedRow { BTreeMap<String, Vec<u8>> }` with a compact encoded tuple representation based on schema field order.

This PRD makes the storage layer follow the same model as the schema layout: every fact is one encoded tuple, and every access path key contains the full tuple.

## Current State

Current storage has two representations of a row:

- Current-row payload under `NS_CURRENT_ROW`.
- Current index entries under `NS_CURRENT_TUPLE`.

Anchors:

- `NS_CURRENT_ROW`: `crates/bumbledb-lmdb/src/storage.rs:19`
- `EncodedRow { fields: BTreeMap<String, Vec<u8>> }`: `storage.rs:301-341`
- `EncodedRow::payload`: `storage.rs:314-320`
- `EncodedRow::from_payload`: `storage.rs:322-341`
- Insert writes current row payload: `storage.rs:516-520`
- Replace updates current row payload: `storage.rs:460-464`
- Delete removes current row payload: `storage.rs:549`
- `decode_index_scan_item` branches on `layout.covers_full_row`: `storage.rs:1684-1708`
- `current_row_key`: `storage.rs:1862-1866`

## Required New Types

Add internal encoded tuple representation:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EncodedTuple {
    relation: RelationId,
    bytes: Vec<u8>,
}

impl EncodedTuple {
    pub(crate) fn field<'a>(&'a self, layout: &CompiledRelationLayout, field: FieldId) -> Result<&'a [u8]>;
    pub(crate) fn from_row(schema: &StorageSchema, relation: &CompiledRelationLayout, row: &Row, intern: impl FnMut(u8, &[u8]) -> Result<u64>) -> Result<Self>;
}
```

The exact `CompiledRelationLayout` name may differ, but `StorageSchema` must expose field offsets by relation and field id.

Required compiled field layout:

```rust
#[derive(Clone, Debug)]
pub(crate) struct CompiledFieldLayout {
    pub id: FieldId,
    pub name: String,
    pub value_type: ValueType,
    pub offset: usize,
    pub width: usize,
}
```

Required relation layout:

```rust
#[derive(Clone, Debug)]
pub(crate) struct CompiledRelationLayout {
    pub id: RelationId,
    pub name: String,
    pub fields: Vec<CompiledFieldLayout>,
    pub tuple_width: usize,
    pub covering_access: AccessId,
}
```

## Required Storage Key Model

Keep or rename `NS_CURRENT_TUPLE`; it becomes the only current fact/index namespace.

Delete `NS_CURRENT_ROW`.

Delete `current_row_key`.

Delete row payload puts/gets.

Every access path key is:

```text
NS_CURRENT_TUPLE || relation_id || access_id || encoded field components in access component order
```

The access key's component suffix contains the full tuple.

## Required Encoding Flow

Old flow:

```text
Row -> EncodedRow(BTreeMap) -> primary bytes -> current-row payload + index keys
```

New flow:

```text
Row -> EncodedTuple(field-order bytes) -> access keys
```

Required helper:

```rust
fn current_access_key(layout: &CurrentIndexLayout, tuple: &EncodedTuple, relation_layout: &CompiledRelationLayout) -> Result<Vec<u8>> {
    let mut key = layout.key_prefix.clone();
    for component in &layout.components {
        let field_id = component.field_id;
        key.extend_from_slice(tuple.field(relation_layout, field_id)?);
    }
    Ok(key)
}
```

`IndexComponent` should move from field-name-driven to field-id-driven if possible:

```rust
pub struct IndexComponent {
    pub field_id: FieldId,
    pub field_name: String,
    pub value_type: ValueType,
    pub encoded_width: usize,
    pub role: ComponentRole,
}
```

## Required Decode Flow

Because every key covers the full tuple, `decode_index_scan_item` should not fetch a current-row payload.

Target shape:

```rust
fn decode_index_scan_item(
    dict: crate::RawDatabase,
    txn: &heed::RoTxn,
    relation: &RelationDescriptor,
    relation_layout: &CompiledRelationLayout,
    layout: &CurrentIndexLayout,
    key: &[u8],
) -> Result<ScanItem> {
    let encoded = decode_access_key_to_tuple(relation_layout, layout, key)?;
    let row = decode_encoded_tuple(dict, txn, relation, relation_layout, &encoded)?;
    Ok(ScanItem { row, encoded_components: ... })
}
```

Delete the old branch:

```rust
if layout.covers_full_row { ... } else { fetch current row }
```

## Segment Publication

Current segment publication builds columns by scanning primary index at `storage.rs:660-716`.

Update to scan covering access layout through `StorageSchema::covering_layout`.

Since every access path covers full tuple, the segment builder should decode field bytes from the covering access key, not current-row payload.

## Dictionary Ownership

String and bytes interning remains owned by the dictionary DB.

Do not implement dictionary GC here.

Ownership rule:

- User `Row` owns `String` and `Vec<u8>`.
- `EncodedTuple` owns intern IDs only, not raw strings/bytes.
- LMDB dictionary owns raw string/bytes payloads.
- Query output decodes intern IDs back to owned `String`/`Vec<u8>`.

## History

Current history stores primary bytes plus optional old/new payloads at `storage.rs:978-1009`.

New history should store the full covering tuple bytes for old/new facts.

Recommended:

```rust
fn append_history(
    &mut self,
    op: u8,
    relation_id: RelationId,
    old: Option<&EncodedTuple>,
    new: Option<&EncodedTuple>,
) -> Result<()>;
```

Do not preserve primary-key bytes in history. They are not meaningful in v2.

## Delete Required Old APIs

Delete or rewrite:

- `EncodedRow::payload`
- `EncodedRow::from_payload`
- `encode_primary_key`
- `encode_primary_key_existing`
- `primary_bytes`
- `current_row_key`
- `NS_CURRENT_ROW`
- all current-row payload gets/puts/deletes

## Non-Goals

- Do not implement set insert idempotency here unless it naturally falls out. PRD 08 formalizes behavior.
- Do not implement dictionary GC.
- Do not preserve current row payload fallback.
- Do not support old databases.

## Tests Required

- Inserted row is scannable from covering access path.
- Inserted row is scannable from secondary access path without current-row lookup.
- Segment publication builds columns from covering access path.
- Query image fallback works without `primary` and without current-row payloads.
- `get_row` either deleted or replaced by exact-row existence helpers.
- No storage test depends on `KeyValues`.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- Grep for `NS_CURRENT_ROW` returns no real code references.
- Grep for `current_row_key` returns no real code references.
- Grep for `primary_bytes` returns no real code references.
- Grep for `encode_primary_key` returns no real code references.
- Grep for `covers_full_row` returns no branchy runtime code references. It may be removed entirely.

## Completion Criteria

- Current storage has one fact/index namespace, not row payload plus indexes.
- Any access path key can decode the full row.
- Storage hot path no longer builds `BTreeMap<String, Vec<u8>>` for encoded rows.
- Segment and query-image code read from covering tuple keys.
