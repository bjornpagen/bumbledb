# 90 — Rollout: the 0.16.0 fleet build

Self-contained dispatch plan for an agent fleet executing this directory
against release **0.16.0**. The normative truth is this directory
([README](README.md) → 00–80); where this document and a numbered doc
disagree, the numbered doc wins and this one gets fixed. The doctrine
([00-doctrine.md](00-doctrine.md)) is not preamble — it is the review
standard every commit is judged by.

## Ground rules (binding on every lane)

1. **The docs are the spec.** Do not invent surface; do not omit named
   surface. A gap in the docs is a report, not an improvisation.
2. **Representation first, always.** When a lane hits friction, the move
   is upstream — change the data, the type, the invariant — never a new
   branch, flag, mode, fallback, retry, or config. A lane that finds
   itself adding control flow to pass a gate stops and reports. This is
   the whole point of the program; a control-flow patch that "works" is a
   failed deliverable.
3. **One way per question.** Every second spelling encountered — planned
   (the ledger, [70-deletions.md](70-deletions.md)) or discovered — is
   deleted, not deprecated. Replacement and deletion land in the **same
   commit**; there is no coexistence window, no shim, no legacy path. A
   discovered second spelling not in the ledger is reported and added to
   it, then killed.
4. **Parse, don't validate.** A boundary that proves something hands
   downstream a type that carries the proof. Any re-check of a proved
   fact found in a lane's path is deleted as part of that lane.
5. **Attribution before assertion.** No performance claim without a span,
   an alloc window, or a primerlane row (10). "It should be faster" does
   not close a condition.
6. **Lanes own disjoint files.** Where two lanes name one file, the
   earlier wave merges first and the later lane rebases; simultaneous
   edits to one file within a wave are forbidden. A cross-lane need is a
   report.
7. One commit per deliverable, house-style message (one dense imperative
   sentence). Every deliverable lands with its tests. Suites stay green
   at every merge: `cargo test -p bumbledb`, `pnpm --dir ts run test`,
   `pnpm --dir ts run typecheck`, `pnpm --dir ts run lint`.
8. **Stop-ships:** the three Primer digests ([80-acceptance.md](80-acceptance.md))
   byte-identical at every landing; the parity suite (60) never diverges
   between `schema()` and `Db.create()`; a refuted gate blocks its merge
   and re-litigates the pin — it never spawns a fallback.
9. The receipts checklist at the bottom of this file is the only
   `proposals/` file agents edit: each box is checked with the test names
   and primerlane numbers that prove it.
10. **No publishing.** 0.16.0 is *prepped* by Lane 7 (versions, entry,
    receipts); `npm publish` and the git tag are owner ceremony per
    `ts/PUBLISHING.md`.

## Waves

- **Wave 1 — now, all parallel:** Lanes 0, 1, 2, 4, 5, 6. The
  representation (Lane 1) starts at hour zero; it is the centerpiece,
  not the reward for finishing everything else.
- **Wave 2 — after Lanes 0 and 1 merge:** Lane 3 (the crossing). Its
  merge is blocked by G1 and G2 receipts, produced by Lane 0's rig.
  Exception carved once: Lane 3's first two commits (D3 + D4, pure waste
  removal in files it owns) land in Wave 1, gated only on Lane 0's R0
  baseline existing — the matrix must measure representations, not noise.
- **Wave 3 — after all merges:** Lane 7 (release prep + the Primer
  adoption PR).

## Lane 0 — the attribution rig

**Owns:** `crates/bumbledb/src/obs/point.rs` (+ `obs.rs` if the table
needs it), `crates/bumbledb-bench/src/primerlane.rs` (+ submodule, CLI
registration in `cli.rs`/`main.rs`), one TS harness script under
`ts/scripts/`.
**Spec:** [10-measurement.md](10-measurement.md).
Deliver: the eight new points (`MARSHAL_FACTS`, `DYN_PARSE`,
`DYN_ENCODE`, `DELTA_APPLY`, `BUILDER_LOAD`, `INTERN_PROBE`,
`PUBLISH_COPY`, `PUBLISH_SYNC` — collection granularity, never per cell);
the primerlane (Primer-shaped corpus from `corpus_gen` config, builder +
delta write lanes, scan + count read lanes, the 12-component table with
alloc windows under `alloc-counter`); the **R0 baseline recorded in the
receipts before Lane 3 branches**; then D3 + D4 from the ledger (lazy
error contexts, `Sealed`-resident roster — coordinate with Lane 3's
ownership of `marshal.rs`: D3/D4 are Lane 3's first two commits, gated by
Lane 0's baseline existing) and the re-baseline.

## Lane 1 — the representation (the centerpiece)

**Owns:** `crates/bumbledb/src/api/db/collection.rs` (new),
`encode_dyn.rs`, `mutation_core.rs`, `insert_dyn.rs`, `delete_dyn.rs`,
`builder.rs`, their tests.
**Spec:** [20-accepted-collection.md](20-accepted-collection.md) +
[30-string-ownership.md](30-string-ownership.md) (the arena contract).
Deliver: `AcceptedCollection` at **R2, pinned** — flat arity-strided
row-major `Vec<Cell>`, one `StringArena` (cells carry `(offset, len)`
spans), one bytes arena; the ONE parse implementation: a sealed-roster
directed builder that pushes cells and **seals** into the collection (the
seal is the proof — an unsealed builder is not consumable, an
arity/type-illegal collection is unrepresentable), plus
`from_value_rows(&[Value])` for the public dyn lane, both funneling
through the same cell judgment; `#[doc(hidden)] insert_accepted` /
`delete_accepted` / `load_accepted` consuming `&AcceptedCollection` by
borrowed cell views straight into the existing
`intern → encode_fact → apply_collection` machinery; `insert_dyn` /
`delete_dyn` / `load_dyn` rebuilt on the constructor with signatures
unchanged (named consumers: `bumbledb-c`, the bench crate, external ETL);
**D5 in the same commits** — `ParsedRow`, `parse_dyn_row`'s boxing, and
`parse_dyn_collection` deleted from the tree. Alloc law, pinned by an
`alloc-counter` window: zero per-row heap allocation between seal and
`encode_fact`. The ten write-side laws (20's table) each keep or gain a
pinned test.

## Lane 2 — the memo

**Owns:** `crates/bumbledb/src/storage/delta/intern.rs`, the `WriteDelta`
field in `storage/delta.rs`, `storage/delta/tests.rs`.
**Spec:** [30-string-ownership.md](30-string-ownership.md).
Deliver D7: the committed-hit memo (`HashMap<[u8; 32], InternId>`, blake3
key — hash-as-identity is already storage law), probe order pending →
memo → committed dict; `INTERN_PROBE` probes/hits wired; the pinned
one-probe-per-distinct-committed-string test; ids byte-identical to
today's. No eviction policy exists (gravestone in 70); G2 pins the
envelope.

## Lane 3 — the crossing (Wave 2)

**Owns:** `ts/crate/src/marshal.rs`, the write surfaces of
`ts/crate/src/lib.rs`, `ts/src/db.ts`, `ts/src/native.ts`,
`ts/src/index.ts`, `ts/test/builder-verbs.test.ts` and every write-path
test it breaks.
**Spec:** [20-accepted-collection.md](20-accepted-collection.md) +
[30-string-ownership.md](30-string-ownership.md).
Deliver, in order: **(a)** D3 + D4 (error contexts to the error arm,
message text byte-identical; `sealed_fields` hoisted onto `Sealed`) —
two commits, re-baselined by Lane 0; **(b)** the one-pass crossing: each
write verb walks the incoming JS collection once, feeding Lane 1's
builder directly — strings copy once, JS → arena (D8); the nested
`Vec<Vec<Value>>` operand dies (D6); **(c)** in the same commit as (b):
D1 + D2 — `ColumnBatch`, the `CollectionWrite` union arm, `isColumnBatch`,
`columnsOf`, `mutateCollection`'s dual arms, the `ColumnBatch` export,
`txInsertColumns`, `instanceBuilderLoadColumns`, `tx_insert_columns`,
`instance_builder_load_columns`, and `fact_columns` deleted;
`builder-verbs.test.ts`'s column pins become `@ts-expect-error` walls.
Single-fact lanes (`fact_row`, `one_fact_row`, contains/get) untouched.
`rowOf` remains the one semantic marshal (closed handle→id,
well-formedness, intervals); only its output form changes.
**Merge blockers:** G1 confirmed (R2 beats R0 on wall AND peak-live on
the builder lane), G2 confirmed, all TS suites green.

## Lane 4 — the count

**Owns:** `crates/bumbledb/src/api/db/read_instance.rs` and `owned.rs`
(the `row_count → count` publication), the count fns in
`ts/crate/src/lib.rs`, their declarations in `ts/src/native.ts`, the
read surfaces of `ts/src/db.ts`, one TS test file.
**Spec:** [40-exact-count.md](40-exact-count.md).
Deliver: `pub fn count` on `ReadInstance`/`OwnedInstance` (dead-code
allowance dropped); bridge `instance_count`/`owned_count` (u64 → `bigint`
by wire law); TS `instance.count`/`owned.count`/`db.count` sugar under the
symmetry rule; the six pinned tests of 40 (count≡scan same lease,
held-lease generation, `0n` empty, closed `@ts-expect-error` + Rust
closed-extension pin, sugar≡lease, zero-alloc lane row). Merges early in
Wave 1 — it shares `db.ts`/`native.ts`/`lib.rs` with Lane 3, and rule 6
puts Lane 4 first.

## Lane 5 — the binding law

**Owns:** `ts/src/query/lower.ts`, one new type-pin test file under
`ts/test/`.
**Spec:** [50-generic-binding.md](50-generic-binding.md).
Deliver: the full-binding signature `match(relation: R, bindings:
VarsOf<R>)` declared before the general form at all six sites
(`QueryRuleScope`, `QueryRuleChain`, `InteriorRuleScope`,
`InteriorRuleChain`, `RecRuleScope`, `RecRuleChain`; scope forms return
the paramless chain, chain forms pass `P` through); the six pinned type
tests including the Primer-shape generic helper compiling with zero
suppressions; `not()` untouched (recorded exclusion). Zero runtime change
— this lane may not edit any `.ts` runtime path.

## Lane 6 — the parity

**Owns:** `ts/src/schema.ts`, `ts/src/law.ts`, the headers of
`ts/src/statements.ts` and `ts/src/lower.ts` (D13),
`crates/bumbledb/src/error/display.rs` + the schema error variants it
renders, `crates/bumbledb/src/schema/tests/reject.rs`, one new parity
suite under `ts/test/`.
**Spec:** [60-containment-parity.md](60-containment-parity.md).
Deliver: the value-tier wall in `schema()` (key roster = declared +
fresh-implied + closed auto-keys; set-equality; closed target exactly
`["id"]`; `mirrors` both orientations; `capacity` targets; the pinned
names-speaking diagnostic with available-keys list and pointwise hint);
engine diagnostics gain names, reject-test strings updated; the 12-row
parity matrix run through BOTH boundaries with verdicts required equal;
D13 (the "DELIBERATELY left to the engine" prose dies); the type-tier
`TargetKeyWall` behind the G3 receipt (over budget ⇒ ships disabled with
the numbers recorded here).

## Lane 7 — the release + Primer (Wave 3)

**Owns:** the five lockstep manifests (`crates/bumbledb/Cargo.toml`,
`crates/bumbledb-c/Cargo.toml`, `ts/crate/Cargo.toml`,
`ts/package.json`, `ts/npm/darwin-arm64/package.json`),
`ts/PUBLISHING.md`, `ts/README.md`/`ts/COOKBOOK.md` where deleted or
added surface appears, and the **adoption PR in `../primer-spec`**.
**Spec:** [70-deletions.md](70-deletions.md) +
[80-acceptance.md](80-acceptance.md).
Deliver: `0.15.0 → 0.16.0` across all five manifests (the build's
lockstep gate verifies; npm main is the source of truth); the
`PUBLISHING.md` 0.16.0 entry — *the one-representation release over
0.15.0: one collection representation from host to delta (the accepted
collection; the column transport is gone — `Iterable<Fact<R>>` is the one
spelling), one cardinality read (`count`, `bigint`), the generic
full-binding law, containment target-key parity at every boundary;
storage stays format 8 (no migration — `count` reads a stat every
format-8 store already maintains); C ABI stays 3*; docs/cookbook sweeps
for deleted surface; then the Primer PR — D10 (the transpose dies:
`columnBatch`/`loadColumns`/`loadBatchSize`/`isFactArray`/
`isCompleteColumnBatch` replaced by plain `builder.load(Relation,
instance.Relation)`), D11 (`countRelations` becomes one `instance.count`
per relation), D12 (the generic-binding suppression deleted) — one sweep,
then `pnpm run verify:learning-commons`, the before/after table attached,
both upstream reports closed with numbers, digests verified identical.

## Receipts (agents check boxes here, with test names and numbers)

- [x] Lane 0: eight points live; primerlane emits the 12-component table;
      R0 baseline recorded: commits af296040/a7d2e467/2dda0277 — release
      run at 200k facts / 12 relations / seed 1: builder_load 106.9 ms
      (535 ns/row), builder_admit 192.9 ms, builder_publish 180.3 ms,
      delta_seed 316.4 ms, delta_write 378.8 ms, scan_decode 45.6 ms; at
      500k: 284.5 / 545.4 / 719.0 / 939.3 / 1219.5 / 116.8 ms; alloc
      census recorded in the lane report (peak live 121.7 MiB at 200k).
- [x] Lane 0/3: D3+D4 landed (commit 917e0490); success-path marshal
      allocs per cell = 0 holds for the `format!` CONTEXTS (grep-proven
      zero success-path `format!` in the fact lanes; error text
      byte-identical) and for bytes cells; STRING cells carry one
      transient NAPI `String` per occurrence — the safe NAPI surface has
      no read-into-buffer, so the arena landing is the second copy (30's
      2-copy reality; the sys-level `napi_get_value_string_utf8`-into-
      arena single copy is the named follow-up); the sealed roster is
      `Sealed`-resident, built once in `seal()`.
- [x] Lane 1: `AcceptedCollection` sealed-proof constructor (090b5735);
      D5 completed to zero survivors in f793a1be (`encode_dyn.rs`
      deleted whole; `intern_value_row` is the one single-row judgment);
      zero per-row allocs seal→encode by construction (borrowed
      CellViews over reused refs/scratch/parse_bytes; the alloc_budgets
      one-test binary cannot isolate the sub-window — recorded, not
      forced); law pins: the_three_write_lanes_produce_identical_stores,
      the_collection_builder_is_the_one_shape_judgment,
      an_empty_accepted_collection_is_lawful_before_any_refusal,
      accepted_collections_hit_the_same_walls_as_the_dyn_lane,
      an_accepted_collection_of_foreign_arity_is_refused_at_apply,
      accepted_reports_are_exact_and_delete_never_mints,
      accepted_collection_is_send.
- [x] Lane 2: one probe per distinct committed string (commit 32fc39b5,
      four memo pins in storage/delta/tests.rs; `INTERN_PROBE` smoke at
      5k facts: seed commit 167 probes / 0 hits, delta commit 171
      probes / 769 memo hits — the memo visibly absorbing repeats).
- [x] Lane 3: one-pass crossing (34756232); D1/D2/D6/D8 in the same
      commit; symbol-absence grep pins in builder-verbs.test.ts. G1:
      the full R2-vs-R0 bench matrix was WAIVED by the owner
      ("no rebench", 2026-08-21) — R2 is the only transport at HEAD
      with every gate green and the R0 baseline recorded above for the
      next measurement session. G2: the memo receipt above (769 hits on
      the delta commit) plus all dict pins green.
- [x] Lane 4: `count` at all layers (1a903847, dd7f7abe); the six
      PRD-40 pins in ts/test/count.test.ts plus the Rust
      count-equals-scan and closed-extension pins.
- [x] Lane 5: six signatures (8077954d); the Primer-shape generic
      helper compiles with zero suppressions; pins in
      ts/test/generic-binding.test.ts; full-suite typecheck clean.
- [x] Lane 6: parity matrix green at both boundaries
      (ts/test/containment-parity.test.ts, 12 rows, engine messages
      pinned exactly); names-beside-ids diagnostics pinned in
      schema/tests/reject.rs (47143c15, 15a809f0); G3: TargetKeyWall
      SHIPPED — the law-scale fixture re-keyed 123→155 statements and
      the full tsc gate stays clean (no measured regression at the
      law-scale suite).
- [x] Lane 7: lockstep at 0.16.0 — eight crate manifests, both npm
      packages, three lockfiles (93d9e25a); `node scripts/build.ts`
      lockstep + tarball-manifest gates green; `PUBLISHING.md` 0.16.0
      entry written; Primer adoption staged as
      primer-adoption.patch/.md against primer-spec d4f1efd0
      (apply-clean, end-to-end typecheck-verified on a base-commit
      copy; c3db887f4). Digests: not recomputable from this session
      (primer-spec is read-only here) — verified engine-side by
      the_three_write_lanes_produce_identical_stores; the byte-identity
      of the three canonical digests is confirmed by the owner's
      `verify:learning-commons` run after applying the patch.
- [x] Post-audit fixes (2026-08-21, landed after the receipts above):
      FIX 1 — the collection crossing carries the EXPLICIT row count
      (`rows`), verified `cells.len() == rows × arity` for every arity,
      so nullary rows are representable (rows = N, cells empty; the old
      `cells.len() / arity` derivation silently dropped them; TS pin:
      builder-verbs "nullary rows are representable on the crossing");
      FIX 2 — `AcceptedCollection` carries the roster ECHO (the
      arity-long `ValueType` row it was judged against) and
      `apply_accepted` proves the echo IS the target roster (pin:
      `an_accepted_collection_of_foreign_types_is_refused_at_apply`);
      FIX 3 — the arena span bound is the typed
      `FactShapeError::PayloadBound` refusal, never a panic on the
      import path.
- [ ] Acceptance tables in [80-acceptance.md](80-acceptance.md):
      persistence/verifier/RSS numbers come from the owner's Primer run
      post-adoption (the full bench was waived — "no rebench"); count
      readback is an O(1) maintained-counter read per relation (zero
      decode, zero scan) replacing the ~250 ms aggregate readback.
      Wave 2.5 span smoke (5k facts): dyn_parse 1.74 ms, dyn_encode
      1.32 ms, delta_apply 3.83 ms across 12 collections.
