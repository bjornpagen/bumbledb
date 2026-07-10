# PRD 20 — The data surface, ruled: schemas are code, queries are data

**Depends on:** 05 (the rules-shaped IR is the surface being ruled on);
composes with 15 (the renderer serves the oracle's arbitration bundles too).
**Modules:** `crates/bumbledb/src/ir/` (validation-path panic sweep, render),
`crates/bumbledb-macros` (id-constant emission — emission, never grammar),
`schema.rs` (the manifest), docs.
**Authority:** `20-query-ir.md` (pure-data IR doctrine), `70-api.md`,
`00-product.md` (dependency law, the text-language OPEN item).
**Representation move — the code/data boundary is logic's own.** A schema is
the *theory*: signature plus axioms, fixed at build time, type-providing —
which is why `schema!` is structurally forced (type providers cannot live in
expression position) and why it is Rust's alone. A query is a *sentence in*
the theory: a runtime object, constructed and evaluated — data, in whatever
language the host speaks. The asymmetry is not an ergonomics compromise; it
is the same line logic draws between a theory and its formulas. The
pure-data-IR doctrine, recorded for testability, hereby gains its second
reader: **a foreign-function boundary can only carry data**, and the IR
already is data. Two prior refusals are vindicated by a requirement that did
not exist when they were made: the borrowed-results redesign (a
snapshot-lifetime result cannot cross a language boundary; the memoized
one-copy heap can) and the dyn write surface's typed-error discipline (it is
the portable half of the API, not ETL plumbing).

## Context (decided shape — owner-ruled 2026-07-10)

1. **`schema!` is the sole idiomatic schema surface, and its grammar is
   FROZEN**: relations + statements, forever. Emission may grow (structs,
   newtypes, the theory ZST, id constants); grammar may not. The descriptor
   path (`SchemaDescriptor` implementing the definition trait) remains the
   *data* schema surface — the bench crate, the oracle, and any future
   binding that needs runtime schemas — existing, not blessed.
2. **The query surface is the IR, permanently: pure data.** No builder API,
   no typed query variables, no text language, no ergonomic layer in the
   engine — ever. Any convenience syntax lives in a downstream package (in
   any language) and lowers to IR data; the engine never knows it exists.
   This supersedes the text-language OPEN item with a sharper ruling: sugar
   is downstream territory, in every language, permanently. The typed query
   builder considered on 2026-07-10 is **refused, recorded**: it would bind
   query construction to Rust's type system and closures — exactly what a
   foreign host cannot invoke — and its compile-time-checking dividend is
   re-provided by the roster (below), which foreign callers need anyway.
3. **Id constants and the manifest — named data, not ergonomics.** The macro
   emits declaration-order id constants on the theory
   (`Calendar::BUSY: RelationId`, `Calendar::BUSY_PERSON: FieldId`) so the
   Rust host never writes magic numbers; the theory renders a **manifest**
   (name → id, relations/fields/enums, from the descriptor it already
   builds) so a foreign host gets the same numbers as data. Both are
   emission; the grammar is untouched.
4. **The IR-validation path is a trust boundary.** Queries arrive as data —
   eventually foreign data — so every panic reachable from an `ir::Query`
   value is a crash a caller can trigger. The law, extended from the dyn
   surface's (`error.rs`: "ETL input is data, not code"): **no panic
   reachable from IR data**; validation, normalization, DNF lowering, and
   prepare return `Ok` or a typed error on *arbitrary* input — out-of-range
   ids, duplicate bindings, vacuous masks, cap-exceeders, hostile nesting.
   The caps (`MAX_RULES`, `MAX_OCCURRENCES`, `MAX_DISTINCT_VARS`, the DNF
   blowup cap) are reframed as boundary guards, not planner hygiene — they
   already exist; their reader list grows.
5. **`ir::render` — the read-side syntax.** The statement renderer's sibling:
   roster errors and EXPLAIN print the offending query in the docs' rule
   notation (`head(p, d) :- Busy(person: p, during: d), Allen(INTERSECTS)…`).
   When the write-side surface is data, the renderer *is* the pretty syntax
   — ergonomics on the side that costs nothing and crosses every boundary.
6. **JS/N-API bindings are explicitly PUNTED — pure anticipation, zero
   deliverable.** Recorded shape for whenever the owner wants them: a
   quarantined downstream crate on the bench-crate precedent (it may hold
   the N-API dependency; the engine never depends on it; no engine decision
   may lean on its existence), compiling the application's `schema!` in,
   exposing prepared-query handles, the dyn read/write surfaces, and the
   manifest; marshaling IR-as-data in and result copies out. Nothing in this
   set builds any of it; this PRD exists so the engine-side surface is
   *already correct* the day it is wanted.

## Technical direction

1. **The panic sweep**: a property test drives arbitrary structurally-random
   `ir::Query` values (the querygen machinery inverted — generate *invalid*
   shapes deliberately: unknown ids, arity mismatches, dup rules, cap+1,
   mask ∅/full, MAX-point literals) through validate→normalize→prepare and
   asserts every outcome is `Ok` or a typed error. Any panic is a red run.
   `unreachable!` arms downstream of validation are exempt (they are
   guarded by it — the point of the sweep is proving the guard total).
2. Macro: id-constant emission per relation/field/enum-variant;
   `Theory`-level manifest render (plain data out of the descriptor — no
   serde, the dependency law stands; the manifest is a Rust value the
   downstream binding serializes however it likes).
3. `ir::render`: rule notation, deterministic, used by `SchemaError`-class
   query errors and the stats/EXPLAIN surface; golden-tested.
4. Docs: the two-surface framing ("the theory surface / the data surface"),
   the grammar freeze, the trust-boundary law, the punt record.

## Passing criteria

- `[test]` The adversarial-IR property test: 10⁴+ random malformed queries,
  zero panics, every rejection a typed error naming its roster line.
- `[test]` `ir::render` goldens: the calendar union query and one
  Pack/Duration head render to the documented notation byte-exactly.
- `[shape]` No query builder, no query macro, no text-language surface
  exists in the engine (grep); the grammar-freeze sentence is in
  `70-api.md`; the punt record and the two refusals (builder; engine-side
  sugar) are in the refusals ledger.
- `[shape]` Id constants exist per relation/field; the manifest is
  reachable from the theory; no serde/N-API dependency exists anywhere in
  the engine workspace (the dependency law's grep, extended).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`20-query-ir.md`: the surface ruling, the trust-boundary law, the renderer.
`70-api.md`: the two-surface framing, the grammar freeze, the manifest, the
punt record. `00-product.md`: the text-language OPEN item superseded by the
sharper ruling; the anticipated-binding note added to the non-goals with its
quarantine shape. Architecture README: OPEN list updated accordingly.
