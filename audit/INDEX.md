# Audit index — final pass (store-and-value + hardcore laws)

Lens: [REQUIRED-READING.md](REQUIRED-READING.md). Owner-approved plan,
filed 2026-08-19. The interior pass (01–19) is closed; its record is git
history. Standing rulings: [kept.md](kept.md). Downstream gaps:
[primer-integration.md](primer-integration.md). Convention:
[README.md](README.md).

Three laws govern this pass:
1. **The purge** — reads and writes on the store, plus the proven value;
   nothing else public.
2. **Zero-dyn engine** — `dyn` lives only in the SDK bridges; the engine's
   one exemption is `std::error::Error::source`.
3. **Allocation discipline** — zero allocations on steady-state hot paths,
   pinned as budget tests, plus the recorded perf-debt ledger closed with
   traces.

## Roster

| # | File | Lane | One line |
| --- | --- | --- | --- |
| 20 | [purge ephemeral](20-purge-ephemeral.md) | A | **fixed this pass** — store kind dies; hidden NOSYNC open remains |
| 21 | [purge exhume](21-purge-exhume.md) | A | **fixed this pass** — exhume/Exhumed die; encode/fingerprint stays |
| 22 | [purge Instance trait](22-purge-instance-trait.md) | A | **fixed this pass** — concrete types only; un-promote `row_count`/`profile` |
| 23 | [`_meta` four keys](23-meta-four-keys.md) | A | **fixed this pass** — four-key roster; `EnvMode` is a `File` |
| 24 | [dyn intern resolver](24-dyn-intern-resolver.md) | C | **fixed this pass** — `F: FnMut`; alias deleted |
| 25 | [dyn hatch unit](25-dyn-hatch-unit.md) | A | **fixed this pass** — concrete unit decline; family still Io |
| 26 | [dyn render names](26-dyn-render-names.md) | B | **fixed this pass** — `N: Names + ?Sized`; goldens identical |
| 27 | [dyn census gate](27-dyn-census-gate.md) | integration | **fixed this pass** — `zero_dyn_engine_pins_error_source_exemption`; spec-census (g) |
| 28 | [alloc law + budgets](28-alloc-law-budgets.md) | C | **fixed this pass** — `alloc_law_budgets`; K=3 |
| 29 | [alloc hot-path hunt](29-alloc-hot-path-hunt.md) | C | **fixed this pass** — classify/fix/rule; Checker key copies gone |
| 30 | [IntervalTail merge](30-interval-tail-merge.md) | B | **fixed this pass** — `ValueType` width owner; `IntervalTail` gone; `golden_fingerprint_pins_the_hash` |
| 31 | [NodeScratch copies](31-node-scratch-copies.md) | C | **fixed this pass** — residual specs; no predicate clones |
| 32 | [delete lane 3.1–3.4×](32-perf-delete-lane.md) | D | trace the untraced twin, then fix |
| 33 | [commit ladder](33-perf-commit-ladder.md) | D | re-baseline on the NOSYNC flag, then attribute |
| 34 | [r6 descend 1.46](34-perf-r6-descend.md) | D | fresh flame first — the tree moved |
| 35 | [overlap re-pin](35-overlap-repin.md) | owner | quiet-machine sweep; never by inspection |
| 36 | [leaf batch-of-1 (014)](36-perf-leaf-batch.md) | D | re-bench o4 first |
| 37 | [telescoped Count (044)](37-perf-telescoped-count.md) | D | sequenced after 34's trace |
| 38 | [min/max fence (009)](38-perf-minmax-fence.md) | D | confirm premise under `GroupState` |
| 39 | [owned-instance lanes](39-owned-instance-lanes.md) | D | the heap arm gets its ladder + the Primer gate |
| 40 | [docs store-and-value](40-docs-store-and-value.md) | E | proposal, arch docs, census tokens; Lean untouched |

## Order

Phase 1, parallel lanes (file-disjoint): **A** = 22 → 20 → 21 → 23 → 25
(owns `error.rs`, `env/`, bridges' deleted surfaces) · **B** = 26 → 30
(schema/encoding) · **C** = 24 → 28 → 29 → 31 (judgment/exec/alloc).
Integration 1: merge, land 27, full suites. Phase 2: **D** = 32 → 33 → 34 →
37 → 36 → 38 → 39 (bench crate; needs stable code) with **E** = 40 in
parallel; 35 runs on the owner's quiet machine. Final integration:
suites + census + checkpoint.
