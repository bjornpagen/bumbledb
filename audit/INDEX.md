# Audit index — 0.15 interior, live-tree pass

Lens: [REQUIRED-READING.md](REQUIRED-READING.md). Format **8**, ABI **3**,
crate **0.15.0** shipped on `main` (`d2ed04ab`); everything here is the
working-tree interior. One numbered file per open issue (convention in
[README.md](README.md)); standing do-not-fix rulings in [kept.md](kept.md);
downstream gaps in [primer-integration.md](primer-integration.md). The
earlier 0.13 and 0.15 second-pass area files are deleted — every still-open
row was carried into a numbered file, and the fixed rows' record is git
history.

The tree is hot. Statuses below were verified 2026-08-19 ~17:20 EDT and
agents land fixes continuously — trust the `Status` line inside each file
over this table when they disagree.

## Roster

| # | File | Status |
| --- | --- | --- |
| 01 | [napi error carrier](01-napi-error-carrier.md) | **fixed this pass** (`create_error` + `kind`; suite green pending re-run) |
| 02 | [one temporal shape](02-ts-temporal-shape.md) | **fixed this pass** — AsyncTask control plane; lease flag on publish |
| 03 | [owned single read](03-ts-owned-single-read.md) | **fixed this pass** — five direct owned reads; lease spelling gone |
| 04 | [TS builder verbs](04-ts-builder-verbs.md) | **fixed this pass** — full builder verbs; column load/insert |
| 05 | [WriteDelta lifetime](05-writedelta-lifetime.md) | keep **accepted** (cost ruling; reopening horizon recorded) |
| 06 | [instance one body](06-instance-one-body.md) | keep (scan) / OPEN (dict + scratch) — rustdoc `execute_args` ghosts **fixed this pass** |
| 07 | [view binding](07-view-binding.md) | **fixed this pass** — per-occurrence `Binding` / `OccMemo` |
| 08 | [relation slot](08-relation-slot.md) | **fixed this pass** — one `RelationSlot` table; SPINE-16 epochs from `ImageBind` |
| 09 | [profile + stats](09-profile-stats.md) | **fixed this pass** — `Instance::profile` + one `hit` |
| 10 | [codec value vocabulary](10-codec-value-vocabulary.md) | **fixed this pass** (typed decode; no `ValueRef` `Fixed*` arms) |
| 11 | [C ref slots](11-c-ref-slots.md) | **fixed this pass** (retired slots leak on destroy; `MISUSE` test pinned) |
| 12 | [C owner tokens](12-c-owner-tokens.md) | **fixed this pass** (`OwnerToken`; bridge pre-refusal test pinned) |
| 13 | [C exit threading](13-c-exit-threading.md) | keep — one `Result<()>` channel; rider spelling narrowed (unforgeable decline, later) |
| 14 | [V8 lazy accounting](14-v8-lazy-accounting.md) | **fixed this pass** (`OwnedSlot.accounted` cell; ops sync `retained_bytes`) |
| 15 | [exec one evaluator](15-exec-one-evaluator.md) | **fixed this pass** (one `holds` entry; two-module gate pinned) |
| 16 | [verify-store embedding](16-verify-store-embedding.md) | OPEN (later) — structural variants transcribe; `intern_id: u64` |
| 17 | [docs vocabulary](17-docs-vocabulary.md) | **fixed this pass** — fence table, snapshot prose, case count, Lean prose, census token |
| 18 | [ramdisk probe](18-ramdisk-probe.md) | OPEN (environmental) — capability probe, skip-with-reason |
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

1. **02** — make the async signatures honest (`fromInstance` first: it is
   O(catalog) on the event loop).
2. **03 + 04** — one way to read an owned instance; full builder verb set.
3. **10** — typed decode both sides; delete the last `unreachable!`s.
4. Owner rules on **07 / 09**, then whichever open.
5. **13** (unforgeable decline) + **06** (codec dict path, scratch pools,
   doc ghosts) + **15** (evaluator entries).
6. **16**, **17**, **18** in any order; **18** also gates release
   on bare metal.
