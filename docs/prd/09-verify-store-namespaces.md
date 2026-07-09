# PRD 09 — `verify_store`: namespace coherence

**Depends on:** nothing (10 builds on this).
**Modules:** new `crates/bumbledb/src/verify_store.rs` (+ submodule dir), reading
through `crates/bumbledb/src/storage/{keys.rs,read/,env/}` and
`crates/bumbledb/src/schema.rs`.
**Authority:** `50-storage.md` (namespace layout, the R-delete asymmetry
paragraph — which currently cites an offline sweeper that does not exist),
`30-dependencies.md` (guard derivation, statement resolution).

## Context (decided)

The commit path self-checks F/M/U on delete (`MembershipDesync` hard errors) but
deletes `R` edges **without** verifying they existed
(`storage/commit/applier.rs:64-68`) — deferred to "the offline sweeper" — while
target-side judgment **trusts** `R` prefixes as the survivor authority
(`storage/commit/judgment.rs:300-317`). The one unverified namespace is one that
commit verdicts lean on. **Placement decided: engine-side** — the sweeper's
knowledge (key layouts, guard slicing, statement resolution) is engine knowledge;
a bench-side copy would drift. This PRD builds the namespace sweeps; PRD 10 adds
the global judgment re-verification and the CLI wrapper.

## Technical direction

`Db::verify_store(&self) -> Result<StoreReport, Error>` — read-only, one LMDB
read snapshot, O(store). At the ≤10⁷-fact axiom this is seconds; no incremental
mode, no parallelism.

1. **Report machinery first.** `StoreReport { findings: Vec<StoreFinding> }`;
   `StoreFinding` is a typed enum — one variant per desync class below, payloads
   in the `CorruptionError` discipline (namespace tag, relation/statement ids,
   offending key bytes as `Box<[u8]>`; never formatted strings). A clean store
   returns an empty report; the *caller* decides whether findings are fatal
   (`verify_store` itself never errors on findings — `Err` is reserved for I/O).
2. **F↔M (bidirectional):** scan `F` per relation: each fact's blake3 must have
   an `M` entry whose row id points back; scan `M`: each entry's row id must
   resolve to an `F` fact whose hash matches. Reuse `fact_hash` from encoding —
   never reimplement.
3. **F↔U:** for every key (Functionality) statement: each fact's guard bytes
   (the shared `keys::guard_bytes` slicer — never duplicated) must exist in `U`
   under that statement with the fact's row id; each `U` entry under the
   statement must resolve to a live fact that re-derives the same guard.
   **Pointwise keys additionally re-verify per-group disjointness** by an
   ordered walk over the guard prefix groups (the invariant the neighbor probe
   assumes but never re-checks globally): within each scalar-prefix group,
   successive intervals must satisfy `prev.end <= next.start`.
4. **F↔R:** for every containment statement: each source fact satisfying φ
   (reuse the commit path's selection-satisfaction helper) must have its `R`
   edge under the statement with the permuted key bytes; each `R` edge must
   resolve to a live source fact that still satisfies φ and re-derives the key
   bytes. This is the namespace with no online verification — the heart.
5. **Counters:** per relation, `S` row count equals the `F`-prefix cardinality
   counted during the F scan (no second scan); row-id high-water ≥ max observed
   row id; dict next-id > every referenced intern id. Dangling dict entries
   (ids referenced by no fact) are the *accepted leak* — count them into the
   report as an informational statistic, never a finding.
6. Structure the sweeps so each namespace pass is one function over one cursor
   range — the module should read like the `50-storage.md` key-layout table.

## Passing criteria

- `[shape]` `verify_store` lives in the engine crate; the bench crate contains
  no namespace-layout knowledge beyond calling it; guard/key derivation is
  imported from `keys.rs` (grep: no second slicer definition).
- `[test]` Fixture stores with each desync class hand-injected (raw LMDB writes
  through a test-only env handle: missing `M`, orphan `M`, missing `U`, orphan
  `U`, pointwise-overlap pair, missing `R`, orphan `R`, wrong `S`, low
  high-water) each produce exactly their finding variant; a clean store
  produces an empty report with the dict statistic populated.
- `[test]` The finding payloads carry the expected ids/bytes (assert on one
  case per class).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

Deferred to PRD 10 (one amendment covering the whole tool once the CLI exists).
