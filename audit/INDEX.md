# Audit index — 0.15 interior, live-tree pass

Lens: [REQUIRED-READING.md](REQUIRED-READING.md). Format **8**, ABI **3**,
crate **0.15.0** shipped on `main` (`d2ed04ab`); everything here is the
working-tree interior. One numbered file per open issue (convention in
[README.md](README.md)); standing do-not-fix rulings in [kept.md](kept.md);
downstream gaps in [primer-integration.md](primer-integration.md). The
earlier 0.13 and 0.15 second-pass area files are deleted — every still-open
row was carried into a numbered file, and the fixed rows' record is git
history.

Statuses verified 2026-08-19 after the audit-lane fanout landed on
`main`. Trust the `Status` line inside each file over this table when
they disagree.

## Roster

| # | File | Status |
| --- | --- | --- |
| 01 | [napi error carrier](01-napi-error-carrier.md) | **fixed this pass** (`create_error` + `kind`; suite green pending re-run) |
| 02 | [one temporal shape](02-ts-temporal-shape.md) | **fixed this pass** — AsyncTask control plane; lease flag on publish |
| 03 | [owned single read](03-ts-owned-single-read.md) | **fixed this pass** — five direct owned reads; lease spelling gone |
| 04 | [TS builder verbs](04-ts-builder-verbs.md) | **fixed this pass** — full builder verbs; column load/insert |
| 05 | [WriteDelta lifetime](05-writedelta-lifetime.md) | keep **accepted** (cost ruling; reopening horizon recorded) |
| 06 | [instance one body](06-instance-one-body.md) | keep (scan) / **fixed this pass** (dict + ScratchPool seed/park) |
| 07 | [view binding](07-view-binding.md) | **fixed this pass** — per-occurrence `Binding` / `OccMemo` |
| 08 | [relation slot](08-relation-slot.md) | **fixed this pass** — one `RelationSlot` table; SPINE-16 epochs from `ImageBind` |
| 09 | [profile + stats](09-profile-stats.md) | **fixed this pass** — `Instance::profile` + one `hit` |
| 10 | [codec value vocabulary](10-codec-value-vocabulary.md) | **fixed this pass** (typed decode; no `ValueRef` `Fixed*` arms) |
| 11 | [C ref slots](11-c-ref-slots.md) | **fixed this pass** (retired slots leak on destroy; `MISUSE` test pinned) |
| 12 | [C owner tokens](12-c-owner-tokens.md) | **fixed this pass** (`OwnerToken`; bridge pre-refusal test pinned) |
| 13 | [C exit threading](13-c-exit-threading.md) | keep (one `Result<()>` channel) / **fixed this pass** (unforgeable hatch) |
| 14 | [V8 lazy accounting](14-v8-lazy-accounting.md) | **fixed this pass** (`OwnedSlot.accounted` cell; ops sync `retained_bytes`) |
| 15 | [exec one evaluator](15-exec-one-evaluator.md) | **fixed this pass** (one `holds` entry; two-module gate pinned) |
| 16 | [verify-store embedding](16-verify-store-embedding.md) | **fixed this pass** (`StoreFinding::{Judgment, Corruption}`; `InternId`) |
| 17 | [docs vocabulary](17-docs-vocabulary.md) | **fixed this pass** — fence table, snapshot prose, case count, Lean prose, census token |
| 18 | [ramdisk probe](18-ramdisk-probe.md) | **fixed this pass** (`RamDiskProbe` sum; live lock `#[ignore]`) |
| 19 | [violations attach seam](19-violations-attach-seam.md) | **fixed this pass** (`from_pairs`; decoration builds the stored pairs) |

## Owner rulings required

1. **07 view binding** — owner ruling landed: the filed `Binding` sum
   proceeds. Status in [07](07-view-binding.md).
2. **08 relation slot** — **ruled: filed fix proceeds** (keep overturned).
   Landed this pass: one `Box<[RelationSlot]>`; `Closed` carries no
   generation; `ImageBind` mints `ViewEpoch`.
3. **09 profile promotion** — owner ruling landed: `profile` is counting
   instrumentation, not a drift clock. Status in [09](09-profile-stats.md).
4. **11 already resolved by fix** — recorded here because it closed a gate
   contradiction: destroy now leaks retired slots (the C parse of "the slot
   outlives the handle"), and the UAF gate is met by test.

## Fixed this pass (receipts; record in git history and file Status lines)

1. `ReadInstance { core: InstanceCore<LmdbSource<'txn>, S> }`; `LmdbImages`
   deleted; store prepare/execute through generic `prepare_on`/`bind`.
2. `WriteTx { mutation: MutationCore<StoreMutation> }`; one
   `MutationPhase`; empty apply short-circuits before poison.
3. `Violations` is `Box<[(Violation, Box<[CitedFact]>)]>` with private
   fields and a `compile_fail` pin; `Violation` carries `StatementRef`
   only (`StatementId` derived via `Schema::id_of`). Decoration is
   `from_pairs` — no parallel `Vec` + length assert.
4. N-API throws a real `Error` carrying `kind`; forced `ErrorFamily` table;
   TS open/prepare refusals wrap exported `Err*` values.
5. V8 external memory on admit/close **and** lazy image birth.
6. C: one `phase: AtomicU32` handle word; retired-slot leak on destroy
   (`MISUSE`, never UAF — test); `OwnerToken` on prepared/instance-ref with
   bridge pre-refusal (test); tagged admissions with the `moved` arm.
7. `ExhumeOutcome` three variants; fresh-range tag; docs `TypeDesc`→
   `ValueType`, introspection v7, ABI-3 rows.
8. One `Box<[RelationSlot]>` (`Closed` | `Frozen` | `Ordinary`);
   `ImageBind` mints `ViewEpoch`; no `txn.generation()` under `image/`.
9. TS control plane is `AsyncTask` (`create`/`open`/`fromInstance`/`exhume`);
   owned reads are five direct natives; builder has the engine verb set
   (`temporal-shape`, `owned-read`, `builder-verbs` tests).
10. One `holds` entry; `exhaustive_filter_predicate_matches_live_in_two_modules`.
11. Per-occurrence `Binding` / `OccMemo` (`residual_bindings_memoize_under_lru`
    and the four sibling memo tests).
12. `Instance::profile` + `KeyProbeStats::from_emitted`
    (`profile_on_owned_and_lease_agrees`).
13. Typed decode; no `unreachable!("schema-typed")`; no `ValueRef` `Fixed*`
    arms (`typed_decode_reads_the_layout_arm`).
14. Store `CodecRead` through catalog dict; `assemble` takes a `ScratchPool`;
    `Db::read` seeds/parks it (`the_reader_cache_is_invisible_except_in_speed`).
15. `StoreFinding::{Judgment, Corruption}`; dangling ids are `InternId`.
16. Docs/Lean snapshot-word sweep; census token; ramdisk `RamDiskProbe` sum
    with live lock `#[ignore]` (`timed_families_refuse_a_live_ram_disk`).
17. Unforgeable C decline hatch (`hatch_reuses_io_family_and_downcasts`,
    `abort_plus_engine_failure_reports_engine_failure`). No new
    `ErrorFamily` arm. Pointer: [kept.md](kept.md) (C abort still rides
    one `Result<()>`).

## Already right (do not "fix" back)

- `Admission<T>`, `Check::{Holds, Violated}`, `Committed<R>`,
  `ConditionalWrite::Moved`.
- No public `Snapshot`, `CommitSeq`, `CommitRejected`, `GenerationMoved`,
  `NotInitialized`, `ForeignSnapshot`, `Admitting`, `Durable`-as-`Db`.
- Format 8 / ABI 3 / 0.15.0 lockstep; format-7 refused everywhere.
- `InstanceBuilder = MutationCore<HeapMutation>` with no query methods.
- `ViewEpoch::{Closed, Frozen, Store(GenerationId)}`; no dummy generation.
- `CatalogRead`/`CatalogWrite`, `FrozenCatalog` without `_meta`,
  `CatalogIdentity`, `CodecRead`/`CodecWrite`,
  `Probe::{Encoded, ProvablyAbsent}`, `InternId`.
- `Staleness::{NoStatistics, Measured}`, `RuleStats` enum, sealed
  `MutationReport`.
- Lean L1–L5 and the three-oracle complete-admission conformance lane.
- `scripts/spec-census.sh` as the three-way drift gate.

## Suggested order of work

The numbered interior roster is closed except accepted keeps (**05**,
**06** scan, **13** one-channel). Remaining release work:

1. **18** live ramdisk lock on bare metal (`--ignored`) — no release
   checklist exists under `docs/**`; the one-liner stays in
   [18](18-ramdisk-probe.md).
2. **06** scan unification only if a catalog member is built at lease
   birth (the recorded keep).
