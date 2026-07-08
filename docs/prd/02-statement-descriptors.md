# PRD 02 — Statement descriptors replace constraints

**Depends on:** 01.
**Modules:** `crates/bumbledb/src/schema.rs`, `crates/bumbledb/src/schema/relation.rs`,
`crates/bumbledb/src/schema/runtime.rs`, `crates/bumbledb/src/error.rs` (schema error types only).
**Authority:** `docs/architecture/30-dependencies.md` (judgments, statements), `10-data-model.md` (fingerprint inputs, serial auto-key).

## Goal

Delete `ConstraintDescriptor` (`Unique`, `ForeignKey`) and the per-relation
constraint list entirely. Dependencies become **schema-level statement
descriptors** with schema-global, materialized-order ids.

## Technical direction

1. New descriptor types in `schema.rs`:
   ```rust
   pub struct StatementId(pub u16);            // schema-global, materialized order
   pub enum LiteralValue {                     // selection literals; one variant per type
       Bool(bool), U64(u64), I64(i64), Enum(u8),
       IntervalU64(u64, u64), IntervalI64(i64, i64),
       String(Box<[u8]>), Bytes(Box<[u8]>),
   }
   pub struct Side {
       pub relation:   RelationId,
       pub projection: Box<[FieldId]>,                    // ordered, the statement's written order
       pub selection:  Box<[(FieldId, LiteralValue)]>,    // σ; empty = unselected
   }
   pub enum StatementDescriptor {
       Functionality { relation: RelationId, projection: Box<[FieldId]> },  // R(X) -> R
       Containment   { source: Side, target: Side },                        // source <= target
   }
   ```
   There is **no bidirectional variant**: `==` is lowered (by the macro, PRD 05, and
   by any hand-built descriptor) to two `Containment` statements with the sides
   swapped. Statements are **anonymous** — no `name` field exists anywhere.
2. `SchemaDescriptor` becomes `{ relations: Vec<RelationDescriptor>, statements: Vec<StatementDescriptor> }`.
   `RelationDescriptor` loses its `constraints` field entirely.
3. **Materialization** (in validation, PRD 03, but the ordering rule is owned here):
   the sealed statement list = one auto-`Functionality` per `Serial` field (relation
   declaration order, then field order; projection = the one serial field), followed
   by declared statements in declaration order. `StatementId` = index into that
   list. This replaces the old per-relation auto-unique materialization — delete it.
4. Sealed runtime form (what `Schema` holds after validation):
   ```rust
   pub struct Statement { pub descriptor: StatementDescriptor, pub resolved: Resolved }
   pub enum Resolved {
       Functionality { interval_position: Option<usize> },   // index into projection; None = scalar key
       Containment {
           target_key: StatementId,             // the Functionality statement probed on the target
           key_permutation: Box<[u16]>,         // statement projection order -> target key order
           interval_position: Option<usize>,    // positional index shared by both sides
       },
   }
   ```
   `Resolved` is computed by PRD 03; this PRD defines the types and threads them
   through `Schema`/`Relation`. Per-relation derived indices on the sealed
   `Relation`: `keys: Box<[StatementId]>` (Functionality statements on this
   relation), `outgoing: Box<[StatementId]>` (Containments whose source is this
   relation), `incoming: Box<[StatementId]>` (whose target is this relation).
   These replace `unique_constraints` and `fk_targeted` — delete both.
5. `schema/runtime.rs` (the descriptor-construction helpers used by the macro and
   tests) is rewritten against the new types. Every function or type containing
   `unique`, `fk`, `foreign`, or `constraint` in its name in the schema module is
   deleted, not renamed-and-kept.

## Out of scope

Validation logic (PRD 03), fingerprint (PRD 04), macro parsing (PRD 05), storage
key layouts (PRD 06). The tree will not compile after this PRD — expected.

## Passing criteria

- `[shape]` `ConstraintDescriptor`, `ConstraintId`, `unique_constraints`, and
  `fk_targeted` no longer exist anywhere under `crates/bumbledb/src/schema/`.
- `[shape]` `rg -i 'unique|foreign|constraint' crates/bumbledb/src/schema/` returns
  no identifier hits (string/comment hits describing SQL or history are permitted
  only in doc comments that cite the architecture docs).
- `[shape]` `StatementDescriptor`, `Side`, `LiteralValue`, `StatementId`,
  `Resolved` exist exactly as specified; no `name` field on any of them.
- `[shape]` The materialization-order rule (serial auto-FDs first, declaration
  order second) is implemented in exactly one function, with a doc comment citing
  `10-data-model.md`'s fingerprint section.
- `[test]` A unit test constructs a two-relation descriptor with one serial field
  each plus two declared statements and asserts the materialized `StatementId`
  assignment (auto-FDs are ids 0 and 1; declared statements 2 and 3).
