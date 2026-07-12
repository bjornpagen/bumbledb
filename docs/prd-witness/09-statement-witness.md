# PRD 09 — Statement becomes a witness: the sum and its ids

**Depends on:** Phase A complete (clean tree); first of the strict spine
09→10→11. **The tree will not compile when this PRD lands alone — that is
the policy working; PRDs 10–11 restore it.**
**Modules:** `crates/bumbledb/src/schema.rs`, `schema/validate.rs`,
`schema/relation.rs`, `schema/tests/*`.
**Authority:** the set's organizing principle; the audit's flagship
finding (19 re-assertion sites); policy 7 (the fingerprint is
load-bearing: the DECLARATION surface is untouched).
**Representation move:** the sealed `Statement` is today a sum stored as
three parallel fields — `descriptor: StatementDescriptor` ∥
`resolved: Resolved` ∥ `checks: Option<CompiledSides>` (+ `mirror`,
always-None for FDs) — whose variant agreement every consumer re-asserts
with `let … else unreachable!`. One sum type with homogeneous typed
arenas makes the agreement unrepresentable and the accessors **total**.

## Context (decided shape)

The new sealed shape, verbatim (field visibility follows current
conventions — pub where `Statement`'s fields were pub, pub(crate) where
`checks` was):

```rust
/// Witness index into Schema.keys — minted only by validate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyId(pub(crate) u16);
/// Witness index into Schema.containments — minted only by validate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContainmentId(pub(crate) u16);

/// One sealed key statement: `R(X) -> R` with its enforcement flag.
pub struct KeyStatement {
    /// The materialized-order identity — fingerprint-pinned, embedded in
    /// `U` storage keys and error payloads. Never an index into anything.
    pub id: StatementId,
    pub relation: RelationId,
    pub projection: Box<[FieldId]>,
    pub pointwise: bool,
}

/// One sealed containment: both sides, the enforcement plan, the
/// compiled σ checks (no Option — FDs cannot reach this type), and the
/// `==` pairing.
pub struct ContainmentStatement {
    pub id: StatementId,
    pub source: Side,
    pub target: Side,
    pub(crate) enforcement: Enforcement,
    pub(crate) checks: CompiledSides,
    pub mirror: Option<StatementId>,
}

/// The enforcement plan — the old `Resolved`, minus the Functionality
/// arm (keys carry `pointwise` inline).
pub(crate) enum Enforcement {
    Probe { target_key: KeyId, key_permutation: Box<[u16]>, coverage: bool },
    Closed { members: [u64; 4] },
}

/// StatementId -> which arena, which slot. The fingerprint's global
/// materialized order survives as this spine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementRef { Key(KeyId), Containment(ContainmentId) }

pub struct Schema {
    relations: Box<[Relation]>,
    keys: Box<[KeyStatement]>,                 // KeyId indexes — total
    containments: Box<[ContainmentStatement]>, // ContainmentId indexes — total
    order: Box<[StatementRef]>,                // StatementId indexes — total
    dependents: Box<[Box<[ContainmentId]>]>,   // per KeyId — total, typed
}
```

Deleted outright: `Resolved` (whole enum), `Statement` (struct),
`CompiledSides`' `Option` wrapping (the struct itself survives inside
`ContainmentStatement`), `Statement::key_projection` and
`Schema::key_projection`'s panic paths, the `dependents`-indexed-by-
StatementId shape. `Relation.keys: Box<[StatementId]>` →
`Box<[KeyId]>`; `Relation.outgoing` → `Box<[ContainmentId]>`.

Untouched, by policy 7: `StatementDescriptor`, `Side`,
`SchemaDescriptor`, `materialized_statements()`, everything
`schema/fingerprint.rs` hashes, the `manifest`, the macro emission, and
`StatementId` itself (error payloads, `U`/`R` storage keys, EXPLAIN all
keep speaking it).

New accessor surface on `Schema` (all total, no `# Panics` beyond
index-width programmer invariants):
`key(KeyId) -> &KeyStatement`, `containment(ContainmentId) ->
&ContainmentStatement`, `statement(StatementId) -> StatementRef`-based
lookup returning an enum view for display/render,
`dependents(KeyId) -> &[ContainmentId]`, `keys()`/`containments()` slice
accessors for the sweeps. The bounds-checked dynamic-surface siblings
mirror the current `relation_checked` convention.

## Technical direction

1. `schema.rs`: land the types above; delete `Resolved`/`Statement`;
   keep `closed_member` beside `Enforcement`.
2. `schema/validate.rs`: `validate()` builds the three arenas in one
   materialized-order pass — each descriptor becomes a `KeyStatement`
   (FD: `pointwise` from the interval-position computation, which stays
   local to validate) or a `ContainmentStatement` (resolution returns
   `Enforcement`; `compiled_checks` feeds `checks` directly — the zip
   phase dies, construction is single-site). `resolve_target_key`
   returns `Enforcement`; the closed branch returns
   `Enforcement::Closed`. The dependents map is built per `KeyId`. The
   `mirror_of` computation is unchanged (descriptor-level).
   `Relation.keys`/`outgoing` fill with witness ids.
3. `schema/relation.rs`: accessor types updated.
4. `schema/tests/`: `valid.rs`'s exact-`Resolved` assertions become
   exact-`Enforcement`/`KeyStatement` assertions (same pins, new shape);
   the member-set and refutation tests re-anchor. Add the two witness
   tests: (a) `dependents` is typed — a `KeyId` round-trips to its
   `KeyStatement` and every dependent resolves through
   `containment(...)` with no panic path in the signature; (b) `order`
   preserves materialized identity — for every `StatementId` i,
   `order[i]` resolves to a statement whose `.id == StatementId(i)`.
5. Everything downstream (storage, verify_store, chase, api, exec
   classify, render) breaks. DO NOT patch it here — PRDs 10–11 own the
   cuts. No shims, no re-exports of the dead names.

## Passing criteria

- `[shape]` `grep -rn "enum Resolved\|struct Statement\b" crates/bumbledb/src/schema.rs`
  → zero hits; `grep -rn "unreachable" crates/bumbledb/src/schema.rs
  crates/bumbledb/src/schema/validate.rs` → only index-width expects
  (statement/field counts), zero variant-agreement asserts.
- `[shape]` `checks` is not `Option` anywhere; `mirror` exists only on
  `ContainmentStatement`.
- `[test]` The two witness tests of direction 4; every pre-existing
  schema test green in its re-anchored form (`cargo test -p bumbledb
  --lib schema::` passes even while the wider tree is red — verify with
  targeted compilation if the crate splits allow, else defer the run to
  PRD 11's close and rely on `[shape]`).
- `[shape]` **The fingerprint pins are untouched**: zero diffs under
  `schema/fingerprint.rs` tests and the bench pin constant
  (`the_fingerprint_is_pinned` remains `63e3b480…` — checked at PRD 11's
  close when the tree compiles).
- `[gate]` Workspace gates green at campaign close (post-11).

## Doc amendments (rule 5)

`docs/architecture/30-dependencies.md` § enforcement: one paragraph — the
sealed statement is a sum with typed key/containment ids; validate is the
only mint; downstream matches are total ("the witness carries the
proof"). `10-data-model.md`'s statement-identity paragraph: StatementId
remains the materialized-order fingerprint identity; KeyId/ContainmentId
are sealed-arena witnesses, never fingerprint inputs.
