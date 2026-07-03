# PRD 10 — Canonical results and multiset comparison

Authority: `50-validation.md` (value-level equality via the typed API).

## Purpose

One canonical result form and one comparison that both engines decode into —
mismatches must be undeniable and debuggable.

## Technical direction

- `compare::Row = Vec<Owned>` with `pub enum Owned { Bool(bool), U64(u64),
  I64(i64), Enum(u8), Str(String), Bytes(Vec<u8>) }` deriving
  `Ord/PartialOrd/Eq/Hash/Debug/Clone` (total order = canonical multiset order).
- From bumbledb: `compare::from_buffer(&ResultBuffer, &[ValueType]) -> Vec<Row>`
  via `rows()` + `PreparedQuery::column_types()`.
- From SQLite: `compare::from_sqlite(stmt, params, &[ValueType]) -> Result<Vec<Row>>`
  — typed getters per expected column type (INTEGER → the expected width with
  range checks; TEXT/BLOB direct; **aggregate columns**: Count → U64, Sum(I64) →
  I64, Sum(U64) → U64 with the `< 2^63` corpus bound asserted; the caller passes
  the result types, which the engine side already knows).
- `pub struct Mismatch { pub ours_only: Vec<Row>, pub theirs_only: Vec<Row>, pub
  ours_len: usize, pub theirs_len: usize }` and
  `compare::multisets(ours, theirs) -> Result<(), Mismatch>`: sort both, two-pointer
  diff, collect up to 8 exemplars per side.
- `impl Display for Mismatch`: counts + the exemplar rows, type-tagged — the
  arbitration artifact's body.

## Non-goals

Order-sensitive comparison (results are sets). Floating anything.

## Passing criteria

- Unit tests: equal multisets (shuffled) pass; a one-row difference reports it on
  the correct side; duplicate-count differences detected ([a,a] vs [a]); Display
  golden for a small mismatch; SQLite round-trip for all six types through a
  scratch table; U64-vs-I64 column confusion is a typed error, not a wrong pass.
- `scripts/check.sh` green.
