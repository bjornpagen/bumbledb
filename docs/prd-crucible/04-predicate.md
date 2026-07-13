# PRD 04 — The predicate: one signature, one derivation

**Depends on:** 01 (toolchain). The heart of Phase B.
**Modules:** `crates/bumbledb/src/ir/validate.rs` (+ `validate/`,
`finds.rs`), `api/prepared.rs` (`column_types`), `api/prepared/build.rs`
(`result_types` and the FindSpec construction), `api/prepared/finalize.rs`
(`all_words`), `exec/sink.rs` (SinkSpec construction inputs),
`api/stats.rs` (EXPLAIN header), tests throughout.
**Authority:** the head-typing dual's bug history (validation's
`head_types()` returned the INPUT positional row; `Count`/`CountDistinct`
broke it during the statically-empty work; `result_types()` was the
workaround) — the one live defect-adjacent seam the audits found. Policy
8: pin first.
**Representation move:** a query IS a predicate definition — one
anonymous predicate whose typed signature is currently derived three
times (`head_types`, `result_types`, `column_types`). Reify it once, at
validation, and every consumer reads the same object. Recursion-readiness
(a signature is IDB typing) is the free byproduct, not the
justification.

## Context (decided shape)

```rust
/// The predicate a query defines — anonymous (names live in the host,
/// exactly like relations pre-`as`), its typed output signature derived
/// ONCE at validation and sealed. The single authority for sink
/// construction, result-buffer typing, finalize's all-words decision,
/// and EXPLAIN's header. Referenced by NOTHING — the named-view refusal
/// stands; a reference to a predicate is the recursion trigger firing.
pub struct Predicate {
    pub columns: Box<[PredicateColumn]>,
}
pub struct PredicateColumn {
    /// The RESULT type — what lands in the buffer. Count is U64 here
    /// whatever it counted; Duration's measure is U64; Min/Max/Sum
    /// carry their input's type; Pack carries the interval type; the
    /// Arg forms carry the projected payload's type.
    pub ty: ValueType,
    /// None = plain projection; Some = the fold producing the column.
    /// Kept together deliberately: the sink needs both jointly, and a
    /// signature-only split would re-create a parallel table (decided
    /// here, not inherited from the sketch).
    pub op: Option<AggKind>,
}
```

- `validate(query) → ValidatedQuery { predicate: Predicate, rules:
  Box<[RuleWitness]> }` — the sealed witness gains the predicate; the
  per-rule alignment rule restates with identical semantics: *every rule
  derives the predicate* (each rule's derived row type must equal the
  signature; positional head alignment is how, the signature is what).
- `PreparedQuery { predicate: Predicate, program: Program, … }` — the
  predicate sits BESIDE the program because `Program::Empty` still has
  an arity and buffer types (`out.arity` today reads `column_types.len()`
  on the empty path).
- **Deleted outright, zero lingering consumers** (the 15 current grep
  hits): `RuleWitness::head_types()` and every reader;
  `build.rs::result_types()` whole; `PreparedQuery.column_types` (the
  field — readers move to `predicate.columns[i].ty`); any test helper
  duplicating the derivation.
- The public IR is UNTOUCHED: hosts still write `head: Vec<HeadTerm>`;
  `Predicate` exists only in the validated witness and the prepared
  query. If it appears in `ir.rs`, the PRD failed.

## Technical direction

1. **Pin first (policy 8):** an exhaustive signature table test against
   CURRENT behavior — one row per `HeadOp`/`FindTerm` form × each legal
   input type (plain var per type; Count/CountDistinct over each; Sum/
   Min/Max over the integer types; Duration; Pack; the Arg forms with
   projected payloads; multi-column heads mixing them). The table's
   expected values are read off today's `result_types`/sink behavior and
   hand-verified against `20-query-ir.md`'s aggregate typing prose. Land
   the test GREEN against current code before any refactor line.
2. Land `Predicate`/`PredicateColumn` + the `ValidatedQuery` shape; the
   derivation lives in `ir/validate/finds.rs` territory (where head/find
   typing already happens) as the ONE function; wire the table test to
   it.
3. Cut the consumers: build's FindSpec construction reads
   `predicate.columns`; `all_words` reads it; the buffer arity and
   `word_cell`/`push_word` typing read it; EXPLAIN's header renders it;
   delete the three old derivations and the field.
4. Sweep tests: anything constructing `column_types` or calling the dead
   fns re-anchors mechanically; assertion VALUES never change.
5. The fence, in code: `Predicate`'s doc carries the no-references
   sentence verbatim (the decided shape above); nothing exports a
   constructor besides validation.

## Passing criteria

- `[test]` The signature table test — written first, green before and
  after (the diff shows it unchanged while the derivations underneath
  it collapsed).
- `[shape]` `grep -rn "head_types\|result_types\|column_types" crates`
  → zero hits.
- `[shape]` `Predicate` appears in `ir/validate` and `api/prepared`
  only — `grep -n "Predicate" crates/bumbledb/src/ir.rs` → zero.
- `[test]` Every existing aggregate/measure/buffer test green with
  unchanged assertions; `Program::Empty` executions still produce
  correctly-shaped empty buffers (the arity test).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`20-query-ir.md`: the head section states the predicate concept (a query
defines one anonymous predicate; rules derive it; the signature is the
result-type row) and the no-reference fence with its trigger.
`70-api.md`: the prepared-query surface names the predicate as the
buffer-typing authority.
