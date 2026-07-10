# PRD 05 — The enum funeral

**Depends on:** 02 (the replacement exists), 03 (its images exist), 04 (its
statements exist).
**Modules:** everything the type touched: `schema.rs`, `schema/validate.rs`,
`encoding/`, `image/` (decode + `ColumnView::Bytes` byte-column path),
`ir.rs` + `ir/validate/`, `api/prepared/result_buffer.rs`,
`bumbledb-macros/src/lib.rs`, `error.rs`.
**Authority:** the algebra set's cut-direct policy; README vocabulary
discipline.
**Representation move:** deletion as the proof. Enum was a vocabulary wearing
an encoding; with closed relations landed, every enum special case in six
subsystems is dead weight, and this PRD removes the type so completely that
`grep -ri "ValueType::Enum\|Cell::Enum\|TAG_ENUM\|variant"` over engine
crates returns nothing. The tree may not typecheck until the last cut lands —
that is the policy working.

## Context (decided shape)

The roster after this PRD: **u64, i64, bool, str, bytes<N>, interval** — six
types, all pure value types. Every field that was `enum K { ... }` becomes
`u64 as KId` referencing a closed relation, with its containment declared.

**The deletion ledger** (exhaustive; the executor works this list top to
bottom and every line is a `[shape]` criterion):

1. `ValueType::Enum { variants }` — the variant and every match arm on it.
2. The 1-byte ordinal encoding: `encoding/` enum arms; the image's 1-byte
   column path exists for `bool` too — KEEP the byte-column machinery
   (`ColumnView::Bytes`), delete only the enum decode/encode arms and the
   ordinal range-check corruption arm (`CorruptionError` variant included).
3. `Cell::Enum(u8)` + `ResultValue::Enum` + the `word_cell`/`push_word` enum
   arms in `result_buffer.rs`.
4. IR: `Value::Enum` (or its post-algebra equivalent), enum literal typing in
   `ir/validate/`, enum membership-set handling (sets of closed refs are
   plain u64 sets — machinery already exists).
5. Validation roster: `EnumWithoutVariants`, `EnumTooManyVariants`,
   `DuplicateEnumVariant` errors and their checks (their *jobs* moved to PRD
   01's closed-relation roster).
6. Macro: the inline `enum` field grammar (`k: enum K { A, B }`) — parsing,
   `emit_enums`, variant-ordinal resolution in `value_expr`. Handle
   resolution (PRD 02) is the survivor; the deleted vocabulary gains *enum*
   with its replacement line ("a vocabulary is a closed relation").
7. Fingerprint: the enum-variant hashing arm (closed extensions hash instead,
   PRD 01); KEEP a fingerprint-moves test proving an enum→closed rewrite of
   the same theory produces a different fingerprint (it is a different
   theory — no store compatibility, no migration, per policy).
8. Selection literals: `| kind == Variant` now resolves through PRD 02's
   handle path exclusively; delete the enum-ordinal path from the macro and
   the engine's selection typing.
9. Every in-tree fixture/test theory using `enum` is rewritten to closed
   relations (the bench-crate theories too — PRD 06 owns the oracle
   *semantics*; this PRD mechanically rewrites the schemas so the workspace
   converges).

## Technical direction

Work the ledger in order; expect the tree red from step 1 until step 9. Do
not alias, deprecate, or feature-gate anything. For step 9, the rewrite rule
is mechanical: `k: enum K { A, B }` → `k: u64 as KId` + top-level
`closed relation K as KId = { A, B };` + `Rel(k) <= K(id);` — apply it
verbatim, then re-run the schema through PRD 01's validation to catch
residue. Where a test asserted enum-specific behavior (ordinal caps, variant
errors), the test moves to the closed-relation roster equivalent or dies
with the feature (record which, per test, in the commit body).

## Passing criteria

- `[shape]` The greps: `ValueType::Enum`, `Cell::Enum`, `ResultValue::Enum`,
  `emit_enums`, `EnumWithoutVariants|EnumTooManyVariants|
  DuplicateEnumVariant`, and the macro grammar token `enum` inside relation
  blocks — all zero hits across `crates/` (the *host* enums emitted by PRD
  02 are named by their theory, not by the token `enum` in the grammar).
- `[shape]` `10-data-model.md` lists six types; the deleted-vocabulary table
  carries *enum → closed relation*.
- `[test]` A rewritten fixture theory (DU arms discriminated by a closed
  reference) validates, commits arm-consistent writes, and rejects
  arm-violating ones — proving the DU pattern survived the discriminator's
  type change intact.
- `[test]` The fingerprint-moves test from ledger step 7.
- `[gate]` Workspace gates green at campaign close (this PRD plus 06 restore
  the tree).

## Doc amendments (rule 5)

`10-data-model.md`: the six-type roster, enum's obituary with the rewrite
rule. `70-api.md`: grammar section loses the inline enum production. Repo
`README.md`: the theory-grammar table swaps the enum row for the
closed-relation row (completing PRD 02's addition).
