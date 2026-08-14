# schema-010: descriptor `RelationDescriptor.extension: Option` is the hostile spelling — recorded so nobody "fixes" it

- **Severity:** low
- **Tree:** schema
- **Status:** WONTFIX (non-violation recorded by the audit itself)
- **Source:** audit/storage-schema.md F25
- **Depends on:** none

`RelationDescriptor.extension: Option<Extension>` (`bumbledb-theory/src/schema.rs:416-425`) is Dijkstra's hostile boundary done correctly: the untrusted declaration admits the Option so `SchemaDescriptor::validate` can refuse `EmptyExtension` / `StrOnClosedRelation` / `FreshOnClosedRelation` / … by name. The codec already stores a sum (tag byte 0/1, `descriptor_codec.rs:111-120`). Analog of engine-037 / CONTRACT C1: do **not** introduce a third relation type at the descriptor, and do not regenerate fingerprints.

The sealed `Relation` is where Option must die — that is schema-001. Charge every `is_closed()` / `extension.is_some()` forest to schema-001, not to this constructor. No edit under this id.
