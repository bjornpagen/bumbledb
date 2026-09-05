# Campaign status — successor implementation

Orchestrator: P00 (Claude Fable, session 2026-09-04). Branch `codex/bumbledb-1-0`.
Base: `3b56de63` (proposal refactor) on top of preserved source `4b127782`.

## Phase

**F3 — final verification and repair (entered 2026-09-05).**

### F2 all-lanes-ready barrier — RECORDED 2026-09-05 (P00)

Every packet P01–P14 has delivered its source and authored tests into the
working tree (waves A–F, 28 agent lanes, all returned complete or with
boundaries subsequently closed). All shared declarations have real
implementations: the C01–C12 contracts each have a landed producer and
acknowledged consumers; the stopped-boundary checklist from chapter 64 was
explicitly closed (computed module + sink wiring: P03/P03R; numerical
guard hoisted: P03; general interiors: P03/P07R; owned snapshots: P02;
worker affinity: P06/P06R; complete Effect cutover: P07/P08; one FS CAS
authority: P05; real generated migration flow: P09/P10). No old public
compatibility path remains (fresh/braids/vector/dictionary/C surface/
Promise twins/TS protocol machine deleted; absence gate authored). All 68
audit IDs and all 17 parent/220 child gate families have owners and
designated lanes (implementation/verification-manifest.md). Remaining
recorded mid-cutover items are F3 repair/regeneration work, not missing
implementation. Verification: NotRun for everything — F3 owns the first
compile and every green claim.

F3 rules now in force: builds/tests/probes execute; repairs and repeated
checks are part of this one final campaign; no failure is suppressed,
skipped, golden-weakened or serialized away; missing credentials/runners
are NotRun blockers, never passes.

## Dispatch plan

- Wave A (producers): P01, P02, P04, P06, P11, P14 — concurrent.
- Wave B (consumers, after wave A integration): P03, P05, P07, P08, P09, P10.
- Wave C: P12, P13, plus resume/repair dispatches per actual boundaries.
- P00 integrates hubs between waves, then records the F2 barrier and runs F3.

## Exclusive write ownership (resolved from 62 defaults)

| Owner | Paths |
| --- | --- |
| P00 | `crates/bumbledb/src/lib.rs`, `crates/bumbledb/src/api.rs`, `crates/bumbledb/src/error.rs`, `crates/bumbledb/src/error/`, `crates/bumbledb-log/src/lib.rs`, all `Cargo.toml`/`Cargo.lock`, `rust-toolchain.toml`, `ts/src/index.ts`, `ts-log/src/index.ts`, `ts/package.json`, `ts-log/package.json`, locks, `scripts/` (except release-results.test.mjs → P12), root `README.md`, `.github/`, `implementation/campaign-status.md`, `implementation/release-results.json` |
| P01 | `crates/bumbledb-theory/` (whole crate incl. lib.rs), `crates/bumbledb/src/{canonical.rs,canonical/,encoding.rs,encoding/,interval.rs,interval/,schema.rs,schema/,changes.rs,changes/,scalar.rs,value.rs,allen.rs,work.rs,work/,digest.rs}` |
| P02 | `crates/bumbledb/src/{storage.rs,storage/,verify_store.rs,verify_store/,api/db/,arena.rs,alloc_counter.rs}` |
| P03 | `crates/bumbledb/src/{ir.rs,ir/,plan.rs,plan/,exec.rs,exec/,image.rs,image/,api/prepared.rs,api/prepared/,obs.rs,obs/}` |
| P04 | `crates/bumbledb-log/src/{history/,writer/,identities.rs,braids.rs (delete),vector.rs (delete),apply.rs,replica.rs}` + new command/receipt/authority modules under those dirs |
| P05 | `crates/bumbledb-log/src/{store.rs,store/,checkpointer.rs,gc.rs,lease.rs,manifest.rs,sidecar.rs,codec.rs,inspect.rs,bin/}`, new backup/restore/erasure/recovery modules, `crates/bumbledb-log/tests/` (lane support) |
| P06 | `ts/crate/src/` (whole, incl. lib.rs — transferred), `crates/bumbledb-log/src/tenants.rs`, `ts/src/{native.ts,runtime.ts,runtime-native.ts,runtime-codes.ts,runtime-errors.ts,marshal.ts}` |
| P07 | `ts/src/` (rest), `ts/test/` (except P12-prefixed files) |
| P08 | `ts-log/src/` except `schema.ts`/`migrations/`/`index.ts`, `ts-log/test/` (except P12-prefixed files) |
| P09 | new `crates/bumbledb-log/src/migration/` (+ its mod files), `crates/bumbledb-log/src/schema_file.rs` |
| P10 | `ts-log/src/schema.ts`, `ts-log/src/migrations/`, migration CLI modules + fixtures under ts-log |
| P11 | `lean/`, `crates/bumbledb-bench/src/{naive/,differential/,verify/,conformance/,corpus_gen/,lawful/,closure/,devhonesty/}`, proof bridge ledger |
| P12 | new `adversarial-*`/`gate-*` prefixed test files in `crates/bumbledb/tests/`, `crates/bumbledb-log/tests/`, `ts/test/`, `ts-log/test/`; `scripts/release-results.test.mjs`; `implementation/verification-manifest.md` |
| P13 | `ts/scripts/`, `ts-log/scripts/`, `ts/npm/`, `examples/`, `docs/reference/`, new permanent docs |
| P14 | `crates/bumbledb-bench/src/` (rest: scenarios/, harness/, sweep/, report/, sqlite_run/, cli/, capacity/, churn/, crud/, calendar/, displaced/, driver/, families/, float/, lanes/, primerlane/, querygen/, rings? via scenarios, trace_out/, translate/, windowed/), measurement docs |

Unassigned paths are unclaimed; ask P00. Hub edits go through "Hub patch
requests" in each `implementation/packets/Pxx.md`; P00 applies between waves.

## Contract registry (C01–C12)

| Contract | Producer | Status |
| --- | --- | --- |
| C01 values/schema/scalar | P01 | dispatched wave A |
| C02 changes/budgets/errors | P01+P06 (P00 hubs) | dispatched wave A |
| C03 candidate judgment/measures | P01 | dispatched wave A |
| C04 LMDB owner/snapshot/candidate | P02 | dispatched wave A |
| C05 query IR/results | P03 | wave B |
| C06 history authority/certainty | P04 | dispatched wave A |
| C07 backend operations | P05 (P04 agrees verbs wave A) | wave B |
| C08 checkpoints/roots/GC/admin | P05 | wave B |
| C09 Node runtime lifecycle | P06 | dispatched wave A |
| C10 public TS surface | P07 | wave B |
| C11 migration plans/history | P09+P10 joint | wave B |
| C12 formats/artifacts | P00 coordinated | F3 probes |

## Audit/gate ownership

As routed in 62 (all 68 audit IDs table + chapter 70 family routing) — adopted
verbatim as the assignment of record. P12 independently maps children to lanes
in `implementation/verification-manifest.md`.

## Barrier record

- F2 barrier: NOT reached.
- F3 verification: NOT started. Verification: NotRun for all packets.

## Wave A results (2026-09-04)

- P01 complete, P04 complete, P11 complete, P14 complete; P02 partial
  (old-path demolition pending cutover), P06 partial (worker-affine sessions,
  log.rs rewiring, tenants successor, publish entrypoint pending). Details in
  implementation/packets/Pxx.md. Verification: NotRun for all.
- Contracts landed: C01/C02/C03 (P01), C04 (P02), C06 (P04, + C07 verb
  proposal in P04.md for P05), C09 partial (P06: registry/handshake landed,
  affine sessions designed not wired).
- P00 hub patches applied: core lib.rs (fresh removal, interval/store/
  FixedIntervalElement exports), error.rs/display.rs stale variant deletion,
  log lib.rs braids/vector mod removal, bench lib.rs (+appperf/hashprobe/
  largefix/space), bench Cargo.toml (+aegis=0.9.15, blake3), bench main.rs
  dispatch arms, bench fixture.rs fresh() deletion + sqlmap/querygen callers.
- Wave B additional assignments: P07 gains crates/bumbledb-macros,
  crates/bumbledb-query, crates/bumbledb-query-macros (exclusive).
  P02-resume: old transitional path demolition + api/db re-point onto
  storage/store. P06-resume: recorded boundaries in P06.md. P14-resume:
  bench Generation/fresh purge (sqlmap.rs:35, primerlane/corpus.rs,
  querygen/coverage.rs), nosync-lane replacement, corpus-float CLI arm.
- Known cross-lane red (expected mid-cutover, owed green at F3): old api/db
  path dangling from P01 roster changes; log codec/sidecar/manifest/
  checkpointer/gc/inspect/tenants/bin reference deleted braids/vector/writer
  surfaces (P05/P06 wave B); ts/src db.ts red against deleted native verbs
  (P07 wave B); api/db/tests.rs references deleted DynIdError::NotAFreshField
  (P02-resume).

## Wave B results (2026-09-05)

- Complete: P05, P06R, P08, P09, P10, P02R, P14R. Partial: P03 (sink-internal
  spill for aggregate/rec state not wired; ScratchRelation + charging exist),
  P07 (native db-bridge verbs are P06's deliverable; computed find terms TS
  wire + typed Rust templates pending). Details in packet files.
- Contracts landed: C05 (P03), C07/C08 (P05, C07 verbs agreed with P04's
  proposal), C09 (P06 complete incl. worker-affine reactor sessions), C10
  (P07 core + P08 log), C11 (P09 native + P10 generator, reconciled).
- P00 hub patches applied: core lib.rs (arena removal, integration re-point,
  store glob, STORAGE_FORMAT_VERSION→store LAYOUT, prepared-result roster,
  query! re-export, fresh doc examples rewritten), error hub (Store variant/
  family/descriptor/display + from_store, FreshExhausted deleted incl.
  ts/crate tags row + TS code union, heed ReadersFull→Store mapping,
  from_commit deleted, census EXEMPT re-point), core Cargo.toml
  (collision-probe feature, query-macros dep), schema.rs value_matches now
  pub (P09), log lib.rs roster (admin/backup/erase/local_roots/recovery/
  restore/migration in; lease/sidecar out), ts/src/index.ts replaced (P07
  barrel + nativeOperation/With), ts-log/src/index.ts replaced (P08 barrel),
  ts-log/package.json (subpaths, bin, deps pruned, description), ts-log
  codes/errors migration reasons (P10), bench main.rs CorpusFloat arm,
  devhonesty ephemeral line, bench-night.sh nosync lanes removed, stale
  conformance/v3 0.x corpus subdirs deleted (identities.json kept).
- P00 decisions recorded: {0..*} capacity acceptance as trivially-satisfied
  law CONFIRMED (P01 review concern 5); Db::write's embedded WorkContext
  unbounded-by-design ACCEPTED (host budgets arrive via the integration
  surface, P02R concern 2); dead-but-marshalled error variants
  (AlreadyInitialized/EnvironmentLocked/PublishedButUnsynced/ReadersFull/
  FormatMismatch/CommitSync) retained until the F3 sweep with P06.
- Cross-lane defects found by P05 (to P04R): blank_initial_digests domain
  mismatch vs canonical empty export; hosted.rs bare control-frame head
  bodies + constructor-time staging epoch violate the GC reference rule;
  TailPolicy + LogError::MaintenanceRequired missing. Exact patches in
  P05.md.

## Wave C/D results (2026-09-05)

- Wave C complete (7/7): P12 (verification manifest mapping all 68 audit IDs
  + 220 child families to lanes, 9 external-blocked lanes named; adversarial
  process/hostile-bytes/trace/close-drain/boundary/capability suites;
  checker regressions), P13 (immutable pack staging, absence gate, consumer
  fixtures, examples/notes Next.js+Alchemy app, docs/reference permanent
  docs, PUBLISHING.md — ownership ratified by P00), P01R (canonical
  Violations evidence codec bumbledb.evidence.v1 + CommandResult codec
  bumbledb.result.v1 + independent cast oracles), P03R (sink-internal spill
  wired for aggregate/rec state; quoted id128 render; compute audit), P04R
  (P05's defect patches applied: blank_initial_digests, composed head
  bodies, parent-epoch staging, TailPolicy, MaintenanceRequired/
  MaterializationStale; evidence codec consumed; decide.rs restructure;
  resolve_after_unknown certainty fix), P06R2 (complete db-native +
  LogNative + migration verb rosters in ts/crate; OpenKind + CloseWire
  mappings resolved), P07R (TS computed find terms; typed Rust templates +
  params!; barrel merge fix).
- P00 hub patches applied after wave C: log dev-dep on bench (P12), P13's
  full set (packed-consumer.ts, packed-import.sh, battery.sh lanes,
  log lib.rs doc(hidden) roster, ts/package.json hook removal + description,
  banned-tokens/spec-census scope move, README packages section),
  ts/crate log dep default-features removal, index.ts Schema/Compute fix,
  core params! re-export, P12 test open-arity fix, ChangeSet::records/
  ChangeRef + canonical::decode/DecodedRow doc(hidden) pub.
- P00 decisions: docs/cookbook.md fence regeneration deferred to the F3
  sync-test-driven pass; absence gate's strict all-doc(hidden) form kept;
  bind() infallible-by-typestate deviation from chapter 34's spelling
  accepted (fallibility lives on prepare/execute).
- Wave D dispatched: D1-core (retained-result rebind_work +
  execute_complete pub + Violations truncation label), D2-log (hosted
  read-side catch-up verb, per-call submit attempts seam, migration
  hosted.rs composed-head CAS), D3-tslog (optional wire fields,
  MaterializationStale code, CommandScalar mapping confirmation).
- F3 notes carried: pin runtimeTake double-take behavior (P12→P06);
  examples/notes migration artifacts generate at F3; goldens regenerate at
  F3 (identities.json, ts/crate/log-identities.json, notation-corpus
  fingerprints, bench Ledger pin, theorygen corpora, float corpus).

## Wave D/E results (2026-09-05)

- Wave D complete (3/3): D1-core (execute_complete pub; CompleteResult/
  ResultCursor byte_len + rebind_work over ScratchRelation::rebind_work;
  Violations truncation label threaded through error.rs/violations.rs/
  evidence.rs — P01 hub request 6 APPLIED), D2-log (public
  HostedHistory::catch_up read-side verb; SubmitOptions + submit_with
  per-call bounded attempts/backoff; migration/hosted.rs moved onto the
  composed HeadRecord grammar mirroring writer/hosted.rs; no new identity
  rows needed), D3-tslog (optional wire fields snapshots/schema/backup/
  binding threaded as typed options; MaterializationStale code — roster now
  33; CommandScalar mapping confirmed+amended: TS never emits tag 8).
- Wave E dispatched: P06R3 (delete the bumbledb.result.v1 codec twin in
  log_wire and re-point at canonical::result — a real C12 defect found by
  D3; 33-code roster pin; fail_of_log exhaustive with structured frame
  payloads for all declared structured reasons; runtimeTake double-take
  pinned to typed refusal), P12R (REC-05 mid-hydrate hold-revocation real
  process arm; manifest debt 8.2).

## Log

- 2026-09-04: F0 ownership + contract registry recorded; wave A dispatched.
- 2026-09-04: Wave A returned (6/6). Hub patches applied. Wave B dispatched:
  P03, P05, P07, P08, P09, P10, P02-resume, P06-resume, P14-resume.
- 2026-09-05: Wave B returned (9/9). Hub patches applied. Wave C dispatched:
  P12, P13, P01R (Violations codec), P03R (spill wiring), P04R (P05 patches),
  P06R2 (native verb roster), P07R (computed terms + typed templates).
