# 02: Schema, Types, And Encoding

**Goal**
- Define the typed schema representation and the byte encodings that make LMDB lexical order match logical order.

**Why This Stage Exists**
- Storage and query planning depend on stable schema descriptors and correct sortable encoding.
- Encoding mistakes are file-format mistakes, so this stage needs strong tests before the write path grows around it.

**Concrete Work**
- Define relation descriptors, field descriptors, type descriptors, index descriptors, and constraint descriptors.
- Define primitive logical types: `bool`, `u64`, `i64`, typed refs, timestamp, fixed-scale decimal, UUID, symbol, string, and bytes.
- Define generated-ID metadata for entity and event relations.
- Define composite identity metadata for edge and set relations.
- Implement sortable byte encoders for all fixed-width primitive types.
- Implement sign-bit-flipped encodings for signed integers, timestamps, and decimals.
- Implement interned-value placeholders for strings and bytes, without needing the full dictionary write path yet.
- Implement schema fingerprint generation from canonical schema descriptors.
- Implement key-layout validation against the LMDB max key size.
- Create one hand-written test schema descriptor for early tests before the macro exists.
- Add tests proving encoded byte order matches logical order for each ordered type.
- Add tests proving incompatible typed IDs are distinct at the descriptor/typechecking level.

**Out Of Scope**
- Full procedural schema macro if it slows progress.
- Full dictionary interning implementation.
- Datalog query parsing.
- Physical writes to relation indexes.
- Runtime schema evolution.

**Passing Criteria**
- Primitive encodings round-trip correctly.
- Ordered primitive encodings sort correctly with byte comparison.
- String and bytes fields are represented as interned IDs in index layouts.
- Schema fingerprints are deterministic.
- Changing a relation, field name, field type, index annotation, or constraint changes the schema fingerprint.
- A schema descriptor can compute all required current index layouts.
- A schema descriptor rejects an index key layout that could exceed LMDB's max key size.
- No relation index key needs runtime type tags for typed fields.
- The hand-written test schema is enough to drive the next storage stage.

**Notes**
- The schema macro is valuable, but it is not the first critical risk.
- Encoding order is more important than API polish here.
- Any future encoding change must be treated as a storage format change requiring ETL.
