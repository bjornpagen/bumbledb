# 00 — Doctrine: representation over control flow, and one way to do each thing

This document is the ruling the rest of the set applies. It is not
advocacy; it is the operating rule for every design decision in 10–80, with
the lineage that grounds it and the register of violations this set exists
to remove. When ratified, it graduates verbatim to `audit/REQUIRED-READING.md`
(the file the top-level proposals README already cites).

## The principle

**The data representation determines a program's complexity. The algorithm
and the control flow are downstream of it.**

When a new case shows up there are two ways to absorb it. Patch the trace —
a branch, a flag, a guard, a second transport, a fallback — and complexity
gathers in the control flow, forever, at every point the value travels.
Or change the structure — the data, the types, the invariants — so the case
stops being special, or stops being expressible at all. Same problem, two
surfaces, opposite cost profiles: the branch is free now and expensive
forever; the representation is expensive once and free forever.

The lineage is explicit and cited, four practitioners across thirty-one
years, two of them citing the first by page:

- **Brooks, 1975** (*Mythical Man-Month*, ch. 9, under the heading
  "Representation Is the Essence of Programming"): "Show me your
  flowcharts and conceal your tables, and I shall continue to be mystified.
  Show me your tables, and I won't usually need your flowcharts; they'll
  be obvious." And: "strategic breakthrough will come from redoing the
  representation of your data or table. This is where the heart of a
  program lies."
- **Pike, 1989** (*Notes on Programming in C*, Rule 5, citing Brooks
  p. 102): "Data dominates. If you've chosen the right data structures and
  organized things well, the algorithms will almost always be
  self-evident." And later: algorithms "can often be encoded compactly,
  efficiently and expressively as data rather than, say, as lots of if
  statements."
- **Raymond, 1997** (*Cathedral and the Bazaar*, lesson 9, attributing
  Brooks): "Smart data structures and dumb code works a lot better than
  the other way around" — learned by replacing fetchmail's protocol
  branching with a method table.
- **Torvalds, 2006** (git list): "Bad programmers worry about the code.
  Good programmers worry about data structures and their relationships…
  I'm a huge proponent of designing your code around the data, rather than
  the other way around."

The type-theoretic floor under the slogan: **Minsky** — make illegal states
unrepresentable (a sum type admits exactly the valid states, so the guards
have nothing left to guard); **King** — parse, don't validate (a validator
checks and discards its proof, so every downstream layer re-checks; a
parser returns a type that *carries* the proof, so the check happens once
at the boundary); **Hoare** — null is the counterexample (a quasi-member of
every type is exactly why every use must branch); **Reynolds/Wadler** — a
polymorphic signature is an enforced specification (well-typed clients
provably cannot branch on the representation); **Dijkstra EWD831** — some
special cases are coordinate artifacts (the half-open interval does not
*handle* the off-by-one, it makes it unrepresentable — this repo's
`FreshRange`/interval conventions already live by it).

And the limit that keeps it honest — **Brooks again** (*No Silver Bullet*):
representation collapses *accidental* complexity, never *essential*
complexity. Forcing two genuinely different cases into one representation
does not remove the branching; it hides it inside flags. Every ruling in
this set names which side of that line it stands on.

## The operational rule: one way to do each thing

The SDK already enforces canonical utterance for *user* inputs — one
meaning, one spelling: duplicate statements are refused, a one-element
literal set is "the bare literal respelled", implied keys may not be
restated. This set applies the same law to the system's **own**
representations and surfaces:

1. **One physical form per value per boundary.** A collection crosses the
   host→engine boundary in exactly one representation, built exactly once,
   consumed by borrow everywhere downstream. A second form is not an
   optimization; it is a copy of the first form plus the control flow to
   keep them agreeing.
2. **One public spelling per operation.** Two public spellings of one
   semantic operation (row objects *and* column batches; scan-and-measure
   *and* aggregate-count *and* exact count) are a standing invitation for
   callers to branch on transport. Where a second spelling exists only as
   a performance workaround, the fix is to make the one spelling fast and
   delete the second — never to document which to use when.
3. **One law, one judge per boundary — every boundary.** A law enforced at
   the engine but not at the SDK is not "deferred"; it is two boundaries
   disagreeing, and the caller discovers the disagreement at the most
   expensive possible moment. The house pattern is the *runtime twin*: the
   same wall at the type tier (best effort, degrades on widened tuples)
   and the value tier (authoritative, always on), both mirroring the
   engine's judgment exactly. Two **tiers** of one wall is not two ways —
   it is one law stated where each audience can hear it.
4. **Proofs travel in types, not in re-checks.** When a boundary has
   proved something (arity, shape, well-formedness, key resolution), the
   representation it hands downstream must carry that proof so downstream
   *cannot* re-check it. A re-check is a branch guarding an impossible
   state.

## The violation register

Every defect both upstream reports name, restated as the representation
violation it is. The doc column owns the fix.

| # | Violation | Where it lives today | Doc |
| --- | --- | --- | --- |
| V1 | One collection, six-plus physical forms: Primer's column transpose → per-fact JS row arrays (`rowsOf`) or a second full column copy (`columnsOf`) → per-row bridge `Vec<Value>` inside `Vec<Vec<Value>>` → per-row engine `ParsedRow`/`Box<[ParsedCell]>` → encoded bytes. The bridge's shape proof is discarded and re-derived by the engine's parse — validation, not parsing. | `ts/src/db.ts` (`rowsOf`, `columnsOf`, `mutateCollection`), `ts/crate/src/marshal.rs` (`fact_rows`, `fact_columns`, `one_fact_row`), `crates/bumbledb/src/api/db/encode_dyn.rs` (`ParsedRow`), `mutation_core.rs` (`parse_dyn_collection`) | 20 |
| V2 | Two public spellings of one write: `CollectionWrite<R> = Iterable<Fact<R>> \| ColumnBatch<R>`, with paired native entries (`txInsert`/`txInsertColumns`, `instanceBuilderLoad`/`instanceBuilderLoadColumns`) — the column arm exists only because the row arm is slow, and `fact_columns` rebuilds rows anyway | `ts/src/db.ts:87-138`, `ts/src/native.ts`, `ts/crate/src/lib.rs` (`tx_insert_columns`, `instance_builder_load_columns`) | 20, 70 |
| V3 | Error-context strings built on the success path: one `format!` per **cell** in `schema_value` and `fact_columns`, per row in `one_fact_row` — ~25–30 M alloc/free pairs per Primer run that are never read | `ts/crate/src/marshal.rs:199` and siblings | 20, 70 |
| V4 | Per-call re-derivation of the sealed field roster (`Vec<(Box<str>, ValueType)>` per insert call) of a roster that is immutable per handle | `ts/crate/src/marshal.rs::sealed_fields` | 20, 70 |
| V5 | One string, three copies and N probes: NAPI copy → `String` → `Box<str>`; committed strings pay blake3 + one LMDB get **per occurrence** because only pending mints are memoized | `ts/crate/src/marshal.rs::schema_value`, `crates/bumbledb/src/storage/delta/intern.rs` | 30 |
| V6 | The maintained exact cardinality (`StatKind::RowCount`, transactional since format 8, O(1) to read, pinned equal to scan count) is `pub(crate)` + `allow(dead_code)` at every public layer, so callers count by decoding 4 M facts or by full-relation aggregate queries — and the aggregate returns the empty set for an empty input, forcing a caller-side branch to reinterpret absence as zero | `crates/bumbledb/src/api/db/read_instance.rs:32`, `owned.rs:229` | 40 |
| V7 | A lawful law with no spelling: `v(R)` **is** the full binding of `R`, but `match`'s signature cannot state it for generic `R` — `VarsOf` maps over `keyof … & string` while `MatchShape` maps over bare `keyof`, and tsc cannot relate the deferred mapped types — so hosts carry type suppressions | `ts/src/query/scope.ts:146`, `ts/src/query/atom.ts:241`, six `match` sites in `ts/src/query/lower.ts` | 50 |
| V8 | One containment law, two admission answers: the engine requires the target projection to set-match a declared key (`resolve_target_key`, Lean-priced); `schema()` documents the check as "DELIBERATELY left to the engine" and `lower()` emits the inadmissible schema; the eventual refusal speaks ids, not names | `ts/src/schema.ts:1-10`, `ts/src/statements.ts:27-34`, `crates/bumbledb/src/schema/validate.rs` | 60 |
| V9 | Attribution by folklore: the Node profile cannot split the native `commit` frame (12,235 samples) into plan, judgment, dictionary flush, index application, and LMDB commit; accumulation-side phases (marshal, parse, intern, delta apply) have no spans at all | `crates/bumbledb/src/obs/point.rs` (commit spans exist; accumulation spans absent) | 10 |
| V10 | Consumers compensating: Primer's 16,384-row column transpose (`columnBatch`/`loadColumns`) and scan-based `countRelations` are host-side patches over V1–V6 | `primer-spec/src/storage/bumbledb/runtime.ts` | 70, 80 |

Reading the register through the doctrine: V1/V3/V4/V5 are *parse, don't
validate* failures (proofs discarded, work re-done); V2 and the count
triple in V6 are *two spellings of one meaning*; V6's empty-set-vs-zero and
V7 are *illegal or inexpressible states* forcing caller branches; V8 is
*one law, two judges*; V9 is the attribution-first house law unmet; V10 is
what downstream teams build when V1–V8 stand. None of these is essential
complexity. All of them are accidental, and every one is removed by a
representation change, not by a new branch, flag, mode, or config.

## What this set refuses to do

- **No flags, no modes, no fallbacks.** There is no "fast path" toggle, no
  legacy transport kept "for compatibility", no config that selects a
  count strategy. Where a change is breaking (V2's column transport), the
  break ships with the replacement in the same release and the ledger in
  [70-deletions.md](70-deletions.md) names every affected caller.
- **No speculative generality.** The accepted collection is not a public
  extension point; the count is not a statistics framework; the type-tier
  wall is not a general dependent-type experiment. Each fix is exactly as
  large as its violation.
- **No unmeasured representation choices.** 20's physical form is pinned
  by 10's gates, not by taste — the one place this set defers a decision,
  it defers it to a measurement, and the deferral has a deadline (the
  gate), not a mood.
