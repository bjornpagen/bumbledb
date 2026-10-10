# bumbledb cutover plan (living document)

Status: **done, 2026-10-09.** All nine lanes and the consolidator landed on `main`; the release is **2.0.0** ([release notes](release-2.0.md)).

## Outcome

- **Landed:** A, B, C1–C17, D (sans-IO log core, checkpoints, migrations, Freeze/Migration/Thaw), E1–E8, F (one package, generated bridge outputs, serde inputs, one validator, `Database`), G (CI lanes, one allocation counter, one compile-fail runner, README and cookbook as doctests, bench restructure, toolchain bump script), H, I and L. D20 `bdb` naming is applied repo-wide. Swarm tooling and lane boards are deleted.
- **Final local gate:** fmt; clippy default and `--all-features`; rustdoc `-D warnings`; cargo-deny; cargo-shear; nextest 1934 passed, 12 skipped (the `deep` sweeps, the bless test, the crash child); doctests (README and cookbook included); addon build, TS lint, typecheck, `dts.sh --check`, 283 TS tests, package pack and smoke; notes typecheck, 8 tests, `migrations:check`, `next build`.
- **Not run locally:** the full Miri suite (the tests rescaled for it were run individually), musl (the dev machine has no virtualization), the deep sweeps, udeps and benchmarks. 2.0 has not been benchmarked; `docs/perf/results.md` is the 1.3.0 run.
- **Open, for the owner:** U10 (Lambda `nodejs26.x` availability); the `release` environment and npm trusted publishing for `release.yml`; U12 log GC stays deferred. There are no S3 CI lanes, so U13 is void: the store conformance suite passed by hand against real S3 Express and Standard buckets.

**Governing law:** [docs/design/representation-first.md](design/representation-first.md). Every item below is justified as a representation change, not a new branch. Items are tagged with the rule they apply: R1–R7, the bumbledb rules at the end of that document.

---

## 0. Ground rules (owner decisions)

| # | Decision |
|---|---|
| D0 | **Representation first** is the governing design law (CLAUDE.md). |
| D1 | **No users of bumbledb or bumbledb-log, so hard cutover and cull all legacy.** Old formats are trashed with **zero** backward compatibility: no readers, refusals, migrators or detectors for prior layouts/frames/wire, no compat aliases, no "retired"/"successor"/"transitional" code, no tests of old formats. Every version label and tag resets to 1. The release is **2.0.0** (workstream L). |
| D2 | **Nightly Rust is mandatory.** Bump to nightly-2026-10-09 (rustc 1.101, LLVM 23.1.1) and make future bumps cheap. Never move to stable. |
| D3 | **Bump every dependency to latest:** Rust crates, Effect 4.0.2, TypeScript 7, Node 26, napi-rs 3.14, pnpm 12, biome, GitHub Actions, and the example app's dependencies. |
| D4 | **Keep hosted S3 and make it fast.** It should become the owner's personal serverless database. The protocol may be redesigned (Turso research in flight). |
| D5 | **bumbledb-log is dead simple.** Its only job is to make bumbledb work on the S3 path. Performance work stops at "no waste". |
| D6 | **Migrations work like Expo/Drizzle.** They are bundled with app code, applied automatically on deploy or startup, and recorded in the log. |
| D7 | **fearless_simd replaces std::simd** for all portable kernels on every backend, with runtime dispatch on x86. **Keep the hand-tuned NEON paths.** |
| D8 | **Delete the second join evaluator** and all beyond-u32 machinery (§1). |
| D9 | **Workflow:** commit directly on `main`, no worktrees, no adversarial review waves. The repo's tests are the gate. |
| D10 | **Node 26 hard upgrade.** `engines: >=26` everywhere. Corepack is gone in Node 25+, so CI uses pnpm/action-setup. |
| D11 | **Backup/restore are dropped.** No backup, verify or restore API or formats. The immutable log plus Standard-bucket versioning on `ckpt/` is the safety net. `fork`/time-travel is deferred. |
| D12 | **Maximal on-disk churn.** The LMDB layout, checkpoint images and every frame format are redesigned for the best end state (§2b U3, §3 C16). Nothing is kept for compatibility. |
| D13 | **The S3 protocol follows the research verdict:** create-only `log/{seq}` on S3 Express, `ckpt/` on S3 Standard, a nonce per submission, `Idle \| InFlight` writer state, entries carrying ChangeSets, timer-free group commit, no deletes on `log/` (§3 D). |
| D14 | **Sans-IO Rust log core;** S3 I/O lives in TS/Effect through the app's S3 client (resolves Q2). |
| D15 | **NaN semantics option C** (IEEE/JS); **the FP guard becomes a read-only check** (resolves Q3 and Q4). |
| D16 | **Fixed virtual LMDB map** (resolves Q5/U1). One configurable ceiling (default 1 TiB) set at open. `Error::Full{ceiling}`. Elastic resize, grow/retry loops and drain-for-resize are deleted. |
| D17 | **Drop pure authoring** (resolves Q6/U2). Schema and query construction call the addon synchronously and return validated handles. The engine validator is the only validator. |
| D19 | **Latest pnpm only (12.10.1).** No pnpm 11 anywhere: `packageManager`, CI (`pnpm/action-setup@v6.1`), lockfiles re-resolved. |
| D20 | **Canonical extension `bdb`.** Every file, format, and artifact name that used "bumbledb" as an extension or namespace becomes `bdb`: `<name>.bdb` database dirs and checkpoint images, `.bdb/` notes data dir, `bdb.lock`, `bdb.<kind>.v1` format tags (result, evidence, every log frame), `bdb.<platform>.node` addon files, `bdb.*` TS brands and symbols. Crate names, npm package names, the `bumbledb` CLI and env vars are unchanged. Applies to code, tests, docs, notes and scripts. Each lane applies it in its own paths; the consolidator finishes it repo-wide with a grep gate. |
| D18 | **All §2b recommendations accepted.** Cache `MDB_NOSYNC`; S3 time for Freeze deadlines; `bumbledb migrate` in the deploy pipeline with prod `onOpen: "verify"`; non-VPC Lambda by default with Express latency measured in CI; a container image if Lambda `nodejs26.x` isn't available; sans-IO `Machine::step` interface; v1 accepts the rolling-deploy window, writer contention and log growth. |

## 1. What the "beyond-u32 machinery" is

The query engine keeps each relation in RAM as a column image. Rows are numbered with 32-bit integers, so one relation can hold at most about 4.29 billion rows. The dedup and group-by hash tables (`WordMap`, at 33% load) top out at about 1.43 billion entries.

For data past those limits there is a second, disk-backed engine:

- **A second join evaluator** that walks LMDB cursors row by row: `api/prepared/fallback.rs`, 1,085 lines. It is selected at `execute.rs:244-250`.
- **Throwaway temp-LMDB "spill" tiers** for dedup sets, GROUP BY tables and recursive-query stages:
  - `exec/scratch.rs`
  - `exec/sink.rs:454-520`
  - `exec/sink/aggregate/spill.rs`
  - `api/prepared/reach/spill_bounded.rs`

None of this can trigger on the target hardware, a 512 MB Pi Zero 2: 4.29 billion rows is at least 34 GB. Tests reach it only by forcing it (`force_spill`, `force_cursor_fallback`). Even the constraint checker's spill path is `#[cfg(test)]`.

**Replacement:** a typed `Error::Capacity(ResidentRows | DistinctRows | Groups | ResultBytes)` at the two decision points.
**Size:** about −4.5k product lines and −3k test lines.
**Oracle role:** the fallback also served as a differential test oracle. That moves to the bench crate's independent `naive` evaluator and the SQLite differential.

## 2. Open questions

| # | Question | Status |
|---|---|---|
| Q1 | Hosted protocol | **Resolved → D13** |
| Q2 | Where S3 I/O lives | **Resolved → D14** |
| Q3 | Float NaN semantics | **Resolved → D15 (option C)** |
| Q4 | FP-environment guard | **Resolved → D15 (read-only check)** |
| Q5 | Fixed virtual LMDB map | **Resolved → D16** |
| Q6 | Pure authoring without the addon | **Resolved → D17 (dropped)** |
| Q7 | Node engines | **Resolved → D10 (`>=26`)** |

## 2b. Unresolved issues and proposed resolutions

| # | Issue | Proposed resolution | Status |
|---|---|---|---|
| U1 | **Elastic LMDB map.** Today the map starts at a 4 GiB virtual reservation (`map.rs:20`). When a write overflows it: the transaction aborts with MapFull (`candidate.rs:255`), `Store::grow` takes the 886-line gate exclusively (`gate.rs`), waits for **every** live read transaction to drain (heed's `resize` is UB with live txns), remaps larger, and replays the whole write. `copy.rs` has the same loop 3×. Error types: `MapFull`, `MapGrowthExhausted`, `ResizeBlockedByReaders`, `GrowReport`, `MapPolicy`. | **Fixed virtual ceiling** (e.g. 1 TiB, configurable) set once at open. A mapping costs address space, not RAM or disk: without `WRITEMAP` the file grows only as pages are written. The crate is already 64-bit only (`lib.rs:44`; 48-bit VA = 256 TiB). Exceeding the ceiling → one `Error::Full { ceiling }`. **Deletes:** grow, the retry loops, drain-for-resize (the gate shrinks to close-drain only), `map.rs`, 3 error variants (~−1.5k). **Verify first:** a 1 TiB map on macOS and Linux leaves the file small; `RLIMIT_AS` on Lambda and the Pi. | **accepted (D16)** |
| U2 | **Pure authoring.** Today `@bjornpagen/bumbledb` can be imported and schemas/queries built **without loading the `.node` addon** (lazy loader `ts/src/native.ts:537`; checked by `scripts/packed-pure-authoring.ts` with platform packages absent). So TS must re-implement engine checks at authoring time (part of the 78 `AuthoringError` sites in `query/lower.ts`, e.g. `:1124-1130` mirrors `ir/validate/finds.rs:155-197`). | **Drop it.** The README already says Node only, no browser or Edge, and the addon ships for every supported platform. Query/schema construction calls the addon **synchronously** (napi calls are sync) to parse into a branded `ValidatedQuery`/`Schema`, so the engine's validator is the only one (R3). TS keeps only checks that are TS-level by nature (unknown authoring names, lowering shape). Delete the mirrored semantic checks, `parseQueryIr` on self-produced IR, and `packed-pure-authoring`. Engine diagnostics carry rule/atom/find indices that TS maps back to authoring names. | **accepted (D17)** |
| U3 | **Checkpoint image representation and portability.** Images are compacted LMDB files written on any platform. macOS arm64 uses **16 KiB** pages; Linux (Lambda, Pi) uses 4 KiB. The store id is minted **from the path** (`format.rs:61`, re-minted in `copy.rs:199`), so images aren't relocatable. | (a) LMDB adopts the page size recorded in an existing file's meta page, so a 16 KiB image *should* open on 4 KiB Linux (all targets are 64-bit LE). **Test first** (log PR D4: darwin-arm64 image → linux-x64/arm64 in CI). (b) **Identity moves out of the file path:** the database identity is `databaseId` from log Genesis; the LMDB meta records `{databaseId, seq, revision, layout=1}`; no path-minted id. (c) **Verify by `Snapshot::content_digest()`** over canonical rows, never by file bytes. (d) If the portability test fails, the image becomes a canonical sorted row stream bulk-loaded on open. One representation either way, chosen by the test, never both. | **accepted** |
| U4 | **Cache durability.** The local LMDB is a disposable cache; the log is the authority. | Open the cache with `MDB_NOSYNC`, no `WRITEMAP`. LMDB keeps integrity (it loses only the tail) on write-order-preserving filesystems. A lost tail is re-applied by catch-up. If LMDB reports corruption on open → delete and rehydrate. Lambda `/tmp` is ephemeral anyway. In local FsStore mode the **log file** write is the fsynced authority (temp + fsync + `link` + dir fsync), so the Pi stays durable. | **accepted** |
| U5 | **S3 Express topology.** A directory bucket lives in **one AZ**. Turso's 6.4 ms numbers are same-AZ. Non-VPC Lambda may run in any AZ, and NAT caps bandwidth. | Config: `S3.store({ log: { bucket: "name--use1-az4--x-s3" }, checkpoints: { bucket: "name-ckpt" }, prefix })`. Default deployment is non-VPC Lambda in the bucket's region; **measure** cross-AZ commit latency in the Express CI lane. For the lowest latency, a documented VPC setup: subnets in the bucket's AZ plus an S3 gateway endpoint (no NAT). Express has no versioning; that's fine because `log/` objects are immutable and never deleted. Turn versioning on for the Standard `ckpt/` bucket. | **accepted; measure in CI** |
| U6 | **Freeze deadline clock.** Client wall clocks skew. | Time comes from the authority: deadline = the Freeze object's S3-assigned `Last-Modified` + duration, compared with the `Date` header of the latest S3 response. Never the local clock. FsStore uses file mtime + local clock (one machine). | **accepted** |
| U7 | **Where migrations run.** Lambda has a 15-minute cap, limited memory and ephemeral `/tmp`. | `bumbledb migrate` runs in the **deploy pipeline** (CI runner or dev machine with S3 credentials) before traffic shifts. Production `Database.layer({ onOpen: "verify" })` refuses with `MigrationPending`. `onOpen: "migrate"` is for dev and local. | **accepted** |
| U8 | **Rolling deploy window.** Between `migrate` and the alias/traffic shift, old instances get `SchemaAdvanced` (writes refused). | Accept for v1 (seconds; personal scale). Document the order: migrate → shift alias. Expand/contract compatibility, where old code keeps writing through additive migrations, is deferred; it would need schema-compatibility rules. | **accepted** |
| U9 | **Concurrent serverless writers.** Each Lambda instance races on `log/{t+1}`. Group commit only batches within one process. | A writer refused at slot n PUTs its batch at n+1 in the same step, with the commands queued behind it, while it GETs n, so every writer races for every slot. The entry records the head it was judged at and is judged again where it lands. A single-writer broker is deferred. | **done** |
| U10 | **Lambda `nodejs26.x` runtime availability.** AWS has historically added each Node LTS runtime about a month after it goes LTS; Node 26 goes LTS on 2026-10-28. | **Verify before the TS cutover.** If `nodejs26.x` isn't available, deploy notes as a Lambda container image (or custom runtime) with Node 26. `engines >=26` stays (D10). | verify |
| U11 | **Sans-IO core interface.** | Rust: `Machine::step(Input) -> Step { io: Vec<IoRequest{id, op: Get\|PutIfAbsent\|List\|Delete, key, body}>, done: Option<Outcome> }`. TS executes requests concurrently (bounded) and feeds `IoResponse{id, ...}` back. Rust owns the protocol and can be simulation-tested deterministically with a fake store and fault injection (dropped responses, 409/412/5xx, delays). TS owns credentials, retries of *idempotent* reads, timeouts and interruption. | **accepted** |
| U12 | **Log growth.** v1 never deletes `log/` keys. | Express storage is $0.11/GB-month, negligible at personal scale. Later GC overwrites folded entries with `Compacted{ckpt}` stubs; never a delete or lifecycle rule (SlateDB's re-create hole). | deferred |
| U13 | **Owner actions needed for S3 lanes** (before the G4 S3 lanes PR). | Provision one Express directory bucket and one Standard bucket in one region, plus a GitHub OIDC role scoped to `ci/` prefixes with a 1-day lifecycle on Standard `ci/`. Express gets no lifecycle on `log/` in prod, but the `ci/` prefix may expire. | owner |
| U14 | **Things to verify against upstream** before the TS cutover. | napi-rs `_tag` discriminant codegen; Effect 4.0.2 `RcMap` capacity/idle semantics; an interrupt probe for the 4.0.1/4.0.2 finalizer changes; the AWS SDK v3 handling Express `CreateSession` automatically; pnpm 12 strict workspace keys. | verify |

## 3. Workstreams

### A. Correctness fixes (first; small; independent)

- [ ] **`query!` string literals don't compile.** The macro emits `Box::from(s.as_bytes())` into a `Box<str>` field (`bumbledb-query-macros/src/lib.rs:1629`). Fix: `Box::<str>::from(text)`, plus a test. The underlying cause is string codegen; it goes away with quote (C14).
- [ ] **The TS bridge matches error arms that can never fire:** `Error::SchemaMismatch`, `DestinationExists` and `EnvironmentLocked` (`ts/crate/src/runtime_wire.rs:738-754`; also `bench lanes/curves.rs:563`). Fingerprint-mismatch, destination-exists and busy refusals never surface as typed errors. *(R2: dead variants allow this bug.)*
- [ ] **Incomplete offending-row citations.** The pointwise key sweep compares each interval with the previous one only; it should track `prev_end = max(prev_end, end)` (`schema/judge.rs:1081-1086`).
- [ ] **`gather_words` returns 0 on an out-of-bounds index in release builds** (`exec/kernel/gather.rs:30-33`). Also `colt/gather.rs:178-214`: `get_unchecked` guarded only by `debug_assert`. Fix with real asserts or index types that carry the proof (R3).
- [ ] **The plan validator `.expect` panics in release** (`api/prepared/build.rs:739`). Return an `Error` instead.
- [ ] **Log: local create is 3 commits, and a crash between them wedges the directory** (confirmed). Hosted create has the same problem. Superseded by the log rebuild (staged atomic cache install), but if the old log lives longer, fix it.
- [ ] **Log: a local commit error is reported as `NotSubmitted`** (`writer/local.rs:156-167`). It must be `OutcomeUnknown`. Superseded by the rebuild.
- [ ] **Hosted tail policy is unbounded by default and nothing checkpoints automatically**, so cold opens replay all of history. Superseded by the rebuild (automatic checkpoints).
- [ ] **One lockfile.** Make `ts/crate` a workspace member. Today the addon resolves dependencies separately: blake3 1.8.5 vs 1.8.7, and ~55 other differences.
- [ ] **Repair ~19 `// SAFETY:` comments truncated by the earlier comment purge.**

### B. Security and lock refresh (on the current nightly, before anything else)

- rustls 0.23.43 → 0.23.45 (RUSTSEC-2026-0285), in both locks.
- napi 3.10.5 → 3.14.2: GHSA fixes in 3.12.4, and `External` provenance validation in 3.13. ts/crate uses `External<T>` about 169 times.
- blake3 floors → 1.8.7 (it removes `arrayref`, RUSTSEC-2026-0260 compromised-owner risk).
- uuid 1.27, tokio 1.53.2, napi-derive 3.6.12, object_store 0.14.2 with `default-features=false`, and make `futures` optional.

### C. Engine (target design done; PR order below)

The target is about −19 to −23k lines including C17, plus whatever dead code rustc finds once the public surface shrinks.

| # | PR | Δ prod / test | Depends on | Rule |
|---|---|---|---|---|
| C1 | **Dead code.** Never-constructed `Error` variants (`FormatMismatch`, `SchemaMismatch`, `AlreadyInitialized`, `DestinationExists`, `PublishedButUnsynced`, `EnvironmentLocked`, `CommitSync`, `Hatch`); aliases (`ReadInstance`, `ApplicationChanges`, `StoreFinding`); dead doc(hidden) verbs; legacy `determinant_candidates`; tests for the transitional store. Fix the TS arms to match the real errors. | −700 / −250 | — | R2 |
| C2 | **Comment purge.** Process IDs (chapter N, P\d\d, ENG-, HASH-, C0–C9, D01–D29…), tombstones, "successor/transitional", truncated docs. | −1k comment lines | — | hygiene |
| C3 | **Delete the cursor fallback evaluator.** Resident overflow becomes `Error::Capacity`. | −1.5k / −0.8k | — | R2, D8 |
| C4 | **Delete the engine's LMDB scratch tier and spills.** The judge's grouped maps become RAM-only. The log's two external-sort uses become a log-owned `TempMap`, or disappear in the rebuild. | −4k / −2.5k | C3 | D8 |
| C5 | **Delete the FactLayout codec.** Closed relations store canonical rows and build images through the same `canon::row_words` path. This removes 13 `unsafe` blocks. | −1k / −0.5k | — | R4: closed relations stop being special |
| C6 | **Cancellation.** Plain slice ops instead of 4 KiB-chunked memcmp; `sort_unstable_by` instead of the hand-written heapsort; delete `work/clock.rs` (mach FFI) and use `Instant`. | −400 / −150 | — | R7 |
| C7 | **Visibility cutover.** `store`, `schema::judge` and `work::Scratch*` become `pub(crate)`. `integration` becomes `bumbledb::host`. One `testing` feature replaces `collision-probe` and `ground-off`. Deny `unreachable_pub`. Delete what `dead_code` then flags. | −2 to −4k / −1.5k | C1, C3, C4 | R7 |
| C8 | **One `Error`.** It absorbs `StoreError`, `IntegrationError`, `JudgeError<E>`, `ScratchFault` and the 3× `WorkError` wrapping. `ErrorFamily` and its descriptor table become `Error::kind()`. Fold `ValidationError` from 59 variants to ~35. | −1.2k | C7 | R2 |
| C9 | **One `WriteOutcome<R> { Committed{value,generation,changed} \| Rejected(Violations) \| Moved{witnessed,current} }`.** It replaces `ApplyOutcome`, `ConditionalWrite`, `Admission<Committed>` and `CoreCommit`. One private commit path instead of 3 copies. Delete `RowIndexer`, `CandidateJudge`, `store::Judgment`, `LawfulParent` and `UnindexedRows`. | −700 / −600 | C8 | R2, R7 |
| C10 | **Judge.** One `Facts` trait returning `Result<ControlFlow>`, which kills the 23 `walk_error` tunnels. Two entry points instead of 5. Split into key/containment/capacity files. Index the `ChangeSet` by relation once instead of rescanning it per call. | −600 | C9 | R5 |
| C11 | **Prepared queries.** `EitherSink` implements `Sink` by delegation, removing 6 match sites. One shared Free Join rule runner. One `i64_word` instead of 4 copies. Group `PreparedQuery`'s ~25 fields into Program / Bound / Runtime. | −800 | C3, C4 | R5 |
| C12 | **Executor context.** `JoinCtx` + `JoinState` structs instead of 13-argument functions, which deletes most `too_many_arguments` expects. Keep `exec/kernel` behind a narrow lane API as the fearless_simd seam. | −300 | C11 | R1 |
| C13 | **Performance.** Reuse key-probe buffers; batch residual compares through the kernels; give `ImageCache` a byte cap or eviction for 512 MB devices. | ±200 | C12 | — |
| C14 | **One proc-macro crate.** proc-macro2 + quote (no syn), spanned `compile_error!` instead of 34+ panics. Delete the `bumbledb-query` crate and move its tests. `query!` resolves relations through types that `schema!` emits, not a duplicated `screaming_snake` naming convention. | −1.5k | — | R3, R5 |
| C15 | **Move pure schema validation into `bumbledb-theory`.** `schema!` then emits compile errors, so bad schemas fail at build time. | ±0 (−300 macro checks) | C5, C8, C14 | R3: parse once, at the earliest boundary |
| C16 | **On-disk layout v1, maximal (D12).** One LMDB environment; LMDB's **default comparator** so stock `mdb_*` tools work; **fixed-width keys** — `rows: [rel u16][home 16B][ordinal u64] → canonical row` where home is the exact scalar if one exists, otherwise the 16-byte row fingerprint. That folds the membership index away; every relation is probed the same way. `det: [proj u16][routing ≤16B][ordinal] → home` stays: it is essential, because uniqueness is judged, not keyed. `meta`: magic + `layout=1`, schema fingerprint, `{databaseId, seq, revision}` (identity from log Genesis; **no path-minted store id**), `next_row_id`, per-relation `(count, version)` merged into one key, host records (receipts `r‖id`, migrations `m‖i`) and the head attachment. Delete `PhysicalComparator`, `DataTree`, `KeyLayout` width derivation, `PhysicalKeyWidths`, and the membership tag and its corruption checks. Deterministic `Snapshot::content_digest()` verifies checkpoint images (U3). Fingerprint label and every wire tag are renumbered densely from 1. | −900 / −300 | C9 | R4: homeless rows and path identity stop being special |
| C17 | **Fixed virtual map (D16).** Set once at open; `Error::Full{ceiling}`; delete grow/retry/drain-for-resize, `map.rs` and 3 error variants; the gate shrinks to close-drain only. | −1k / −0.5k | C16 | R4: growth stops being a special case |

C1, C2, C3, C5, C6 and C14 are mutually independent.

### D. bumbledb-log rebuild: S3-first, dead simple (target design done; reconcile with Q1/Q2)

**Proposed protocol.** The log *is* the database (R4, R6):

- **Commits.** Each commit is one immutable object `log/{seq:020}`, written with `PUT If-None-Match:*`. The key position is the chain, so there is no hash chain, mutable HEAD, ETag CAS, epoch, GC barrier or tail policy.
- **Checkpoints.** Automatic, immutable compacted LMDB images `ckpt/{inv(seq)}` (`LIST max-keys=1` returns the newest). The local LMDB directory is a disposable cache.
- **Entries.** `Genesis | Commands | Freeze | Migration | Thaw` (control flow reified as data). `Outcome = Committed | NoChange | PreconditionFailed | InvariantRejected{evidence}`. The precondition becomes `ExactRevision(u64)`.
- **Submit.** Catch up → check for a receipt → native `decide` → `PUT log/{t+1}`:
  - Created → `apply` → decided.
  - 412 → PUT the batch at the next slot at once, with the commands queued behind it, and GET the winner. An entry that lands past the head it was judged at is judged again where it lands.
  - Ambiguous → re-PUT the identical bytes.

  Byte-compare ownership makes SDK retries safe and makes `not-submitted` *provable*.
- **Round trips.**

  | Operation | Today | Proposed |
  |---|---|---|
  | Submit | 3 serial RTs + fsync, with the LMDB write txn held across 2 RTs | 1 RT |
  | `latest` read | — | 1 RT (`GET log/{tip+1}` → 404) |
  | Cold open | 3 + C + T serial RTs plus whole-DB re-judgment | ~3–4 RTs + image download, no re-judgment |
- **Local mode** is the same protocol over FsStore (write temp, fsync, `link()`). That deletes `LocalHistory` and every `*_local`/`*_hosted` fork (R4).
- **Migrations (D6).**
  - Authoring: `bumbledb generate` writes `migrations/NNNN_name/{schema.json, schema.ts, migration.ts}` and a bundled `index.ts`. `copyUnchanged` carries over unchanged relations, so additive changes need no code.
  - Apply: `open()` compares the applied migration list against the bundled one and runs `migrate()`, or `bumbledb migrate` runs it as a deploy step.
  - Rolling deploys:
    - **Optimistic:** `PUT Migration` at tip+1.
    - **Contended:** `Freeze` → migrate the frozen state → `Migration`.
    - **Failure:** `Thaw` + `MigrationRejected(violations)`.
    - **Old code** gets `SchemaAdvanced`.
    - **Takeover** by another new-code process is safe without leases, because `If-None-Match` picks one winner.
  - Applied migrations are recorded as entries plus `m/{i}` rows (the `__drizzle_migrations` analogue).
- **Placement (proposed, Q2).** I/O lives in TS behind a 4-verb `ObjectStore` (GET, PUT-if-absent, LIST, DELETE): MemStore, FsStore, and S3Store over the app's own S3Client. Rust is pure CPU + LMDB (~1.8k lines; no tokio, object_store or credential code; no `once_cell_try`).
- **Size:** ~57k lines today → ~7.5k including tests.
- **PR order:**
  1. Purge dead code: `ts/crate/src/log.rs`, identities, `transition/hosted.rs`, `duty`, erase, local_roots, certainty, `gate-baseline-ports`. −8k.
  2. `schema_file` and bindings move to `ts/crate` on serde_json; delete `json.rs`.
  3. Merge ts-log into ts as `./hosted`.
  4. Test that a `Db::compact` image is portable across platforms.
  5. New Rust log core alongside the old one.
  6. Bridge + TS protocol.
  7. Migrations + CLI.
  8. Switch the notes example over.
  9. Delete the old machine (~−45k).
  10. Docs.
- **Known limit:** `log/` objects are never deleted in v1. A stale writer could otherwise recreate a deleted key, so safe compaction needs a fence and is deferred.

**Research verdict.** The full report is [reports/Serverless S3 design for bumbledb.md](../reports/Serverless%20S3%20design%20for%20bumbledb.md). The create-only `log/{seq}` skeleton is right. It is the protocol Delta, Graft and SlateDB converged on, and SlateDB explicitly rejected a HEAD pointer. Five representation additions finish it:

| Pri | Change | Why |
|---|---|---|
| P0 | **A per-submission nonce in every entry.** Every non-200 (412, 409, timeout, 5xx) is resolved by reading the object back and comparing. If still ambiguous, re-PUT identical bytes. | Without it, two clients submitting identical commands both read back "ours". It also makes hedged PUTs safe. |
| P0 | **Writer state `Idle{tip} \| InFlight{tip, entry}`.** | Two in-flight PUTs become unrepresentable. Without a hash chain, a pipelined t+2 could land on someone else's t+1. |
| P0 | **Entries carry the decided ChangeSet.** | Catch-up applies effects and never re-judges. Logical, not page shipping, which was Turso's lesson. |
| P0 | **Never delete `log/` keys; no lifecycle rules on them.** A future GC overwrites folded entries with a `Compacted{ckpt}` stub. | Closes SlateDB's stale-writer re-create hole. |
| P1 | **`log/` on an S3 Express directory bucket, `ckpt/` on an S3 Standard bucket.** LIST is used only on Standard. | ~7 ms commits; a `LIST max-keys=1` that works; checkpoints that survive the loss of one availability zone. |
| P1 | **Warm submit = decide on a read snapshot, then one PUT.** No pre-PUT GET; apply in a short write txn; never fsync the cache. | 1 round trip instead of 3 + fsync. No network I/O under the LMDB lock. |
| P1 | **Group commit without a timer.** Queue commands while a PUT is in flight and flush them as one `Commands` entry when it returns. | Zero added latency when idle. |
| P1 | **Automatic checkpoints** every N entries, or when log bytes reach the image size. Any cold opener that replays more than N entries writes one. Images are verified by digest, not re-judgment. | Bounded cold start. |
| P1 | **Cold open in parallel:** CreateSession, LIST, ranged image GETs and 32-wide tail waves. | Estimated 100–200 ms for a ~10 MB database. |
| P1 | **Commit returns `seq` as a bookmark.** A strong read costs one `GET log/{tip+1}`. Poll with GET, never LIST (LIST is billed as a write). | Read-your-writes; the bill rounds to cents. |
| P2 | **Migration ledger keyed by name + verified hash**, with a 4-way comparison. Optimistic `Migration` entry by default; `Freeze` carries a deadline; rejection is sticky. `bumbledb migrate` runs as a deploy step. | Atomic and lock-free; it can't wedge and it fails loudly. Better than Drizzle, which has no lock and can skip migrations. |
| P3 | **Sans-IO Rust core** that emits I/O requests as data (Turso's `IOResult` pattern), executed by TS/Effect with the app's S3 client. | Deterministic simulation tests in Rust; the SDK handles Express sessions and credentials. |
| Defer | Express append segments, RenameObject, a 3-bucket Express quorum, a write broker, lazy page fetch. | Each adds a second representation or a service before there is evidence it is needed. |

**Open risks:**
- Express p99 rests on one microbenchmark; measure it in a CI lane.
- Express is single-AZ, so frequent Standard checkpoints are needed.
- Many hot writers will lose races repeatedly; this is fine at personal scale.
- LMDB image portability from macOS to Linux is unproven; test it first.
- Freeze deadlines depend on wall clocks.
- The log grows forever, so stub GC will be needed later.

**Research inputs** (`research_notes/Serverless S3 design for bumbledb/`, 5 notes):

- **Turso Cloud acknowledges a commit only after its WAL is in S3 Express One Zone.**
  - Uploads are batched across tenants, with the batching wait capped at 10–100 ms depending on plan.
  - A 4 KB Express upload averages 6.4 ms (p99 7 ms), vs 31 ms average / 102 ms p99 on S3 Standard.
  - Frequent checkpoints go to S3 Standard as 128 KB segments.
  - Local NVMe is only a cache, and the database is never cold.
  - Turso's lesson: ship **logical** changes, not pages. Page shipping cost 4 KB per tiny write and caused replica divergence. LMDB's copy-on-write pages would be worse.
  - Supabase announced it is acquiring Turso on 2026-10-02.
- **Durable Objects** commit locally, then hold replies behind an *output gate* until 3 of 5 followers ack. Object storage is asynchronous.
- **Single-writer pattern** (turbopuffer, WarpStream, S2, SlateDB): one writer, batching whatever arrived while the previous PUT was in flight. Each batch is one `If-None-Match` object, which is both the commit point and the fence. Each object carries a txn id so a 412 can be resolved by reading the object back.
  - Expected latency: ~100–200 ms per commit on S3 Standard, ~20–40 ms on an S3 Express quorum, under 10 ms only with your own follower tier.
- **S3 Express caveats for this design.**
  - ListObjectsV2 on directory buckets is **not lexicographic**, so `LIST start-after` / `LIST max-keys=1` tricks break. The protocol must discover the tip by probing `GET log/{tip+1}`, and find checkpoints through a known key, not LIST order.
  - Express does have append with an offset CAS, RenameObject with a `client-token`, and conditional DELETE.
  - Pricing: Express is $1.13 per million PUTs vs $5 on Standard.
  - LIST is billed as a write, so poll with GET/HEAD, never LIST.
- **Hedged conditional PUTs** make a 412 ambiguous: the earlier attempt may have won. The design already resolves ownership by reading the object back and comparing bytes.
- **Migrations:**
  - Drizzle v0 decides what to run by `created_at` only, which can skip migrations (issue #5769), and it never checks hashes.
  - Neither Drizzle line locks against concurrent runs.
  - Prisma, Rails and Ecto take a database lock.
  - The log design's Freeze/Migration entries with `If-None-Match` take the place of that lock.

### E. Floats and SIMD (target design done; owner decision D7)

| # | PR | Δ | Gate |
|---|---|---|---|
| E1 | **`gather_words` out-of-bounds.** Accumulate `bad \|= addr >= len` per lane, then assert after the loop. A wrong value can never escape into SUM/MIN. | +20/−15 | kernel tests, Miri |
| E2 | **`bumbledb-bench micro --levels all`.** Timing rigs leave `cargo test`. Margins are kernel/scalar-twin ratios within one run, so host drift doesn't matter. JSON goes to `docs/perf/runs/`. **New `float_stats` family:** 1M rows with 1% NaN; SUM/AVG/MIN/MAX, grouped over 10k/1M keys, `v > c`, computed `qty*price`. No bench family uses F64 today. | +550 | timing-free `cargo test`; baseline committed |
| E3 | **Replace the FP guard with a read-only check (Q4).** The current install-asm / compute-asm / restore-in-Drop sequence is *itself UB* per Rust/LLVM (`_mm_setcsr` docs, LangRef §floatenv, Ralf Jung 2026-03). On a default host it is a no-op. **Instead:** read FPCR/MXCSR once per query that has F64 arithmetic, and refuse with `NonDefaultFloatEnvironment` if it isn't default. Arithmetic becomes plain `f64` ops, which are bit-identical under the default environment because NaN is canonicalized. | +80/−380; 6 asm + 3 unsafe → 1 read-only asm per arch | float fixtures, a 10⁷-pair differential against the old asm |
| E4a | **fearless_simd foundation.** `level()` is cached in a `OnceLock`. Dispatch happens **per kernel call (per batch)** and the executor is not generic over `S`, so x86 doesn't get 5 copies of the engine. Bodies use fixed-width `u64x4`/`f64x4`, never native-width types, so chunking, tails and bitmasks are identical across levels. Port filter/fold/gather and drop `portable_simd`, leaving **zero feature gates** in bumbledb. `force_support_fallback` is a dev-dependency only. Miri → `Level::fallback()`. Never use non-precise float min/max/reduce or `mul_add`. | +300/−250 | **every level** bit-identical to the scalar twins (Fallback, Sse2, Sse4_2, Avx2, Avx512 downgrades / Neon); Neon within ±3% of today |
| E4b | **x86 Allen.** A portable branch-free fearless kernel replaces the scalar decision tree: 6-bit signature → injective 4-bit hash → nibble-packed table lookup. Exhaustively checked. **The NEON kernel and prefetch stay as is** and are chosen via `level.as_neon()`. Also: `compact` uses fearless `compress` on Avx2+ only; NEON has no compress instruction, and scalar measures 1.00 cycle/item on M2. | +220/−60 | exhaustive Allen differential, micro |
| E5 | **NaN semantics (Q3).** Recommended option **C**, IEEE/JS-style: order predicates never match NaN; MIN and MAX propagate NaN; equality stays "one NaN, one zero, NaN = NaN" (JS `Set` semantics). Implementation: clamp F64 ranges to `[0, KEY_POS_INF]` at lowering and in planner folding; flag var-var residuals; MIN maps the NaN key to 0 (never a canonical key) and back. No kernel or encoding changes. | +250/−30 | fold differential, querygen oracle |
| E6 | **Exact SUM/AVG via xsum.** Neal's small superaccumulator: 67 `i64` chunks, lazy carries, about 20 ops per value instead of 200+, with no 272-byte temporary. Output reuses today's `Finite::round` verbatim, so results are **bit-identical**. Batch reducer with 4 lane-private accumulators; float folds get a column-scan path. The old 34-limb code becomes the test oracle. Per-group memory goes from 288 to 552 B; the fallback is a 48-bit-digit variant at 368 B. | +450/−250 | limb-oracle differential, slow oracle, Miri |
| E7 | **Columnar computed outputs.** `ScalarExpr` → a postfix register program over 64-lane `[u64;64]` columns, one `dispatch!` per batch. Canonicalize after every float op (`1/(0*-1)` must stay +Inf). Integer lanes use checked semantics with the first error tracked in program order, which reproduces the scalar evaluator's error exactly. The old recursive evaluator becomes the oracle. | +700/−150 | random-tree differential at every level, error identity |
| E8 | **Exact int↔float comparison, var-const only.** A planner rewrite with integer-only arithmetic turns `x < 2.5` on an int column into `x <= 2` exactly, at zero runtime cost. Var-var mixed comparisons stay refused, with a diagnostic pointing at `toF64Exact`. | +250 | boundary goldens (2⁵³±1, ±Inf, NaN) |

Targets: x86 Avx2 ≥1.5× Sse2 on range and min/max filters; xsum push ≥10× and end-to-end SUM/AVG ≥3×; computed outputs ≥3×.

### F. TS SDK and bridge (target design done)

**Package.** One npm package, `@bjornpagen/bumbledb`. There is already only one addon; ts-log is folded in.

| Entry | Contents |
|---|---|
| `"."` | Authoring (loads the addon on first schema/query definition) plus the runtime: `Bumble` layer, `Database` (S3, directory or memory store), `Migration`, errors |
| `"./engine"` | Raw embedded `Db` for benches, tests and single-process use |
| bin `bumbledb` | `generate`, `check`, `migrate` |

- Keep the 3 platform packages.
- Drop `@effect/platform-node` as a runtime dependency (`bin.ts` uses `Effect.runPromiseExit`).
- The crate moves to `crates/bumbledb-node` as a workspace member, so there is one lockfile.

**Bridge (R3, parse once).** Split by data class:

1. **Rust→JS outputs** become generated `#[napi(object)]` structs and discriminated enums, with one discriminant (`_tag`, PascalCase).
   - The public outcome unions *are* the wire types, which deletes the `outcomeOf`/`decodeSubmit`-style mappers.
   - Counters are `u64`/`i64n`, never lossy `i64`.
   - `binding.d.ts` is committed and gated by `git diff --exit-code`.
   - Deletes: ~420 hand-written `.set()` calls; `native.ts`/`runtime-native.ts`/`db-native.ts`/`ts-log native.ts` (~1.6k lines); `wire-tags.test.ts`; most of `tags.rs`.
2. **Tree-shaped cold inputs** become one JSON string decoded by serde (`tag="kind"`, `deny_unknown_fields`): query IR, SchemaSpec, options.
   - Exact-field rejection, the 128 depth limit and lone-surrogate refusal all come from serde/serde_json.
   - `serde_path_to_error` turns today's discarded `InvalidArgument` into `{path, expected}`.
   - Value encoding: u64/i64 as decimal strings, f64 as bit-hex (keeps −0 and ±∞ exact), bytes as hex.
   - TS input types come from ts-rs, or a TS-emitted IR corpus parsed in a Rust test.
3. **The data plane stays hand-walked** because it is schema-dependent, performance-critical and already strict: `schema_value_in`, `key_row`, `params_in`, `ValueOut`, the binary change codec.
4. **Keep** External handles, the payload-less TSFN per operation, and the custom executor plus cancellation. The design is sound. napi `AsyncTask`+`AbortSignal` cannot join a drain.

**Query validation (D17: one validator).** Schema and query construction call the addon **synchronously** (`descriptor`/`validateQuery` napi calls) and return branded validated handles. The engine's `ir/validate` is the only semantic validator. Its structured `ValidationError` crosses as a diagnostic, and TS maps the rule/atom/find indices back to authoring names. **Delete** the TS checks that mirror engine semantics (the part of the 78 `AuthoringError` sites in `query/lower.ts` that duplicates `ir/validate`, e.g. `:1124-1130`), `parseQueryIr` on self-produced IR (`:2371-2375`), and `scripts/packed-pure-authoring.ts`. Keep only checks that are TS-level by nature, such as lowering-shape errors. `parse-ir.ts` stays only for `queryFromDescription` on external data, if that API survives. Otherwise delete it too.

**Effect 4.0.2 shape.**
- One `native/op.ts` (~120 lines: `call`, `drain`, `scoped`) replaces 2 lease bridges, 3 drain helpers and the interrupt stash.
  - The stash goes only behind a probe test. The interrupt fixes that matter are 4.0.1 #8652 and 4.0.2 #8779; #8585 does not cover this case.
  - On a late completion after abort, use bridge.ts semantics (no second cancel).
- **Scope-only resources.** No `close()` methods, no `state.closed` flags.
- `#private` handle fields with `#h in x` brand checks instead of 8 WeakMap side tables (R2).
- One error mechanism: `Schema.TaggedError` with `DbError{operation, reason}`. Delete `NativeReportedError`/`errorFromThrow`.
- `Effect.fn` spans on every public verb; `Schema.brand` identities.
- **Hosted developer experience:** `Database.layer({ schema, migrations, store: S3.store({bucket, prefix}), cache: {directory: "/tmp/bumbledb"}, onOpen: "migrate" | "verify" })`.
  - Build it during Lambda init.
  - `submit(changes, { requestId, precondition })` returns a `SubmitOutcome` union.
  - Consistency: `"latest" | "cached" | { atLeast: stamp }`.
  - `Database.pool(...)` replaces TenantCache.

**Dev loop.**
- Tests run against `dist` today, which is why every change needs a family build.
- Codemod `#x.ts` imports to relative imports and delete the `bumbledb-src` condition and `declarations.ts`. Tests then run on `src` under Node 26 type stripping.
- `napi build --platform --dts` handles dev builds. Release builds become a separate command (`build.ts:26-31` goes).

**Delete:**
- ts/scripts: absence-gate, pin, declarations, platform, native-artifact.
- All of ts-log/scripts.
- `build-family`, `release-ready`, `release-results`, `.config/obligation-inventory.json`, `measure-cutover`.
- Collapse the packed-* scripts into one ~100-line packed smoke test. `packed-pure-authoring` is deleted (D17).

**Size:** TS + scripts −6 to −7k lines; Rust bridge −3k ±1k.

**PR order:**
1. Version bumps.
2. Crate becomes a workspace member.
3. Dev loop.
4. Merge ts-log in.
5. `native/op.ts` plus the interrupt probe.
6. Scope-only resources, `#private`, errors, `Effect.fn`.
7. Generated outputs: db first, log after D lands.
8. serde JSON inputs and structured diagnostics.
9. Database/Migration developer experience (needs D).
10. Delete release bureaucracy.

### G. CI, tests and bench (target design done)

**Workflows.**

| Workflow | Runs | Contents |
|---|---|---|
| `ci.yml` (PR + main) | — | **lint:** once on ubuntu — fmt, 2 clippy configs (down from 5), rustdoc, cargo-deny, cargo-shear, biome/tsc, `bench --no-run`. **test:** macos-26 / ubuntu-24.04 / ubuntu-24.04-arm — Swatinem/rust-cache, nextest via `taiki-e/install-action`, `--cargo-profile ci` (opt-level=1, no LTO), doctests, a tree-clean check. **addon:** linux-x64 on PRs, all 3 on main. |
| `deep.yml` | nightly | Miri via `cargo miri nextest` on native linux + mac (deletes `miri-cross-cc.sh` and miri.sh's hand-written filters; uses `cfg_attr(miri, ignore)`), musl, `BUMBLEDB_DEEP` sweeps, release alloc gates, macOS clippy, udeps, the S3 vendor matrix |
| `toolchain-canary.yml` | weekly | See the toolchain bump below |
| `release.yml` | `v*` tag | `needs:` on ci + musl + s3-aws, an `environment: release` approval, npm trusted publishing over OIDC |

- **No git-snapshot re-exec.** Checkout gives the exact tree; add `--locked`/`--frozen-lockfile` and a final `git diff --exit-code`.
- Estimated PR critical path: **27.5 → ~12 min**. Runner-minutes per PR: ~89 → ~45.

**S3 lanes (D4).**
- **Every PR, every platform:** an in-process s3s server whose `put_object` is serialized behind a mutex. Unwrapped s3s-fs checks the condition and then writes without a lock, so racing creates can both "win".
- **Linux PRs:** a SeaweedFS `weed mini` service container. It has a real per-key lock, and losers get 412.
- **main, nightly and release:** AWS S3 over OIDC, with a 1-day lifecycle on `ci/`.
- **Weekly:** R2, Tigris and RustFS cells.
- **One suite**, `s3_wire.rs`, selected by `BUMBLEDB_S3_TARGET`. Its first test is a contract probe (32 racing creates, at most 1 win).
- **Fix the vacuous smoke test.** `s3_smoke.rs`'s `require_credentials!` returns early, so all 5 of its tests "pass" without credentials (R2).
- **Rejected:** MinIO (community repo archived 2026-04), LocalStack (needs an auth token since 2026-03), Garage (no conditional writes).

**Test pyramid.**

| Tier | When | Contents |
|---|---|---|
| T0 | lint, once | static checks |
| T1 | every PR, 3 platforms | unit, merged integration binaries (`tests/it/main.rs` per crate), sharded property/adversarial tests, differential vs naive and SQLite, conformance replay, compile-fail, in-process S3, crash tests, alloc `<= budget` |
| T2 | PR, linux | SeaweedFS + alloc gate + addon |
| T3 | main | fat-LTO addons + AWS |
| T4 | nightly | deep lanes |
| T5 | manual, M2 Max | microbench report, app-perf |

- **One allocation counter.** `alloc_counter::CountingAllocator` registered under `#[cfg(test)]`; the `alloc-counter` feature is deleted from all crates.
  - Delete `alloc_census.rs` (1,244 lines).
  - One `alloc_budget!` macro. `== 0` gates stay. Nonzero exact equalities become structural assertions.
- **`#[ignore]` goes from 31 to 1:**
  - 17 timing pins → nightly `#[bench]` reports that never assert.
  - Regenerators → `BUMBLEDB_BLESS=1`.
  - Subprocess entry points → env-guarded children.
  - Demos and the census → cut.
- **One compile-fail runner** (~150 lines instead of 743) that discovers artifacts through cargo's `--message-format=json`, not the build-dir layout. `trybuild` is rejected because its full-stderr snapshots churn on every nightly.
- **Doctests** via `#[doc = include_str!("docs/cookbook.md")]` delete `cookbook.rs` (2,481) and `readme.rs` (240). Each cookbook recipe becomes one self-contained fence.

**Bench.**
- Cut ~3.8k lines.
- Move `closure/history_model` into the log tests; the log then stops depending on the whole bench crate.
- `stress.rs` moves to core.
- Layout becomes `oracle/` (naive, differential, querygen, sqlite, conformance), `worlds/` and `harness/`.
- Seeded conformance fixtures are generated in-test from seeds, with only digests checked in (−20 MB). The naive evaluator gets a deterministic step budget, so the roster no longer depends on machine speed.

**Toolchain bump.**
- `rust-toolchain.toml` is the single source of truth. Bench provenance comes from `rustc -vV` in a build.rs. Trim components to rustfmt + clippy.
- `scripts/bump-toolchain.sh` steps:
  1. Pick the newest qualifying nightly from the channel manifest.
  2. Rewrite the channel.
  3. `fmt` + `clippy --fix`.
  4. Delete unfulfilled `#[expect]`s.
  5. Run the gates.
  6. Produce an old-vs-new microbench diff report (never blocking).
  7. Commit.
- Target cost: ~5 files per bump, down from 111.
- A weekly canary opens or updates a `bot/toolchain` PR, or files an issue.

**Hygiene.** `[workspace.dependencies]`, Dependabot (grouped), cargo-deny + cargo-shear in lint, udeps nightly.

**PR order:**
1. CI skeleton.
2. Remove reruns; one allocator.
3. Profiles, sharding, merged integration binaries.
4. S3 lanes.
5. One lockfile + deny/shear.
6. Compile-fail runner.
7. Doctests.
8. Bench restructure.
9. Timing pins → `#[bench]`.
10. Miri via nextest.
11. Toolchain script + canary.
12. release.yml.
13. Delete orphan scripts.

### H. Toolchain and dependencies (plan done)

Order:

1. **Shrink first:** relax the noisy pedantic lints workspace-wide (−~300 `#[expect]`) and do the C/D/G deletions.
2. **Lock refresh on the old nightly** (B).
3. **Nightly bump alone** to nightly-2026-10-09. Main risk: the next-gen trait solver (45 `-> impl`, 12 GATs, 116 `dyn Fn`, proc-macro impls); build and triage. Expected fallout:
   - `!`/`Infallible`: no break.
   - New clippy lints: about 0 hits; `map_unwrap_or` broadening, up to 7 fixes.
   - The `try_blocks` comment is stale; delete it or use the feature.
   - Re-earn timing and asm once, combined with the fearless_simd port.
4. **rusqlite 0.32 → 0.40** (bench only):
   - u64 `FromSql` now needs `fallible_uint` (corpus.rs:136/153, calendar/corpus.rs:140).
   - `progress_handler` returns `Result` (curves.rs:270/282).
   - `execute` rejects statement tails.
5. **TS batch:** pnpm 12.10 (strict workspace keys; Corepack gone), effect 4.0.2, biome 2.5.15, @types/node 26.6, arkregex 0.0.13.
6. **notes example:** next 16.4, react 19.3, alchemy beta.81 (regenerate its patch), mongodb **6.21** (7.x breaks alchemy's peer range), client-s3 3.1148.
7. **CI:**

   | Item | From | To |
   |---|---|---|
   | checkout | v5 | v7 |
   | cache | v5 | v6 |
   | upload-artifact | v4 | v7 |
   | setup-node | v5 | v7 |
   | pnpm/action-setup | v4 | v6.1 |
   | docker setup-buildx | v3 | v4 |
   | docker build-push | v6 | v7 |
   | nextest | 0.9.143 | 0.9.148 |
   | alpine | 3.23 | 3.24.2 |

   Also add Node 26.
8. **Separate workstreams:** fearless_simd (E); the optional hand-rolled S3 client (Q2); an upstream PR making `doxygen-rs` optional in lmdb-master-sys, which drops phf/rand from the build.

**Local Node 26 (D10).** MacPorts `nodejs26` conflicts with `nodejs24`. pnpm's node dependency is path-based, so it is satisfied by nodejs26:

```bash
sudo port -f deactivate nodejs24 && sudo port install nodejs26
```

CI moves off the AL2023 `dnf nodejs24` package to setup-node v7 with Node 26 (glibc 2.34 baseline unchanged), and Corepack becomes pnpm/action-setup v6.1.

### L. Legacy cull (D1): delete every trace of prior formats and eras

**Rule:**
- The code knows exactly one format per artifact: the current one.
- A file that isn't the current format is refused by a single magic/version equality check, which returns `NotABumbleDb { path }`.
- No code names, detects, refuses specifically, migrates or tests anything older.
- Comments and identifiers that narrate history go too ("successor", "retired", "transitional", "0.x", "braid", "sidecar", "tombstone of", "chapter N", ticket IDs).

**Inventory (grep, 2026-10-09):**
- **Files with markers:**

  | Marker | Files |
  |---|---|
  | `successor` | 105 |
  | `retired` | 80 |
  | `tombstone` | 38 |
  | `sidecar` | 21 |
  | `braid` | 18 |
  | `0.x` | 16 |
  | back-compat | 12 |
  | `legacy` | 10 |
  | `transitional` | 7 |

- **Code, not just comments:**
  - **log, "retired migration records" detection/refusal:** `codec::has_retired_history` (`recovery.rs:289-290`, `checkpointer.rs:185,228-229`), the `m`-prefix refusals (`codec.rs:115,281,436,508-525`), the test `transition_local.rs:261-292`.
  - **log, other legacy code:** receipt-epoch retirement (`certainty.rs:114-153`, `retired_through`/`retired_filter` in recovery/checkpointer); `conformance_v3.rs` "no retired 0.x families" test; `conformance/v3/{identities,machine-constants}.json`; braid/sidecar remnants in `replica.rs`, `store.rs`, `inspect.rs`, `writer/mod.rs`, `history/admission.rs`, `store/fs.rs`, `bin/duty.rs`, `identities.rs`.
  - **engine:** compat aliases `ReadInstance = ReadFrame`, `ApplicationChanges`, `StoreFinding`; old-family/old-layout lifecycle tests (`storage/store/tests/lifecycle.rs:168-241`); split `UnrecognizedStore`/`LayoutMismatch` refusals; `LAYOUT = 7` and the history doc on it; `bumbledb-schema-v6` fingerprint label; tag gaps in `schema/wire.rs`; retired grammar hints in `bumbledb-macros` (`unique`/`fk`/`enum`/`fresh` at `lib.rs:259-269,382-389,844-849`) and their compile-fail docs.
  - **bench:** `naive/successor/` (module named after an era); `driver/corpus_gen.rs` and `appperf/hosted.rs` braid references.
  - **TS:** `ts/crate/log-identities.json`; `tags.rs` retired arms; `ts/crate/src/log.rs` (dead).
  - **repo:** `scripts/version-roster.txt` (lockstep roster); `docs/release-1.0.md` … `release-1.3.1.md` (git history keeps them); `docs/perf/runs/1.1.0`; `.config/obligation-inventory.json`.

**How:** one PR per crate, after the structural deletions (C1–C6, D step 1) so less churns twice. Each PR ends with a grep gate that returns nothing for the markers above, with one documented exception: the word "tombstone" may stay only where it names the current design's concept, which today means none.

**Release:** `2.0.0` with fresh `docs/release-2.0.md`. Version strings in `Cargo.toml`/`package.json` jump to 2.0.0, and the platform-package lockstep roster goes (napi `version` syncs them).

### I. Hygiene

- Comment purge (C2).
- Relax pedantic lints, which deletes ~300 `#[expect]` and 265 empty `# Errors` headings.
- No history, ticket IDs or decision logs in source.

## 4. Swarm execution (all on `main`, no worktrees, no review waves)

### 4.1 Readiness

| Item | Status |
|---|---|
| Node 26.10.0 (MacPorts) | ✅ |
| rustup 1.29.1, nightly-2026-08-15 (pin) + nightly-2026-10-09, nextest 0.9.148 | ✅ |
| Machine: 12 cores, 96 GB RAM | ✅ ≤ 6 concurrent Rust agents |
| **Baseline Rust:** `cargo nextest run --workspace` on the pin: **2476 passed, 31 skipped**, 0 failed (cold build 1m10s, total 2m46s) | ✅ |
| **Baseline battery** (`scripts/battery.sh`: 5 clippy configs, nextest, alloc gate, addon, TS, packed consumers) | see §5 |
| Swarm tooling: `scripts/swarm/sandbox.sh` (HEAD + owned paths, private target dir; self-tested), `scripts/swarm/commit.sh` (own-path commits with index-lock retry) | ✅ |
| CLAUDE.md: design law, hard-cutover/legacy rule, **strict comment policy**, swarm protocol | ✅ |
| pnpm: the repo pins 11.9.0 and pnpm auto-switches to it. **D19: latest pnpm (12.10.1), no pnpm 11 anywhere** (W0) | decided |

### 4.2 Protocol

- **Coordinator (main session).**
  - Owns `Cargo.lock`, launches each wave as one Workflow, and runs the **wave gate** after it: fmt, clippy `-D warnings`, `nextest --workspace`, TS build and tests.
  - Fixes integration breaks, regenerates and commits `Cargo.lock`, and updates this plan.
  - Each wave starts only from a green gate.
- **Agents.**
  - Each gets an explicit **owned-path list**, never overlapping another concurrent agent's.
  - Build and test only through `sandbox.sh`, so a half-finished edit elsewhere can't break them.
  - Commit each plan item separately via `commit.sh "<ID>: …"` once its sandbox gate is green.
  - When blocked outside their paths, report instead of fixing.
- **Crate rule.** At most one agent edits a given crate's `src/` per wave, except W2 (comment-only edits can't break builds) and pre-split file lists.
- **Old log demolished first.** The old bumbledb-log stack is deleted in W1 rather than kept compiling through engine churn. The new S3 log is built fresh in W4/W5 on the new engine API. Until then `examples/notes` and the hosted TS tests are **excluded from the wave gate**.

### 4.3 One wave (owner decision): 9 parallel lanes, then one consolidator

Setup is done and committed (`6d5641585`):
- nightly-2026-10-09;
- `crates/bumbledb-node` as a workspace member with one lockfile;
- security lock refresh (rustls 0.23.45, napi 3.14.2, blake3 1.8.7, uuid 1.27);
- fearless_simd 1.1 dependency;
- noise lints relaxed, with 288 redundant expects removed;
- brittle tests deleted.

Baseline at setup: **2572 passed**, clippy (both configs) clean.

All lanes run at once on `main`. Every agent owns disjoint paths, builds and tests in its sandbox, and commits its own items. Each lane writes `docs/swarm/<lane>.md` to announce API changes and requests; lanes read each other's boards.

| Lane | Owns | Items |
|---|---|---|
| **engine-storage** | `crates/bumbledb/{Cargo.toml, src/lib.rs, error*, storage*, schema*, changes*, canonical*, encoding*, work*, digest.rs, verify_store*, alloc_counter.rs, value.rs, interval*, allen.rs, api.rs, api/db*}`, `crates/bumbledb/tests/*` except the files listed for other lanes, `crates/bumbledb-theory/**` | A (sweep), C1, C5, C6, C7 (`bumbledb::host`, early), C8, C9, C10, C15, C16, C17, G2 (engine), L (code) |
| **engine-query** | `crates/bumbledb/src/{ir*, plan*, exec.rs, exec/** except kernel and sink/aggregate, image*, api/prepared.rs, api/prepared/** except computed}`, `tests/{adversarial_ir, point_reads, reach_finalize_hunt}.rs` | A (plan validator), C3, C4, C11, C12, C13, E5 (lowering/fold/residual), E8, L (code) |
| **numeric** | `crates/bumbledb/src/{scalar.rs, exec/kernel.rs, exec/kernel/**, exec/sink/aggregate.rs, exec/sink/aggregate/**, api/prepared/computed.rs, api/prepared/computed/**}`, `tests/float_numerics.rs` | A (`gather_words`), E1, E3, E4a, E4b, E5 (MIN/MAX), E6, E7, C4 (aggregate spill) |
| **macros** | `crates/bumbledb-macros/**`, `crates/bumbledb-query-macros/**`, `crates/bumbledb-query/**`, `crates/bumbledb/tests/{schema_macro.rs, schema-compile-fail/**, compile_fail/**, query/**}`; root `Cargo.toml` `members` | A (`query!` string literal), C14, G6, G7 (doctests), L (macros) |
| **log-core** | `crates/bumbledb-log/**` | D: delete the old machine; build the new sans-IO core (U11) with nonce, `Idle\|InFlight`, ChangeSet entries, Freeze/Migration/Thaw, deterministic simulation tests |
| **bridge** | `crates/bumbledb-node/**`, `ts/src/native/binding.d.ts` | Delete the old log wire; F7 generated outputs; F8 serde inputs; D17 sync validate/compile; hosted verbs over log-core |
| **ts** | `ts/**` except `binding.d.ts`, `ts-log/**`, `examples/**` | D10/D19 pins; F1, F3, F4 (ts-log gone), F5, F6, F9, D17 (TS side), D6 protocol and stores, D7 migrations + CLI, D8 notes |
| **bench** | `crates/bumbledb-bench/**` | G8 cuts and restructure, rusqlite 0.40, E2 (micro + `float_stats`), adapt to engine API |
| **ci** | `.github/**`, `scripts/**` except `scripts/swarm/**`, `.config/**`, `.dockerignore`; root `Cargo.toml` `[profile.*]` | G1, G3, G4, G5, G10, G11, G12, G13, L (scripts and config) |

**Consolidator (one agent, after every lane):**
- Integration: a full green gate (fmt, 2 clippy configs, rustdoc, `nextest --workspace`, TS build and tests, notes).
- The strict **comment purge** across the whole repo.
- The **legacy cull** grep gate (L).
- The **D20 `bdb` extension** grep gate: no `bumbledb.lock`, `bumbledb.<kind>.v1`, `bumbledb.node`/`bumbledb.<platform>.node`, `.bumbledb/` or `bumbledb.*` brand remains anywhere; database dirs and checkpoint images use `.bdb`.
- Docs and README rewritten for 2.0; `docs/release-2.0.md`; versions bumped to 2.0.0.
- Commit `Cargo.lock`.
- Delete `scripts/swarm/` and `docs/swarm/`.

## 5. Investigator status

| Investigator | Status |
|---|---|
| Wave 1 audits (6) | done |
| Engine target design + downstream import inventory | done → §1, §3.C |
| Log target design (S3-first, dead simple, migrations) | done → §3.D |
| Dependency/toolchain bump plan | done → §3.B, §3.H |
| Float/SIMD target design (fearless_simd + keep NEON) | done → §3.E |
| TS SDK/bridge target design | done → §3.F |
| CI/test/bench target design (real S3 lane) | done → §3.G |
| Turso/libSQL + S3-native serverless DB deep research | done → §3.D and the report |

**All investigation is complete and the owner walkthrough is done. Every decision is resolved. Start Phase 0 when the Rust toolchain is installed and the owner says go.**

## Appendix: key numbers

- **Repo:** ~300k lines.
  - Engine: 61.7k product / 78.9k test.
  - Bench: 65k.
  - Log: 25.6k src / 13.8k tests.
  - ts/crate: 23.6k. ts/src: 12.7k. ts-log/src: 3.6k.
- **History:** 3,381 commits between 2026-05-17 and 2026-09-11. The last nightly bump touched 111 files, plus ~6 fallout commits.
- **Floats are already bit-exact across targets:** u64 order keys, integer-only SIMD, exact 34-limb SUM. Workstream E is about speed, not accuracy.
- **Recent releases:**
  - Effect 4.0.0 shipped stable on 2026-10-01; 4.0.2 on 2026-10-07.
  - fearless_simd 1.1.0 on 2026-10-06 (MSRV 1.89; no gather).
  - Nightly float news: `algebraic_*` has been stable since 1.98; `mul_add_relaxed` landed 2026-10-02.
- **Downstream use of `bumbledb::store::*`:**
  - log: 11 items;
  - bench: 8 items;
  - ts: `MapReport` and `StoreError` only;
  - `integration::*`: 9 items (log) and 5 (ts).
- **Unused downstream:**
  - collision-probe API: unused everywhere;
  - `CollectionBuilder` and `load_accepted`/`delete_accepted`: never used.
