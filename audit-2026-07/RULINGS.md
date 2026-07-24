# Audit rulings — 2026-07-23

Every design decision the audit surfaced, ruled by the owner in one interactive pass.
Standing policy applied throughout, retroactively: **maximal churn, maximal elegance —
backwards compatibility is never weighed.** Each ruling lists its flush targets: the
normative docs and Lean modules that must state the law before the fix campaign begins.

## A. Aggregate semantics

### R1 — Cross-rule fold-free nullary Count: REFUSED at validation (001, 043)
**Status: IMPLEMENTED — `34f96cd8` (refusal + flipped tests), `5a8f89c0` (the Lean screen stated).**
A nullary `Count` in a fold-free head of a 2+-rule program is definitionally constant 1
under the head-projection law — an uninformative query. It becomes a typed validation
error beside `ArgAcrossRules`, same modeling answer: one Count per disjunct, host-merged.
The pinned acceptance tests flip to refusal tests.
- **Flush:** `docs/architecture/20-query-ir.md` (aggregation §, beside the Arg doctrine);
  `docs/research/aggregate-comparisons.md`; `crates/bumbledb/src/ir/validate/validate.rs`;
  Lean: refusal enters the validation model.

### R2 — OR + aggregate: FIX THE LOWERING (007)
**Status: IMPLEMENTED — `34f96cd8` (the provenance-keyed union dedup), `5a8f89c0` + `bf3558ef` (dnf_rekey_stream and the conformance glue), `a3e09fe6` (the OR-tree corpus raised tree-quantified).**
Surface `or` must be fold-transparent. DNF-derived rule sets re-key the union dedup on
the shared slot arrays (the variables all disjuncts bind), so disjunction widens
membership without changing the fold domain. Hand-written multi-rule programs keep the
head-projection law (and R1's refusal where the head is fold-free nullary).
- **Flush:** `docs/architecture/20-query-ir.md` (the or-transparency law, stated);
  `lean/Bumbledb/Query/Aggregates.lean` + `Exec/Dedup.lean` (normative denotation);
  naive oracle (`crates/bumbledb-bench/src/naive/query.rs`) aligns.
- **Note:** finding 027 (union-regime fold has no Lean law) is now mandatory, not optional:
  the re-keyed fold gets a normative Lean denotation and at least one theorem.

### R3 — bool is orderable: Any/All fall out free (068)
**Status: IMPLEMENTED — `592a5ffc` (every validation surface), `39f2d7d5` (the Lean order vocabulary).**
bool enters the orderable vocabulary with false < true. `Max` over bool = Any,
`Min` = All; the documented idiom becomes true on every surface. No new IR.
- **Flush:** `docs/architecture/10-data-model.md` (orderability roster);
  `20-query-ir.md`; `ir/validate/finds.rs`; TS types; Lean value ordering.

### R4 — Orderability wall moves into engine validation (069)
**Status: IMPLEMENTED — `592a5ffc` + `0e918f29` (the last two screens), `ebac3267` (the contradiction plant respects the wall).**
Ordering a closed reference is a typed IR-validation error on every surface, with an
explicit bool carve-out (per R3). The TS-only wall dies; the engine backstops the law.
- **Flush:** `docs/architecture/10-data-model.md` ("Orderability, complete" becomes an
  engine law); `70-api.md`; `ir/validate/context.rs` + `finds.rs`; typed error variant.

### R5 — Measure-keyed Arg: IMPLEMENT NOW (118)
**Status: PARTIAL — `113d5f1e` (ArgKey::Measure end to end with ray poisoning), `db1d8f74` (the Rust notation), `6aecf449` (the bench translator); OWED: the TS surface cannot yet utter the measure key (`909e5042` marshals Var only) and the Lean denotation keeps the conformance fence (`5b2958e6`).**
`ArgMax`/`ArgMin` restrictions may key on an interval measure ("longest interval per
group"). Lands inside the same aggregate-law revision as R1–R3 — the aggregate spec
reopens exactly once.
- **Flush:** `20-query-ir.md` (Arg key positions re-stated, "exhaustively" made true
  again); `docs/feature-register.md`; IR + validate + sink + both macro grammars + TS.

### R6 — Ray error semantics: Kleene three-valued fold (024)
**Status: IMPLEMENTED — `6353e6fc` (the naive Kleene fold), `88c73e92` (the engine ray verdict as data), `7db9206a` (the Lean Verdict3 evaluator).**
Error propagation through AND/OR is three-valued logic — order-independent, commutative,
agreeing with DNF lowering by construction. The naive oracle folds verdicts
commutatively; evaluation order is unobservable.
- **Flush:** `20-query-ir.md` (normative definition); naive oracle rewrite; Lean
  denotation where measures-of-rays appear.

## B. Schema, theory, grammar

### R7 — ClosedSpec fuses into one sum (128)
**Status: IMPLEMENTED — `816a352d` (the fused sum), `70d5fbd7` (spec twins cross), `909e5042` (the SDK consumes the fused wire).**
`RelationSpec` closedness becomes `Open | Closed { roster }`. The two illegal states
are unrepresentable. The public SchemaSpec bindings contract breaks; macros regenerate.
- **Flush:** `crates/bumbledb-theory/src/schema/spec.rs`; `crates/bumbledb-macros`;
  `docs/architecture/70-api.md`.

### R8 — Radix rule: full rustc set, everywhere (122, 123)
**Status: IMPLEMENTED — `d0afe2ef` (query!), `22b89f3f` (schema! joins the one parser).**
`0x`/`0o`/`0b` + underscores accepted uniformly in selections, widths, and window
bounds, in both macros. One shared literal parser owns the law (kills the three
divergent parsers). The renderer normalizes to canonical decimal — round-trip is
canonical-form, not verbatim.
- **Flush:** both macro crates converge on one parser; `20-query-ir.md` notation §;
  compile-fail estate updated.

### R9 — query! gains the full condition-tree grammar (129)
**Status: IMPLEMENTED — `727c3b1d` (the condition-tree grammar, round trip closed).**
`or()` and `and()` condition trees enter the sacred Rust text notation as an exact
mirror of the TS grammar. One condition language, two identical surfaces, one renderer.
Lands with/after R2 so the Rust surface never ships the leaky lowering.
- **Flush:** `crates/bumbledb-query-macros`; `20-query-ir.md` grammar §; renderer;
  new compile-fail estate; cookbook examples.

## C. API surface (Rust + TS)

### R10 — abandon() honored: WriteResult becomes a sum (060)
**Status: IMPLEMENTED — `58e63af9` (WriteResult sum, abandon rolls back).**
Returning `abandon(payload)` from a `db.write` callback rolls the transaction back.
`WriteResult` widens to a sum carrying commit-vs-abandon; the outcome is in the type.
- **Flush:** `ts/src/db.ts`; `70-api.md` write-path contract.

### R11 — Tx.insert returns {changed, ...fresh} (061)
**Status: IMPLEMENTED — `58e63af9` (insert returns {changed, ...fresh}), `f6ebe631` (the shadow refusal).**
The engine's changed-state boolean already crosses the FFI; the SDK stops discarding it.
- **Flush:** `ts/src/db.ts`, `ts/src/native.ts`; `70-api.md`.

### R12 — Resource lifetimes: Node 26 explicit resource management (066)
**Status: IMPLEMENTED — `fd49327f` (disposables + using idiom), `5ec60049` (the doc half).**
The SDK assumes the latest Node 26 runtime. `ExhumeHandle` implements
`Symbol.dispose`/`Symbol.asyncDispose` (whichever matches teardown reality); `using` /
`await using` is the documented idiom. Congruence audit: every SDK object holding a
native lifetime (exhume, snapshots/scoped reads) adopts the same protocol. The
zero-closables doctrine is restated as: lifetimes are disposables, never `close()`.
- **Flush:** `ts/crate/src/lib.rs`; `ts/src/exhume.ts` (+ any scoped-read surfaces);
  `70-api.md` resource-lifetime doctrine; ts README/COOKBOOK idioms.

### R13 — TS explain() lands (117)
**Status: IMPLEMENTED — `d9cfd6ef` (the FFI crossing), `fd49327f` (explain() on every read surface).**
Read-only plan introspection crosses the FFI: prepared query → plan-as-data
(FjPlan + counters). Diagnostic surface, explicitly unfrozen. ANALYZE/profiling stays
engine-side.
- **Flush:** `ts/crate/src/lib.rs`; `ts/src/db.ts`; `70-api.md` (diagnostic surface §).

### R14 — Closed-column const accessors emitted (125)
**Status: IMPLEMENTED — `f7c6bf4c` (const accessors on the host enums).**
Closed-relation column values are expansion-time constants; the macro emits `const`
accessors on host enums. The runtime-query workaround dies.
- **Flush:** `crates/bumbledb-macros`; `70-api.md` generated-surface roster.

### R15 — get_dyn uses the Db-owned scratch pool (045)
**Status: IMPLEMENTED — `d890f2aa` (the pooled point path + the extended alloc gate), `d2d541d9` (the composed-key seam).**
Point-read scratch is pooled on Db, symmetric with the WriteTx twins. Callers unchanged;
the point path goes allocation-free (with 010/011/046/113).
- **Flush:** `crates/bumbledb/src/api/db/snapshot.rs` + `get.rs`; alloc-gate extended
  to the read path; `70-api.md` allocation contract.

## D. Storage laws

### R16 — Fresh ids and row ids merge into one allocator (047)
**Status: IMPLEMENTED — `4d19deb8` (format v6, one allocator), `d890f2aa` (the read half), `909e5042` (the legacy fixture regenerated).**
Two monotone u64 allocators that are secretly one, unified. Storage format changes;
scan order becomes fresh order; image append-base and verify_store counters re-derive.
- **Flush:** `docs/architecture/50-storage.md` (id law, stated once);
  `storage/commit/applier.rs` + `keys.rs`; `verify_store/counters.rs`; image build.

### R17 — The lock law is a writer law (150)
**Status: IMPLEMENTED — `ef5b9a42` (MDB_RDONLY lockless exhume), `a0344bfb` (the ffi doc trued), `d2d541d9` (the raw-open chokepoint).**
One-handle-per-path governs writers. Readers open `MDB_RDONLY`, lockless — archival
reads work on read-only media, snapshots, and mounted backups with no carve-outs.
Exhume becomes genuinely read-only.
- **Flush:** `50-storage.md` + `70-api.md` (lock law restated, narrower and truer);
  `storage/env/exhume.rs` + `acquire_lock.rs`.

### R18 — Ephemeral wipes and reinits after a machine crash (151)
**Status: IMPLEMENTED — `ef5b9a42` (dirty marker + wipe), `9836d1ee` (the meta taxonomy), `70fb5f5d` (the directory-mode lockfile named).**
The kind's contract: contents survive process restarts, not machine crashes. Reopening
after a crash yields a valid empty store, always; the corrupt state is unrepresentable.
The law rewords to "never destroys data it promised to keep."
- **Flush:** `50-storage.md` + `70-api.md` (ephemeral contract); `storage/env/ephemeral.rs`
  (dirty marker + wipe path); pairs with 149/152's meta-taxonomy fixes.

## E. Planner and measurement

### R19 — Estimates stay crude; adaptivity is the doctrine (089)
**Status: IMPLEMENTED (doctrine) — the flush states the law; `73215a30` prices the Allen JEPD keep inside the R19 scope; finding 089 superseded; the revisit trigger stands recorded here.**
The P3 "no histograms" ruling is re-affirmed on new grounds: the Free Join thesis places
precision at execution time. 009's GJ-shaped plans + dynamic cover choice bound skew at
runtime. Revisit only if post-009 benches show plan-choice misses covers can't absorb —
that trigger is recorded here.
- **Flush:** `docs/architecture/40-execution.md` (also corrects the overstated
  "WCOJ bounds the damage" claim — true only near the GJ end, which 009 now makes real).

### R20 — RNG fixed, corpora regenerate, numbers re-run (073)
**Status: IMPLEMENTED — `c815308a` (splitmix64), `5b2958e6` (the corpus regenerated), the campaign rerun (`1e9d39ad`/`8065d38c`/`c74242c5`) re-earned every published number.**
The seeded arm emits true 64-bit output. Every pinned corpus digest regenerates; all
published numbers re-run in the end-of-campaign bench night.
- **Flush:** `corpus_gen/rng.rs`; corpus fixtures; README graphs at campaign close.

### R21 — Docs re-pin against the post-campaign bench run (084)
**Status: IMPLEMENTED — `4de40efd` (every doc citation re-pinned to campaign-2026-07-23; retired paths named as git-history records; README graphs + headlines regenerated).**
Citations to deleted bench-out artifacts are not resurrected; normative docs re-pin
against the fresh run once the campaign lands.
- **Flush:** every `docs/architecture/*` measurement citation, at campaign close.

### R22 — Small measurement rulings
**Status: IMPLEMENTED — 071 `e5f35cb2`; 088 `0f13feff` + `2f326193`; 094 `8fddbae6`; 048 `be405715`; 159 `f644f150`; 080 `075a3b03`.**
- **071**: the two DurabilityLane enums fuse; writes-lane oracle envelope set by the
  fairness doctrine (coverage-checked mmap, per 074).
- **088**: pump tail-drain fixed regardless of ~1% price; the ≥4-atom bench shape it
  needs is added.
- **094 / 048**: measured-choice doctrine — microbench (chunk geometry at fanouts
  {2,4,8,64}) and profile pin (Allen const-operand phase fraction) run first; winners land.
- **159**: differential `Op` gains a `Program` arm; recursion enters the differential lattice.
- **080**: a linux SDK CI lane is added; the darwin-only rationale dies.

---

*Process note: rulings R1–R19 were made interactively by the owner on 2026-07-23;
the "resolved by policy" items follow the standing maximal-churn/maximal-elegance
policy recorded the same day. Findings 027 and 087 (Lean coverage of the union fold)
are promoted from optional to mandatory by R2.*
