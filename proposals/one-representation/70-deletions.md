# 70 — The deletion ledger

Every second spelling and every hot-path waste this set removes, each with
its one replacement and the pin that keeps it dead. A deletion is part of
the product: the ledger is normative, and an entry may not be "kept for
compatibility" — where a deletion breaks a caller, the break ships in the
same release as its replacement and the caller is named here.

Baseline note: the primerlane ([10-measurement.md](10-measurement.md))
records R0 numbers *before* any entry lands; every entry's effect is
attributed there, not asserted.

## The ledger

| # | Deleted | Where | Violation | The one replacement | Pin |
| --- | --- | --- | --- | --- | --- |
| D1 | The public column transport: `ColumnBatch<R>`, the `CollectionWrite` union arm, `isColumnBatch`, `columnsOf`, `mutateCollection`'s dual arms, the `ColumnBatch` export, and the "second transport" doc voice | `ts/src/db.ts:84-138`, `ts/src/index.ts` | V2 | `Iterable<Fact<R>>` — the one collection spelling, made the fastest by 20 | `builder-verbs.test.ts` column pins replaced by `@ts-expect-error` walls; API-surface test that no `ColumnBatch` export exists |
| D2 | The paired native crossings: `txInsertColumns`, `instanceBuilderLoadColumns` (`native.ts`), `tx_insert_columns`, `instance_builder_load_columns`, `fact_columns` (`ts/crate`) | `ts/src/native.ts`, `ts/crate/src/lib.rs`, `ts/crate/src/marshal.rs` | V2 | one crossing per verb, carrying the accepted collection (20) | symbols absent from the tree; bridge test enumerating the native surface |
| D3 | Success-path error contexts: the per-cell `format!` in `schema_value` (marshal.rs:199), per-cell in `fact_columns` (dies with D2), per-row in `one_fact_row`, fact-lane `req_at` contexts | `ts/crate/src/marshal.rs` | V3 | error text built on the error arm only — byte-identical messages | AS ENFORCED: grep pin (zero success-path `format!` in the fact lanes — 90's Lane 0/3 receipt); error text byte-identical. The alloc-counter window was NOT implementable in the one-test alloc harness (`alloc_gate` holds one test by invariant) — the grep pin is the standing enforcement |
| D4 | Per-call sealed-roster re-derivation (`sealed_fields`'s fresh `Vec<(Box<str>, ValueType)>`) | `ts/crate/src/marshal.rs::sealed_fields` | V4 | the roster resident on `Sealed`, borrowed per call | AS ENFORCED: construction argument — the roster is `Sealed`-resident, built once in `seal()`; the symbol `sealed_fields` is gone (grep). The per-call alloc window was not implementable in the one-test alloc harness |
| D5 | The engine's second parse: `ParsedRow`, its `Box<[ParsedCell]>` per row, `parse_dyn_collection` | `crates/bumbledb/src/api/db/encode_dyn.rs`, `mutation_core.rs` | V1 | the accepted collection's borrowed cell views; ONE parse implementation, in its one constructor (20) | AS ENFORCED: symbols absent (grep; `encode_dyn.rs` deleted whole); zero per-row allocation seal→encode by CONSTRUCTION (borrowed `CellView`s over reused `refs`/`scratch`); the differential store pin (`the_three_write_lanes_produce_identical_stores`). The seal→encode alloc window was not implementable in the one-test alloc harness |
| D6 | The bridge's nested operand: per-row `Vec<Value>` inside `Vec<Vec<Value>>` handed to `insert_dyn` | `ts/crate/src/marshal.rs::fact_rows` → `ts/crate/src/lib.rs::tx_insert` | V1 | the accepted collection crossing (20); public Rust `insert_dyn(&[Value]-rows)` remains for its named consumers (`bumbledb-c`, bench, ETL) and funnels into the same constructor | primerlane component 2/3 windows |
| D7 | Per-occurrence committed-string probes (pending-miss → blake3 + LMDB get, every occurrence) | `crates/bumbledb/src/storage/delta/intern.rs` | V5 | the committed-hit memo: one probe per distinct string per transaction (30) | `INTERN_PROBE` probes == distinct strings on the delta lane; engine test in 30 |
| D8 | The string triple-copy on the collection lane (NAPI `String` → `Box<str>` → `Value::String`) | `ts/crate/src/marshal.rs::schema_value` | V5 | JS → the collection's string arena; the single pending-flush copy in `PendingInterns` is *kept*, its consumer named (the phase-4 dictionary flush) | AS BUILT: copies per occurrence = 2, not 1 — the safe NAPI surface has no read-into-buffer, so one transient NAPI `String` precedes the one arena copy (`push_str`); the sys-level `napi_get_value_string_utf8`-into-arena single copy is 30's NAMED follow-up, never silently claimed |
| D9 | Counting by scanning or by aggregate: `scan(r).length` as cardinality; full-binding `r.count()` queries as cardinality; the caller-side empty-set→`0` branch both force | hosts; bench read lanes | V6 | `count` — the one cardinality spelling (40); `r.count()` remains what it is, the size of a query's answer set | 40's pinned tests; bench count lane replaces any scan-as-count lane. The bench crate's three surviving scan-as-count sites (`corpus.rs:204,221`, `lanes/storage.rs:316`) are SANCTIONED: they are the independent verification of the maintained counter against the mirror/expected rows, not counting idioms — a `count`-reads-`count` comparison would verify nothing |
| D10 | Primer's packing shim: `columnBatch`, `loadColumns`, `loadBatchSize = 16_384`, `isFactArray`, `isCompleteColumnBatch` | `primer-spec/src/storage/bumbledb/runtime.ts:39-142` | V10 | `builder.load(Relation, instance.Relation)` — the plain call, no intermediate structure | Primer's runtime contains no transpose; upstream acceptance 3 ("Primer needs no Bumbledb-specific packing code") |
| D11 | Primer's counting idioms: `countRelations`' `instance.scan(relation).length` and the temporary full-binding count queries | `primer-spec/src/storage/bumbledb/runtime.ts::countRelations` | V6, V10 | one `instance.count(relation)` per relation in one lease | Primer readback compares counts via `count`; the ~250 ms query readback and the 4 M-object decode are both gone from the verifier profile |
| D12 | Primer's localized type suppression around generic `v(rel)`/`match` | Primer's generic query helper | V7 | the full-binding signature (50) | Primer compiles with zero bumbledb-related suppressions |
| D13 | The stale deferral prose: "DELIBERATELY left to the engine…" | `ts/src/schema.ts:1-10`, `ts/src/statements.ts:27-34`, one header line in `ts/src/lower.ts` | V8 | headers stating the two-tier wall and the engine's final authority (60) | doc-comment review pin in 60's change; grep gate: the phrase "deliberately left to the engine" names no unenforced law |

## Gravestones (considered and refused — do not re-litigate without new evidence)

- **Keeping `ColumnBatch` "for flexibility".** A transport kept alongside
  its replacement is a mode; callers would branch on it and the two paths
  would drift. If a measured host someday genuinely *holds* columnar data,
  that is a new caller-visible distinction to litigate then — with its
  lane in 10 — not a reason to keep this one now.
- **A structural fix for V7** (making the general `match` signature
  generically self-evident). Refused by probe B in
  [50-generic-binding.md](50-generic-binding.md): three independent
  type-level deferrals stand in the way, and loosening them weakens
  concrete checking.
- **`WriteTx.count`.** No consumer names itself; derivable from
  `row_count_delta` the day one does ([40-exact-count.md](40-exact-count.md)).
- **A `snapshot` alias for the read lease.** A second name for one thing.
- **LRU eviction on the committed-string memo.** An eviction policy is a
  mode; the bound, if ever needed, is a fixed capacity pinned by G2's
  numbers ([30-string-ownership.md](30-string-ownership.md)).
- **Synthesizing the missing target key, or demanding a global FD.**
  Refused in [60-containment-parity.md](60-containment-parity.md), in the
  upstream report's own words.
- **General inclusion dependencies into non-key projections.** Essential
  complexity of a different feature; refused with the Lean price theorem
  cited (60).
- **`MDB_APPEND` on the commit put stream.** Already a recorded gravestone
  at the put site (`storage/commit/applier.rs`) — restated here because a
  transport proposal predictably resurrects it: the `M` key embeds the
  fact hash, so the stream is never key-ordered, structurally.

## Coupling (what lands with what)

D3, D4 land first (before the G1 matrix — they are noise, not
representation). D1, D2, D5, D6 land with 20. D7, D8 with 30. D9 with 40.
D13 with 60. D10, D11, D12 land in Primer's adoption change against the
release carrying 20/40/50 — one Primer PR, one ledger sweep.
