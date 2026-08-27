# 60 — The C surface: as-is, the would-be log ABI, and the verdict

Scope: the C-ABI angle of the shared-core refactor. Canon deployment
case 2 is cited from git history: `proposals/settlement/` was deleted
whole at 49d45b5c ("The proposals directory is complete. Settlement and
lockstep live in the code and the gates"), so the reference text is
`49d45b5c^:proposals/settlement/00-canon.md`. Case 2 (canon lines
69–70): "**Embedded macOS (Apple Silicon)** — engine as today; the log
as optional sync/backup in resident mode, via napi or the C ABI."

## 1. As-is: how bumbledb-c wraps the engine

**Crate identity.** A workspace-excluded leaf (root `Cargo.toml:13-16`)
building `staticlib` + `cdylib` (`crates/bumbledb-c/Cargo.toml:9-12`),
whose only dependency is the engine (`crates/bumbledb-c/Cargo.toml:29-31`).
The dumb-bridge law is the first line of the crate: "no logic beyond
marshaling will EVER live in this crate" (`crates/bumbledb-c/src/lib.rs:1-4`)
— the same law as the napi bridge (`ts/crate/src/lib.rs:1-2`). `unsafe_code`
is denied crate-wide with per-site `#[expect]` escapes
(`crates/bumbledb-c/Cargo.toml:15-17`).

**Handle model.** Everything opaque is a boxed handle minted by
`box_out`/`box_out_to` and reclaimed exactly once by `box_in`
(`crates/bumbledb-c/src/lib.rs:253-289`). The engine is monomorphized
once: `type Engine = Db<SchemaDescriptor>` (`crates/bumbledb-c/src/db.rs:24`).
The handles:

- `bdb_db { Arc<Engine>, descriptor, phase, retired }`
  (`crates/bumbledb-c/src/db.rs:42-47`) — `phase` is a reader/writer
  bitfield (`db.rs:26-28`), `retired` stashes spent boxes so a stashed C
  pointer stays allocated and answers `MISUSE` instead of
  use-after-free (`db.rs:49-57, 292-298`).
- Lexical capabilities: each read/write callback mints a
  `bdb_instance_ref` (`db.rs:72-78`), store reads also lend a
  `bdb_witness` (`db.rs:83-88`, retainable via `bdb_witness_retain`
  `db.rs:1252`), writes lend a `bdb_tx_ref` (`db.rs:92-97`). All carry
  `alive: AtomicBool`; the slot dies when the callback returns.
- Callbacks are `extern "C"` fn pointers + `void* context` returning a
  `bdb_callback_control` tag (`db.rs:205-218`,
  `lib.rs:88-93`): `Ok` commits/completes, `Abort` drops the delta and
  the outer call answers `BDB_STATUS_ABORTED`.

**Error model.** Every fallible export returns `bdb_status` and takes a
trailing `bdb_error**` out-param (`lib.rs:73-80`; the header contract
spells it at `cbindgen.toml:27-34`): `OK`, `ABORTED` (no error),
`ERROR` (caller owns a `bdb_error`, frees via `bdb_error_destroy`),
`MISUSE` (contract violation, nothing allocated). The error is
origin + kind + rendered message (`crates/bumbledb-c/src/error.rs:16-19,
26-54, 121-125`) — the kind table is "the FOURTH spelling of the engine
taxonomy" (`error.rs:4-5`). Theory rejection is *not* an error: it rides
`bdb_violations` on the admission rejected arm (`error.rs:1-3, 113-117`),
and proved write outcomes are tagged admission unions returned under
`STATUS_OK` (`db.rs:134-189`).

**Memory ownership.** Constructor mints, matching destroy reclaims,
exactly once (`lib.rs:273-289`). Views handed OUT borrow their carrier
(`bdb_row_set`, `bdb_answers`, `bdb_error`) and die with it; views
handed IN are copied before return (`cbindgen.toml:48-51`). Out-params
are written only after a null check, and a value bound for a null slot
is dropped, never leaked (`lib.rs:257-271`); a previous error in the
slot is dropped before the new one lands (`lib.rs:143-163`).

**Panic boundary.** Every export body runs under `catch_unwind`
(`lib.rs:105-141`); a panic becomes a `bdb_error` of kind `Panic`
("panic across the bridge (store poisoned)", `error.rs:231-241`);
statusless accessors return fallbacks via `guard_value`
(`lib.rs:165-167`). The header warns C++ exceptions may not unwind
through a callback (`cbindgen.toml:44-47`).

**GENERATION discipline.** Two counters, deliberately decoupled:
`bdb_version()` bakes `CARGO_PKG_VERSION` (`lib.rs:30-34`, rides the
version lockstep via `scripts/version-roster.txt`), while
`bdb_abi_version()` is the layout generation, currently **4**, whose
doc comment names each break's cause (`lib.rs:36-49`).
`ts/PUBLISHING.md:263-265`: "`bdb_abi_version()` is layout generation,
not the release spelling." The header is generated and committed;
CI regenerates with pinned cbindgen 0.29.4 and diffs against the
committed file (`.github/workflows/c-abi.yml:94-100`). `cbindgen.toml`
pins the tool (`:1-2`), emits C with `cpp_compat` (`:11-12`), embeds
the boundary-protocol contract as the header banner (`:19-52`), forces
the tag enums into the header via `export.include` because wire fields
are `u32` and cbindgen drops unused types (`:54-92`), and sets
`parse_deps = false` (`:97-98`).

**Size.** 51 exported functions, a 966-line header, `src/tests.rs`
(64 KB) larger than any impl file, plus a pure-C compile smoke
(`crates/bumbledb-c/tests/c_smoke.c`). CI runs its own lane on
macOS + amazonlinux:2023, triggered by any engine/theory change
(`c-abi.yml:15-30`).

## 2. The would-be bumbledb-log C surface (case 2 sketch)

Case 2 is an embedded app as **resident writer** with the log as
optional sync/backup. The Rust surface it must marshal:

**Expose (≈15–20 verbs, vs the engine's 51):**

- **Store config as data, not trait.**
  `bdb_log_store_fs(root)` / `bdb_log_store_s3(endpoint, region,
  bucket, prefix, static keys)` → `FsStore`
  (`crates/bumbledb-log/src/store/fs.rs:216`) and `S3Store::new`
  (`crates/bumbledb-log/src/store/s3.rs:78`). The `ObjectStore` trait
  itself cannot cross: `put_create`/`put_swap` take
  `impl Into<Fenced<'_>>` (`crates/bumbledb-log/src/store.rs:384,
  393-397`) so the trait is not dyn-safe — the bridge monomorphizes a
  closed Fs|S3 enum internally, exactly as `db.rs:24` monomorphizes the
  engine.
- **Writer open**: `bdb_log_writer_open(store, prefix, dir,
  schema_spec, writer_id, ack_mode, out_admission, out_error)` →
  `Writer::open` (`crates/bumbledb-log/src/writer/mod.rs:438-446`) with
  `Options { writer_id, ack }` (`writer/mod.rs:135-138`). Theory =
  `SchemaDescriptor` via the existing `schema_spec_in`
  (`crates/bumbledb-c/src/schema.rs`). `WriterOpened::Refused(OpenRefusal)`
  (`writer/mod.rs:359-362`) is an admission-union arm, not an error —
  the `bdb_db_admission` pattern verbatim.
- **Commit**: `bdb_log_writer_commit(writer, callback, context,
  out_commit, out_error)` → `Writer::commit`
  (`writer/mod.rs:556-579`). The callback receives a batch ref whose
  verbs are `insert`/`delete`/`reserve`/`reserve_capacity`
  (`crates/bumbledb-log/src/writer/batch.rs:34-79`) — the same
  row marshaling the engine surface already ships
  (`bdb_tx_insert`/`_delete`/`_reserve`, `db.rs:1287, 1321, 1413`).
  Outcome union: `Accepted { braid, generation, durability } |
  Rejected(violations)` (`writer/mod.rs:90-98`) — `bdb_violations`
  reused wholesale. `commit_split` (`writer/mod.rs:588-623`) is the
  second verb, returning a `BraidOutcome` array (`writer/mod.rs:102-112`).
- **Read**: a callback over `with_db` (`writer/mod.rs:630-633`) minting
  the existing `bdb_instance_ref` — the whole engine query surface
  (prepare/execute/scan/get) is reused with zero new marshaling.
- **Observability getters**: `vector` (`writer/mod.rs:644`), `losses`
  (`:657`), `deposition` (`:664`, the resident-writer usurpation signal
  `writer/mod.rs:206-216`), `backlog` (`:672`), `wedged_braids`
  (`:681`), `set_checkpoint_cadence` (`:687`), `quiesce` (`:694`).
- **Duty/checkpoint: no new verbs needed.** The resident writer already
  runs checkpoint duty on detached threads — "cadence detection on the
  commit path, compact and CAS off the lock. Commits never wait on the
  duty" (`crates/bumbledb-log/src/writer/duty.rs:1-4`). Cadence and
  quiesce above are the whole control surface.
- **Replica**: `bdb_log_replica_open` (`crates/bumbledb-log/src/replica.rs:322`),
  `refresh` (`:403`), `wait_for(vector)` (`:410`), read-callback over
  `db()` (`:358`), `dispose` (`:433`). `Vector` marshals as
  `(braid u32, count u64)` pairs (`crates/bumbledb-log/src/vector.rs:49`;
  `BraidId(RelationId)` at `crates/bumbledb-log/src/braids.rs:17`,
  `RelationId(pub u32)` at `crates/bumbledb-theory/src/schema.rs:13`).

**Do NOT expose:**

- `StepHook`/`open_hooked` — the fault-injection seam for the
  conformance crash matrices (`writer/mod.rs:154-188, 462`).
- `Checkpointer`/`Gc`/`inspect` as C API — the detached-duty deployment
  unit is the standalone duty binary
  (`crates/bumbledb-log/src/bin/duty.rs:1-4`), already shipped as the
  `bumbledb-log-duty` artifact (`.github/workflows/bumbledb-log.yml:135-159`);
  resident duty lives inside `Writer`.
- The `ObjectStore` trait, `MemStore` (single-process test store,
  `store/mem.rs`), lease/fence internals, `codec`/`manifest`/`sidecar`
  grammars, `Chain` (the `vector` + generation getters suffice;
  `Chain` is the internal algebra).
- `tenants` (the per-tenant replica LRU is deployment case 4, not 2).
- The S3 credential-**refresh** arm in v1: `S3Credentials::Refresh` is
  a `Send + Sync` closure consulted per request
  (`store/s3.rs:34-41`), i.e. a C function pointer that would fire on
  detached driver threads. Static keys only, until a consumer demands
  rotation.
- A second generation counter comes with it: `bdb_log_abi_version()`,
  separate crate, separate header, separate GENERATION — so log-layout
  breaks never force engine-host recompiles.

## 3. What the engine C surface already leaks log-adjacent

- **The manifest fingerprint, exactly.** `bdb_db_fingerprint` renders
  64 hex chars of `bumbledb::schema::fingerprint::fingerprint`
  (`crates/bumbledb-c/src/db.rs:277-290, 860`). That is byte-for-byte
  the value the log manifest records and the open gauntlet compares
  (`crates/bumbledb-log/src/replica.rs:18, 936`;
  `writer/mod.rs:541-546`; `manifest.rs:190`). A C host can already
  compute the value that decides log open/refuse.
- **The sum-generation.** `bdb_write_admission.accepted_generation`
  (`db.rs:176-189`, filled from `committed.generation.value()` at
  `db.rs:1160`) and `bdb_moved_generations { witnessed, current }`
  (`db.rs:143-146`) expose the engine `GenerationId` — which canon laws
  2 and 6 pin as `≡ Σ vector` on every honest store. The engine header
  already speaks the log's store-wide generation. Caution for the log
  header: `Commit::generation` is the **braid slot**, "never the
  store-wide sum" (`writer/mod.rs:86-94`) — two numbers named
  "generation" must not share a spelling in C.
- **Not leaked, correctly:** `db.catalog_digest()` — consumed by the
  checkpoint/gc audit (`crates/bumbledb-log/src/checkpointer.rs:252`,
  `gc.rs:562-567`, `replica.rs:683`) — has no C export, and there is no
  standalone generation getter (the napi bridge has `db_generation`,
  `ts/crate/src/lib.rs:514`; C only rides admissions). The napi
  bridge's digest/descriptor lends (`blake3_hash`, `descriptor` —
  `ts/crate/src/lib.rs:40-62`) exist only because the TS log driver is
  a *reimplementation* that needs the engine's hash; a C log bridge
  links the Rust driver whole and needs no lends. Keep these out of the
  engine header — they belong to the log surface if it ships.

## 4. Shared-core conflicts (napi + cbindgen over one core)

- **Crate features: no conflict.** The engine's features
  (`trace`, `alloc-counter`, `ground-off`,
  `crates/bumbledb/Cargo.toml:7-11`) are default-off and neither bridge
  enables any (`crates/bumbledb-c/Cargo.toml:29-31`,
  `ts/crate/Cargo.toml`). Both bridges are workspace-excluded with
  their own lockfiles (root `Cargo.toml:13-16`), so feature unification
  cannot cross-contaminate — the c-abi lane exists precisely so "the
  engine stays heed+blake3-pure" (`c-abi.yml:1-2`).
- **Async runtime: no conflict by construction.** The log's public
  surface is already synchronous; tokio is a private detail of
  `S3Store`, which builds its own multi-thread runtime at construction,
  blocks every verb on it, and *refuses* construction from inside an
  ambient async context (`store/s3.rs:69-117`, refusal at `:79-85`,
  `spawn_blocking` for credentials at `:403`). From C there is never an
  ambient runtime. The real cost is dependency mass: a log C staticlib
  inherits `tokio` + `object_store` + the AWS stack
  (`crates/bumbledb-log/Cargo.toml:9-11`) where today's `bumbledb-c`
  tree is engine + heed + blake3 only.
- **Panic boundaries: one new wrinkle.** The engine bridge's
  `catch_unwind` guard pattern transfers directly. But the log crate
  spawns detached threads (publisher + checkpoint duty,
  `writer/mod.rs:364-367`, joined by `quiesce` `:694`); a panic there
  never crosses an FFI frame — it kills the thread and surfaces as
  operational state. The bridge must document that and lean on the
  `losses`/`deposition`/`wedged_braids` getters, plus the crate's own
  stderr scream (`writer/mod.rs:372-415`).
- **The real asymmetry: napi does not bridge the log.** `ts-log` is a
  deliberate pure-TypeScript reimplementation — "byte-exact against the
  Rust driver and pinned by cross-language goldens"
  (`ts-log/README.md:1-24`), enforced by the parity/conformance lanes
  (`crates/bumbledb-log/tests/f7_parity.rs`, `conformance_v3.rs`,
  `lane_b_interop.rs`; canon representation 1: "neither driver defines
  arms"). So "one shared Rust core serving napi and cbindgen" is the
  *engine's* architecture, not the log's: case 2's "via napi" arm is
  satisfied by ts-log today, and a log C ABI would make C the **first
  foreign consumer of the Rust log driver** — a new pattern, not a
  third leg of a proven one.

## 5. Historical maintenance cost of the C surface

33 commits on `crates/bumbledb-c` over 11 days (born 2026-08-15,
`4a124438` "Extract the C ABI into Rust and delete the C++ SDK",
+6946 lines). Breakdown by numstat:

- 8 lockstep version bumps (~5/5 lines each: 0.14→0.19.2).
- 17 mechanical comment-purge commits (2026-08-22 campaign).
- ~7 substantive — and each tracks an engine surface change at
  rewrite scale: 0.14.0 collection-write algebra +577/−349
  (`d6d06cf0`); **0.15.0 instance-lifetime cutover +2020/−1138 = ABI 3**
  (`d2ed04ab`); hot-interior and hatch passes +215 (`763d1544`,
  `37f17023`); **0.17.0 measure/duration purge = ABI 4** (`3334b591`,
  `e1e6da4a`).
- **Two ABI generation breaks in 7 days** (ABI 3 on 08-19, ABI 4 on
  08-22), each a forced host recompile (`lib.rs:36-49`).
- Relative churn since the crate's birth: engine 865 commits, log 153,
  ts-log 110, napi bridge 40, C bridge 33. The bridge is ~4% of engine
  commit volume, but its CI lane triggers on every engine/theory change
  (`c-abi.yml:18-30`), and its test mass (64 KB `tests.rs` +
  `c_smoke.c`) is at rough parity with the impl — every new verb pays
  double.
- No in-repo consumer of the C ABI exists beyond the compile smoke
  (`examples/` holds only the Node lambda) — the engine C surface is
  ahead of its consumers, and its churn was paid anyway.

## Recommendation: defer, with the trigger written down

**Do not ship a bumbledb-log C ABI with the shared-core refactor.**
Reasons, in order of weight:

1. **The log's surface is still moving faster than an ABI can afford.**
   153 commits in 11 days; the settlement canon itself was rotated in
   and deleted-as-complete within that window. The engine C surface —
   wrapping a far older core — still broke its ABI twice in 7 days.
   Freezing a C layout over `Writer`/`Replica` now buys generation
   churn, and every break is a host recompile.
2. **Case 2 is not blocked.** The engine-over-C arm exists; the
   log-as-sync arm for embedded hosts rides napi/ts-log today; detached
   checkpoint/GC ships as the `bumbledb-log-duty` binary. The only
   unserved consumer is a *non-Node* embedded app wanting a resident
   writer — and no such consumer exists in or around the repo (the
   engine C ABI itself has none beyond the smoke test).
3. **The cost profile is known and front-loaded.** The engine bridge's
   history says a C surface costs a rewrite-scale commit per core
   surface event plus test mass at parity. The log bridge would add the
   tokio/object_store/AWS dependency tree to a staticlib and make C the
   first foreign consumer of the Rust driver.

**What the shared-core refactor should preserve so the deferral stays
cheap** (all already true; keep them true):

- The log's public verbs stay synchronous — tokio stays private to
  `S3Store` (`store/s3.rs:69-117`).
- The surface stays monomorphizable: `Theory = SchemaDescriptor`
  (`bin/duty.rs` proves it), stores enumerable as Fs|S3.
- The bridge patterns stay reusable: admission unions, `bdb_violations`,
  callback-control, and `bdb_value` row marshaling transfer verbatim —
  the future crate is ~15–20 verbs, mostly mechanical.
- Keep log-adjacent verbs (catalog digest, generation getters, digest
  lends) **out** of the engine header, so the engine ABI generation
  never moves for log reasons.

**Ship trigger:** a named non-Node embedded consumer, or the
writer/replica verb set surviving one full release cycle without a
breaking change — whichever comes first. At that point mint
`bumbledb-log-c` as a sibling leaf crate (own lockfile, own
`cbindgen.toml`, own `bdb_log_abi_version()` generation, own CI lane on
the c-abi.yml pattern), never as growth of `bumbledb-c`.
