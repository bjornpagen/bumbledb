# Verification manifest — the F3 execution map (P12)

Owner: P12 (independent coverage checker, chapter 62). Status: **planning
document; nothing here has been executed. Verification: NotRun for every lane
and every ID.** This manifest maps all 68 audit IDs and all 220 chapter-70
child families (plus the 17 parents) to concrete execution lanes with exact
commands derived from the implemented harnesses. It does not replace the
evidence ledger: `implementation/release-results.json` (P00-owned) is the sole
qualification record; this document is the map P00 executes against at F3 and
P12 reviews coverage with.

Binding rules (chapters 64/70):

- `Passed` requires an actual applicable execution with a NONZERO selected
  test inventory, reviewed output, and exact candidate revision/artifact
  identity. A lane that selects zero tests FAILS its coverage claim.
- Missing credentials/hardware/runner = `NotRun` and release-blocking, never
  `NotApplicable` and never exit-zero-skip counted green. The external lanes
  below name the precise missing authority.
- `NotApplicable` is reserved for declared platform scope (e.g. an
  ARM-instruction assertion on an x86 host) and must name that scope.
- Goldens regenerated during F3 (lane L14) invalidate previously collected
  evidence for the lanes that consume them; rerun those lanes after
  regeneration, in the order of section 4.
- Performance lanes serialize per host through `scripts/measure.sh`
  (`BUMBLEDB_MEASURE_LOCK`); no benchmark agent shares the fabric.

## 1. Environment matrix

| Requirement | Pin / source |
| --- | --- |
| Rust toolchain | `nightly-2026-08-15` + miri/llvm-tools/rustfmt/clippy/rust-src, targets incl. `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin` (`rust-toolchain.toml`) |
| cargo-nextest | `0.9.143` (battery.sh installs `--locked` if absent); config `.config/nextest.toml` (default profile: shared pool, fail-fast off, retries 0, slow warn 180s, NO termination — see §5) |
| Lean | `leanprover/lean4:v4.32.0` (`lean/lean-toolchain`), lake |
| Node | ≥ 24 (both package `engines`); F3 also exercises the OLDEST declared runtime (Node 24) and current (26) per G13 |
| pnpm | `11.9.0` (`packageManager` pins) |
| Effect | `4.0.0-rc.112` exact peer (both packages) |
| Python | python3 (flame renderer selftest, bench viz) |
| Primary host | Apple Silicon (M2 Max class), macOS, APFS |
| Additional hosts | see external lanes X03/X04 |
| Dependency install | pinned lockfiles, lifecycle hooks allowed ONLY at F3 step 2 (chapter 64 order item 2); no stale `ts/dist` or `.node` artifact may precede a fresh build |

## 2. Local lane catalog

Output convention (proposal to P00): every lane tees its full log to
`implementation/evidence/f3/<lane>.log`; machine-readable reports (junit,
JSON) land beside it; `release-results.json` rows reference these paths with
sha256 digests. Artifact inputs are the frozen candidate source tree at the
recorded revision plus the named fixtures.

| Lane | Command (exact) | Runner / environment | Fixture size | Expected nonzero inventory | Artifact inputs | Output |
| --- | --- | --- | --- | --- | --- | --- |
| L01 checker regressions | `node --test scripts/release-results.test.mjs` | any host, Node ≥24 | none | 8 tests (this file's inventory) | `scripts/release-results.mjs`, `final-solution/50`, `final-solution/70` | `evidence/f3/L01.log` |
| L02 format/lint | `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings` | primary host | none | clippy compiles every workspace target incl. all `tests/*` (a target that fails to compile is a FAIL, not a skip) | source tree | `evidence/f3/L02.log` |
| L03 dependency-lean log | `cargo check -p bumbledb-log --no-default-features` | primary host | none | compiles (structural gate) | source tree | `evidence/f3/L03.log` |
| L04 workspace battery | `cargo nextest run --workspace` | primary host, ≥8 GiB free disk in `$TMPDIR` | temp LMDB stores (MiB scale), subprocess re-exec children | ≥ 2,000 selected tests expected across bumbledb (unit+integration incl. `adversarial-close-drain`), bumbledb-log (P04/P05/P09 lanes + P12 `adversarial-process`/`adversarial-migration`/`adversarial-hostile-bytes`/`adversarial-trace`/`gate-baseline-ports`), bumbledb-theory, bench in-crate suites, macros/query crates; **zero matches on any of those named files fails coverage** | source, `crates/bumbledb-log/conformance/v3/identities.json`, `fixtures/` | `evidence/f3/L04.log` + junit |
| L05 check.sh gates | `scripts/check.sh` | primary host | small | workspace doc tests ≥1; alloc_gate (release, `--features alloc-counter`); ground-off clippy+tests; trace-feature tests; `--all-features` clippy; bench `--features obs` clippy+tests; `python3 scripts/flame.py selftest` | source | `evidence/f3/L05.log` |
| L06 Lean | `scripts/lean.sh` (lake build; sorry/admit/axiom greps; `lake exe conformance conformance/cases`; bench three-way comparator `cargo test -p bumbledb-bench --lib -- --ignored --exact conformance::tests::three_way_conformance_over_the_checked_in_corpus`) | primary host, Lean v4.32.0 | checked-in conformance corpus | lake build of all modules incl. new `Txn/Support`, `Float64/Sum`, `FloatInterval`, `Query/Stages`; conformance cases >0; comparator `1 passed` | `lean/`, `lean/conformance/cases`, regenerated corpora (L14) | `evidence/f3/L06.log` |
| L07 census | `scripts/spec-census.sh` | primary host | none | bridge-ledger rows = 116 (P11), census clean | `lean/Bumbledb/Bridge.lean`, source | `evidence/f3/L07.log` |
| L08 bridge crate | `cargo fmt --manifest-path ts/crate/Cargo.toml --check && cargo clippy --manifest-path ts/crate/Cargo.toml --all-targets -- -D warnings && cargo nextest run --manifest-path ts/crate/Cargo.toml --config-file .config/nextest.toml && cargo test --manifest-path ts/crate/Cargo.toml --doc` | primary host | small | P06 reactor/session/pool/log/tenants/fingerprint-lock suites ≥ 40 tests; zero-selection fails | `ts/crate/`, `ts/crate/log-identities.json` (L14 regenerates) | `evidence/f3/L08.log` |
| L09 addon build | `(cd ts && pnpm run build)` | primary host, pinned pnpm | — | fresh `.node` + `dist/` (structural; battery runs it before the TS lanes so a stale artifact is unrepresentable) | `ts/crate`, `ts/src`, lockfile | `ts/dist`, `evidence/f3/L09.log` |
| L10 core TS | `(cd ts && pnpm test && pnpm typecheck && pnpm lint)` — `pnpm test` = `pnpm run build && node --test 'test/**/*.test.ts'` (the build is part of the command; never reuse an old dist) | primary host, Node 24 AND Node 26 passes | temp dirs | every `ts/test/*.test.ts` incl. P12 `adversarial-boundary.test.ts`; ≥ 25 suites; typecheck+lint clean | fresh L09 artifact, `ts/test/fixtures/` (incl. `legacy-store/data.mdb` 0.x refusal fixture, `tags.json`) | `evidence/f3/L10.log` |
| L11 log TS | `(cd ts-log && pnpm test && pnpm typecheck && pnpm lint)` (test invokes its build) | primary host, Node 24 AND 26 | temp dirs | every `ts-log/test/*.test.ts` incl. P08 machine/roster suites, P10 migrations suites, P12 `adversarial-capabilities.test.ts`; typecheck+lint clean | fresh L09 artifact via workspace link at qualification-build time; packed variant is L12 | `evidence/f3/L11.log` |
| L12 packed consumers | `scripts/packed-import.sh` | primary host, `NODE_OPTIONS`/`NODE_PATH` unset (script enforces) | packed tarballs | both tarballs pack, install into a bare consumer, import, and emit strict consumer declarations; effect peer resolved at the exact pin | fresh tarballs from staging | `evidence/f3/L12.log` |
| L13 miri | `scripts/miri.sh` (native aarch64 + cross-interpreted `x86_64-unknown-linux-gnu` with `scripts/miri-cross-cc.sh`) | primary host | none | the scripted filter set selects >0 tests on BOTH targets | source | `evidence/f3/L13.log` |
| L14 golden regeneration (execute once, then rerun consumers) | `cargo run -p bumbledb-log --bin identities > crates/bumbledb-log/conformance/v3/identities.json && cp crates/bumbledb-log/conformance/v3/identities.json ts/crate/log-identities.json` (P05-recorded); `cargo run -p bumbledb-bench -- corpus-float --seed 0xB0B --out fixtures/float` (P11/P14); theorygen corpus regeneration (P11: RNG stream shifted — ALL theorygen-derived corpora); schema-fingerprint re-pins from the deliberate failing tests (P14 `schema.rs::the_fingerprint_is_pinned`, P07 `notation-corpus/schema-fingerprint.txt`) | primary host | fixture files | each generator emits nonempty deterministic files; the red forcing tests (`conformance_v3.rs`, fingerprint pins) go green ONLY through regeneration, never by weakening | binaries built from the candidate | regenerated fixtures + `evidence/f3/L14.log` |
| L15 storage/large local halves | `BUMBLEDB_LARGE_STORE_DIR=<volume> cargo test -p bumbledb --release -- --ignored large` (the `storage/store/tests/large.rs` lane; needs ≥ 60 GiB free) — on the primary host ONLY if the volume exists, else lane X02 | storage-qualified volume | > 40 GiB populated store | the `#[ignore]`d large tests selected and run (`--ignored`); allocated-block (not sparse) assertions internal | generated streaming fixture (bench `largefix` generator) | `evidence/f3/L15.log` |
| L16 measured performance | `scripts/measure.sh <lane>` wrapping, in order: `cargo run -p bumbledb-bench -- verify …` / `verify-store`, `app-perf`, existing `bench`/`scenarios`/`crud`/`lawful`/`writes`/`curves`/`churn`/`heap` lanes, `storage` + `space` census, `hash-probe` (equivalence → KAT → timing), `scripts/bench-night.sh <out-dir>`; baseline comparison against an ISOLATED checkout of `01084e3e` built separately | primary host, serialized via `BUMBLEDB_MEASURE_LOCK`; later repeated on X04 hosts | per plan `docs/perf/measurement-plan.md` §8 | each cell emits its scorecard row; `hash-probe` refuses to time before equivalence+KAT pass; KAT file must exist (absence = NotRun) | frozen corpora, KAT vector file (hand-copied per plan §6), baseline checkout | `evidence/f3/L16/…` |
| L17 fresh pinned installs | `pnpm install --frozen-lockfile` (ts, ts-log), `cargo fetch --locked` | primary host, F3 step 2 only | — | lockfiles resolve exactly; lifecycle hooks run here for the first time | lockfiles | `evidence/f3/L17.log` |
| L18 full battery spine | `scripts/battery.sh` (runs L01–L12 in its recorded order) — the one green exit that gates the candidate commit | primary host | union | every embedded lane's inventory | union | `evidence/f3/L18.log` |

## 3. External-blocked lanes (NotRun-blocking; precise missing authority)

| Lane | Command / drill | Missing authority (release-blocking until supplied) | Covered IDs |
| --- | --- | --- | --- |
| X01 real S3 | `BUMBLEDB_S3_SMOKE_BUCKET=<bucket> AWS_ACCESS_KEY_ID=… AWS_SECRET_ACCESS_KEY=… [BUMBLEDB_S3_SMOKE_ENDPOINT/REGION] cargo nextest run -p bumbledb-log -E 'binary(s3_smoke)'` plus the full S3-01..06 campaign (conditional create/update races, response-loss proxy, multipart/streamed checkpoints, aborted uploads, permission failures, credential rotation) and `duty` CLI S3 arms | **Explicitly provisioned disposable S3 test prefix + least-privilege test credentials.** The current CI skip-with-exit-zero is recorded as NotRun, never green. | S3-01..06, REP-004 real-fault half, PROTO-04/05/06/20 hosted halves, MIG-05 hosted, OPS-002 cloud half, G08 |
| X02 >40 GiB populated storage runner | L15 command + bench `largefix` population/mutation/checkpoint/restore schedule (`docs/perf/measurement-plan.md` §7) with a recorded 8 GiB memory allowance | **A storage-qualified runner with > 60 GiB free real disk** (sparse files refused by the harness's allocated-block check). | STORE-01/02/10, BACKUP-05, E-LARGE, Q-LARGE-STORE, RUN-08 (size half), API-11, APP-LARGE, PERF-002, G05 |
| X03 memory-enforced >RAM Linux runner | bench beyond-RAM plan: cgroup v2 `memory.max` bound, dataset ≥ 4× the bound; `RLIMIT_AS` forbidden by the harness | **An isolated Linux host with cgroup v2 write authority.** | RUN-08 (memory half), Q-DISK/Q-LARGE-STORE enforcement halves, F-RESOURCE large half, TS-MIG-05 large half, G05 item 1 |
| X04 platform matrix | full L04/L08/L10/L11/L12 + float corpus (F-CROSS) + packed-artifact execution on **linux-arm64 (Graviton) at the documented libc floor** and **linux-x64 (Vercel Node floor)**; oldest+current Node | **Access to Graviton and x86 Linux runners (CI or hardware) with the declared libc floor.** | PKG-04, APP-04, F-CROSS/F-ENV foreign-host halves, FFI-06 target half, E-BRIDGE cross-platform, G01/G02/G13 platform rows |
| X05 power-failure rig | FS-05 machine-failure/durability simulation (abrupt power loss during commit/rename/fsync schedules) | **A power-failure rig or virtualized power-cut harness authority; at least one release campaign must run it (G06).** Until then the OS-crash SUBSET is covered by the real SIGKILL lanes (L04 `crash.rs`, `local_ownership.rs`, `adversarial-process.rs`) and the residual substrate assumption is stated in the ledger. | FS-05, E-DURABILITY substrate half, G06 fault row |
| X06 deployment drills | Next.js + Alchemy server-only example: production build, deploy to Vercel Node x64, IAM role attachment/rotation, request/migration drills (chapter 33) | **An authorized disposable cloud scope (Vercel project + AWS IAM/bucket) — never application prefixes.** | APP-01..08 deployed halves, RUN-12/13/14, SDK-013/ARCH-005 consumer halves, OPS-003, ASS-003 examples, G13/G14 deployment rows |
| X07 registry rehearsal | PKG-07A: exact staged digests, empty-project + private/disposable-registry install rehearsal, simulated partial publication | **A disposable/private npm registry scope.** | PKG-07A |
| X08 public distribution | PKG-07B: download the PUBLIC registry artifacts after an authorized publication; verify digest/pins/install | **Explicit publication authorization — post-promotion only; excused pre-promotion by the checker, required after.** | PKG-07B |
| X09 full CI matrix | push the qualified candidate WITHOUT `[skip ci]`; inspect the complete configured matrix (`.github/workflows`) | **P00's final push (chapter 64 order item 8); path-filtered runs do not count.** | G01/G16 CI rows |

## 4. F3 execution order (chapter 64 §"Final campaign order", instantiated)

1. P00 freezes candidate + environment inventory (disk/credentials/hardware),
   records the local candidate commit SHA.
2. L17 fresh pinned installs; L01.
3. L18 battery spine (= L02–L12) — repair-and-rerun loop; L13 miri; L06/L07.
   First-run expectation: the deliberate red forcing tests (identities golden,
   fingerprint pins, corpus staleness) fail → run L14 regeneration → rerun the
   consuming lanes (L04, L06, L08, L10, L11).
4. Pre-format measurements: L16 `hash-probe` + `space` + long-key probes →
   C12 format/hash decision (P00). If the decision changes any format: apply,
   re-run L14 and every consuming lane, re-freeze goldens.
5. Whole-product/large-data: L15/X02, X03, remaining L16 cells, PERF-003
   hosted cells (needs X01).
6. Artifacts/backends/targets: L12 again from FRESH staging, X01, X04, X06,
   X07.
7. Full requalification: rerun every lane invalidated by fixes; final fresh
   L18 + X09.
8. Ledger completion: P00 fills `release-results.json` (one row per child with
   exact test names, counts, digests, review refs); `node
   scripts/release-results.mjs pre-promotion implementation/release-results.json
   <exact-staged-revision>`; honest blocked-gate report for X-lanes still
   missing authority.

## 5. nextest stdio LEAK investigation plan (no suppression, no serialization)

Recorded observation (final-solution/90): one LEAK diagnostic on a focused log
suite at the frozen checkpoint; an isolated rerun passed without LEAK; the
Darwin FD race is a HYPOTHESIS, not attribution. Plan:

1. Reproduce first: run the F3 `cargo nextest run --workspace` (default
   profile: parallel, retries 0, no leak-timeout override) three times;
   collect every `LEAK` line with test names into
   `evidence/f3/leak-inventory.txt`. Do not change `.config/nextest.toml`
   (no `leak-timeout` inflation, no `test-threads = 1`, no per-test overrides)
   and do not mark tests `serial`.
2. Classify each leaking test by mechanism: (a) a child process from the
   re-exec harnesses (`local_ownership.rs`, `adversarial-process.rs`,
   `adversarial-migration.rs`, `crash.rs`, `sysv_sem_rmid.rs`) that inherited
   the harness stdout/stderr and outlived the test (expected for parked
   children — the parent must SIGKILL and REAP before returning; a LEAK there
   is a real test bug: fix the reap, not the reporter); (b) a spawned thread
   holding an inherited descriptor past test exit (fix the join); (c) a
   genuinely unattributed descriptor — escalate as a new defect row in the
   50-matrix (counts are a floor).
3. Verify attribution with `nextest`'s per-test leak report plus `lsof -p` on
   a paused reproduction (`--no-capture` single-test rerun ONLY as a
   diagnostic aid, never as the qualification run).
4. The qualification run's evidence records the leak count verbatim; a LEAK
   on a P12-owned harness blocks that lane's Passed until the reap/join fix
   lands and the full parallel run is clean. The Darwin-FD-race hypothesis is
   confirmed or retired by the classification, not assumed.

P12 harness discipline already applied for (a): every parent kills AND reaps
(`child.wait()`) each child, and child stdout is piped (not inherited), so a
parked child cannot hold the harness's own stdio.

## 6. Audit IDs — all 68 → lanes

Anchors name the primary executed suites (packet-recorded inventories);
`+P12` marks this packet's independent arms. Lanes are from §2/§3.

| Audit ID | Lanes | Executed anchors |
| --- | --- | --- |
| REP-001 | L04, X01 | `lane_gc.rs` (gc01/gc02), `history/authority.rs::tests`, +P12 `gate-baseline-ports.rs::rep001_*`, `adversarial-process.rs` GC arm |
| REP-002 | L04 | `history/authority.rs::tests`, `lane_recovery.rs` (captured-tip reconstruction), `lane_checkpoint.rs` |
| REP-003 | L04 | `lane_local_roots.rs`, `lane_gc.rs::gc03`, `lane_erase.rs` (retained roots) |
| REP-004 | L04, X01 | `tests/writer_hosted.rs`, `history_admission.rs`, +P12 `gate-baseline-ports.rs::rep004_*`; real-fault half X01 |
| REP-005 | L04 | `local_ownership.rs` (SIGSTOP arms), `store/fence.rs`+`fs.rs` tests, +P12 `adversarial-process.rs::a_suspended_history_owner_*` |
| REP-006 | L04 | `tests/writer_hosted.rs` (bounded attempts), `closure/history_model` (`bounded_attempts_return_outcome_unknown_not_a_lie`), +P12 `adversarial-trace.rs` contention |
| REP-007 | L04 | `lane_gc.rs` (gc01/gc03/gc05), +P12 `adversarial-process.rs::a_kill_mid_sweep_*`, `gate-baseline-ports.rs::rep001_*` |
| REP-008 | L04 | `lane_checkpoint.rs` (rebase/lineage), `history/decision.rs::tests`, +P12 `adversarial-hostile-bytes.rs::a_decision_off_the_exact_parent_*` |
| REP-009 | L04 | `local_ownership.rs`, `runtime/tests.rs` (directory owner), `tenants.rs::tests`, +P12 `adversarial-process.rs` |
| REP-010 | L04 | `lane_b_fs_store.rs` (per-phase faults), `local_ownership.rs::fs02`, +P12 `adversarial-process.rs::a_kill_at_the_publication_boundary_*` |
| REP-011 | L04 | `store.rs` key-grammar tests (canonical lower-case), `lane_recovery.rs` (foreign cache), L11 borrows cross-origin |
| REP-012 | L04 | `lane_b_mem_store.rs` (epoch-window probe, no slot scans), `tenants.rs::tests` |
| REP-013 | L04 | `lane_gc.rs` (gc07/gc08/gc10 durable progress), +P12 `adversarial-process.rs` sweep-kill resume |
| REP-014 | L04, L16 | `lane_checkpoint.rs` (moved-head rebase, no quiet window), `codec.rs` tests; PERF-004 cells L16 |
| REP-015 | L04 | `tests/writer_hosted.rs` (catch-up bound), `closure/history_model`, +P12 `adversarial-trace.rs` |
| REP-016 | L04, L10 | `replica.rs::tests` (no writable escape), L10 `effect-surface.test.ts` (no raw db export) |
| REP-017 | L04 | `local_ownership.rs`, `lane_recovery.rs` (lock-first, release-last), `runtime/tests.rs` |
| REP-018 | L04 | `store.rs`/`lane_b_mem_store.rs` (kind/epoch/length/digest), +P12 `adversarial-hostile-bytes.rs` (whole battery) |
| REP-019 | L04 | `lane_gc.rs::gc07` (failed-delete evidence), +P12 sweep-kill resume |
| REP-020 | L04 | `history/authority.rs`, +P12 `gate-baseline-ports.rs::rep020_*`, `adversarial-hostile-bytes.rs::a_forged_rejection_*` |
| ENG-001 | L04 | theory `interval.rs::tests`, core `canonical/tests.rs` (P01) |
| ENG-002 | L04 | core `canonical/tests.rs` (forged codec refusal), `encoding/tests.rs` |
| ENG-003 | L04 | `snapshot_coherence.rs`, `api/db/tests.rs` compact coherence |
| ENG-004 | L04 | schema tests (fresh unrepresentable), `api/db/tests.rs`, +P12 `gate-baseline-ports.rs::rep004_*` |
| ENG-005 | L04 | `schema/judge/tests.rs` (competing permutations), `judged.rs`, `candidate_visibility.rs` |
| ENG-006 | L04 | `snapshot_coherence.rs` (deleted text gone), `image/intern.rs` tests (P03 successor) |
| ENG-007 | L04 | `schema/judge/tests.rs` (resource ≠ rejection), +P12 `gate-baseline-ports.rs::eng007_*` |
| ENG-008 | L04, L05 | `lifecycle.rs` (no NOSYNC flags), bench cli refusal tests (P14R), ground-off lane L05 |
| QRY-001 | L04 | `api/prepared/result/tests.rs`, prepared suites (Q-ATOMIC rows) |
| QRY-002 | L04, X02/X03 | `exec/scratch/tests.rs`, forced-fallback parity suites; large halves X02/X03 |
| QRY-003 | L04, L10 | prepared budget suites, L10 `effect-core`/`changes-lazy` (policy observable) |
| SDK-001 | L04, L11 | `tests/writer_local.rs`/`writer_hosted.rs`, +P12 `gate-baseline-ports.rs::sdk001_*`, L11 `submit-certainty.test.ts` |
| SDK-002 | L08, L04 | `runtime/session.rs::tests` (close stops admission), `tenants.rs::tests`, +P12 `adversarial-close-drain.rs::writers_racing_close_*` |
| SDK-003 | L10 | `changes-lazy.test.ts` (mutation-across-await), `effect-core.test.ts`; L11 `command.test.ts` seal envelope |
| SDK-004 | L04, L11 | `tenants.rs::tests` (distinct borrows), L11 `borrows.test.ts`, +P12 `adversarial-boundary.test.ts` double-take |
| SDK-005 | L08 | `runtime/tests.rs` (open vs shutdown), `runtime/session.rs::tests` shutdown drain |
| SDK-006 | L04 | `local_ownership.rs` (no TTL mint), `store/fence.rs`, +P12 suspended-owner arm |
| SDK-007 | L08, L10 | `runtime_managed_db_close` lanes, +P12 `adversarial-boundary.test.ts::retained wrappers…`, `adversarial-close-drain.rs` |
| SDK-008 | L04, L10 | `replica.rs::tests`, `effect-surface.test.ts` (no writable twin) |
| SDK-009 | L04 | sealed-data submission surface (`writer_local.rs`), `api/db/tests.rs` witness rows |
| SDK-010 | L08, L10 | pool cancellation tests, L10 `runtime.test.ts` interruption joins, `result-pages.test.ts` |
| SDK-011 | L08 | pool reservation/queue tests (P06), `tenants.rs::tests` pressure |
| SDK-012 | L04 | `tenants.rs::tests`, `lane_b_mem_store.rs` (no lifetime scans) |
| SDK-013 | L04, L10, L12 | C-absence checks (P13 lanes), FFI suites, packed import; Rust/Node safety: L08 + L13 |
| SDK-014 | L04 | `candidate_visibility.rs` (candidate never readable), `tests/writer_local.rs` rejection-retains-session |
| SDK-015 | L04 | no-callback-replay surface: `writer_local.rs` retry-dedup, `history_admission.rs` |
| SDK-016 | L04, L11 | `lane_recovery.rs` (foreign/unidentified cache), `lane_ops.rs`, +P12 `adversarial-capabilities.test.ts` cross-origin |
| ARCH-001 | L04 | `history/authority.rs`, `naive/successor/admission.rs`, +P12 `adversarial-trace.rs` witnessed/ABA |
| ARCH-002 | L04 | one-tenant-order suites (`writer_*`, `history_model`) |
| ARCH-003 | L04 | `history/receipt.rs::tests` (retirement), `history_admission.rs`, `lane_ops.rs` retirement rows |
| ARCH-004 | L04, L11 | `lane_recovery.rs` binding tests, +P12 `adversarial-hostile-bytes.rs::a_foreign_identity_*`, `adversarial-capabilities.test.ts` |
| ARCH-005 | L04, L11, L12 | one-machine evidence: `identities.json` golden through Rust (L04/L08) and TS (L11) + packed (L12); no public Rust log SDK (roster checks) |
| ARCH-006 | L16 | hosted contention cells (`appperf::hosted`, PERF-003 schedule) |
| OPS-001 | L04, L11 | `migration_*.rs` (P09), `migrations-*.test.ts` (P10), +P12 `adversarial-migration.rs` |
| OPS-002 | L04, X01 | `lane_backup_restore.rs`, `duty.rs`; cloud half X01 |
| OPS-003 | X06, L11 | outbox/blob drill in the example (X06); `lane_backup_restore.rs` RESTORE-03 hazard row (L04) |
| OPS-004 | L08, L11 | `tenants.rs::tests`, `borrows.test.ts`, noisy-neighbor pool tests |
| OPS-005 | L04, L11 | `replica.rs::tests` (typed unavailable), `history-open.test.ts` (missing ≠ empty), +P12 publication-kill arm |
| OPS-006 | L11 | `errors.test.ts`, `admin.test.ts`, `inspect.rs` redaction fixtures (L04) |
| PERF-001 | L16 | app-perf post-write cells + hot-path gates (`check-asm.sh`, alloc gates L05) |
| PERF-002 | L16, X02 | space census, churn/heap lanes, large fixtures |
| PERF-003 | L16, X01 | `appperf::hosted` per-terminal accounting over the real backend |
| PERF-004 | L04, L16 | `lane_checkpoint.rs` progress rows; checkpoint-pressure cells |
| PERF-005 | L08, L16 | pool fairness tests; event-loop/layer decomposition cells |
| ASS-001 | L06, L04 | `Txn/Support.lean`, `naive/successor/admission.rs` |
| ASS-002 | L04, L06 | `closure/history_model` + `verify_trace`, +P12 `adversarial-trace.rs` (model↔production differential), staged/query models |
| ASS-003 | L10, L11, L12, X06 | cookbook/readme doc tests, packed examples, deployed example |
| ASS-004 | L01 | evidence ledger + checker regressions; `audit/` preserved (P00 review) |

## 7. Child families — all 220 → lanes

Per-child rows. "Anchors" are the executed suites (already authored; NotRun);
where a child's full scope needs an external lane it is named explicitly.

### CONC-01..06 (chapter 02) — primary P04

| Child | Lanes | Anchors |
| --- | --- | --- |
| CONC-01 | L04, L06 | `naive/successor/admission.rs` (mutable-support), `Txn/Support.lean`, `history_admission.rs` |
| CONC-02 | L04 | `schema/judge/tests.rs` key/capacity counterexamples, `admission.rs` union-closure counterexamples, +P12 `adversarial-trace.rs` witnessed schedule |
| CONC-03 | L04 | `tests/writer_local.rs` exact-state ABA, +P12 `adversarial-trace.rs` ABA differential |
| CONC-04 | L04 | `history/authority.rs::tests`, `closure/history_model/tests.rs`, +P12 contention trace |
| CONC-05 | L04 | `naive/successor/admission.rs` (`shared_closed_vocabulary_does_not_merge_supports`) |
| CONC-06 | L04, L16 | one-tenant-authority suites + ARCH-006 contention measurement |

### E-* (chapter 10, 12 children) — primary P01 (P02 storage cases)

| Child | Lanes | Anchors |
| --- | --- | --- |
| E-DELTA | L04 | `changes/tests.rs` (720 permutations), `api/db/tests.rs` three-lane equality |
| E-VALUE | L04 | theory `interval.rs`/`schema.rs` tests, `api/db/tests.rs` closed-relation walls |
| E-CODEC | L04, L10 | `canonical/tests.rs` goldens/forgeries, `boundary-codec.test.ts` |
| E-SNAPSHOT | L04 | `snapshot_coherence.rs`, `api/db/tests.rs` empty-write row |
| E-NO-RESERVE | L04, L10 | schema/spec tests (fresh unrepresentable), `types.test.ts` fresh absence, +P12 `rep004_*` |
| E-ADMIT | L04 | `schema/judge/tests.rs`, `judged.rs` physical path, `verify_store/tests.rs` offline half |
| E-TEXT | L04 | `snapshot_coherence.rs` deleted-text rows, `image/intern.rs` tests |
| E-DURABILITY | L04, X05 | `lifecycle.rs`, `crash.rs` subprocess kills; power-failure residual X05 |
| E-VISIBILITY | L04 | `candidate_visibility.rs`, `api/db/tests.rs` own-writes/fall-through |
| E-ORIGIN | L04 | `schema/fingerprint.rs` v6 byte goldens, `lane_recovery.rs` binding |
| E-LARGE | L15/X02 | `storage/store/tests/large.rs` (`--ignored`, `BUMBLEDB_LARGE_STORE_DIR`) |
| E-BRIDGE | L08, L10, X04 | `fingerprint_lock.rs` twin pin, marshal/tags tables, `wire-tags.test.ts` |

### F-* (chapter 11, 13 children) — primary P03 (P01 values, P11 oracles)

| Child | Lanes | Anchors |
| --- | --- | --- |
| F-CANON | L04 | theory `float.rs::tests`, `verify/f64_oracle/tests.rs` |
| F-GOLDEN | L04, L14 | theory bit fixtures + `corpus-float` regenerated corpus + `driver/corpus_float.rs` determinism |
| F-ORDER | L04 | order-key ladders (theory + oracle + encoding tests) |
| F-ARITH | L04 | `float/exact.rs` differentials, `f64_oracle` reference arithmetic |
| F-ENV | L04, X04 | guard tests (`exec/kernel/numeric`), oracle-vs-host differentials; forced FTZ/DAZ + foreign-host halves on X04 targets |
| F-AGG | L04, L06 | aggregate sink suites, `Float64/Sum.lean` kernel goldens, oracle sum/mean |
| F-SET | L04 | `naive/successor/staged.rs` grain fixtures, judge grouped-measure rows |
| F-OPT-NEG | L04 | `ir/normalize/fold` negative rewrites, oracle native-fold divergence fixture |
| F-CROSS | L10, L14, X04 | `boundary-codec.test.ts` bit images + regenerated float corpus through Rust/Node on all three targets |
| F-WIRE | L04, L10 | canonical tag 4/9 goldens, `$f64` wire tests |
| F-RESOURCE | L04, X03 | `exec/scratch/tests.rs` transitions; enforced->RAM half X03 |
| F-PROOF | L06, L07 | `Float64/Sum.lean`, `FloatInterval.lean`, bridge ledger census |
| F-INTERVAL | L04 | theory dense battery, `allen.rs::dense_tests`, `finterval_oracle` |

### Q-* (chapter 12, 13 children) — primary P03

| Child | Lanes | Anchors |
| --- | --- | --- |
| Q-ATOMIC | L04 | `result/tests.rs`, prepared partial-output rows |
| Q-BUDGET | L04 | scratch/image charge-before-growth tests, prepared budget rows (sink-internal spill gap: see §8) |
| Q-DISK | L04, X03 | scratch spill parity, forced-fallback suites; enforcement X03 |
| Q-LARGE-STORE | L15/X02 | large store lane + prepared execution over it |
| Q-COLLISION | L04 | `collision.rs` (forced constant fingerprints), interner exactness tests |
| Q-FALLBACK | L04 | forced-fallback vs resident differential (prepared suites) |
| Q-RECUR | L04, L06 | rec-boundary refusal tests, `Query/Stages.lean` containment, staged model |
| Q-GROUP | L04 | aggregate/computed stage suites, grain fixtures |
| Q-TEMPORAL | L04 | dense-interval probes, allen tests, `finterval_oracle` |
| Q-LIFETIME | L04, L10 | `trim()`/retained-bytes tests, `result-pages.test.ts`, +P12 `adversarial-close-drain.rs::retained_owned_results_*` |
| Q-FAIR | L08, L16 | pool fairness/queue tests; two-tenant noisy-neighbor cells |
| Q-IR | L04 | ir validate/normalize suites, `bumbledb-query/tests/composition.rs` |
| Q-INJECT | L04 | scratch cancellation quanta, prepared error-injection rows |

### P-* (chapter 13, 9 children) — primary P11

| Child | Lanes | Anchors |
| --- | --- | --- |
| P-KERNEL | L06 | lake build + `#guard` kernel goldens; no sorry/admit/axiom greps |
| P-SEMANTIC | L04, L06 | staged/admission/history models vs production (+P12 trace differential) |
| P-FLOAT | L04, L06 | `f64_oracle`, `Float64/Sum.lean`, F-ARITH first-run rule (P01) |
| P-REPRESENTATION | L06, L04 | conformance corpus three-way comparator (post-L14 regeneration) |
| P-DISK | L04, X02 | crash/resize schedules; large-store correspondence |
| P-MEMORY | L05, L13 | alloc gates, miri both targets |
| P-SCHEDULE | L04 | `closure/history_model` schedule corpus, +P12 `adversarial-process.rs` real-process schedules |
| P-ARTIFACT | L09, L12 | fresh addon build + packed import + declaration isolation |
| P-PERF | L16 | measurement-plan cells with provenance stamps |

### PROTO-01..20 (chapter 20) — primary P04

| Child | Lanes | Anchors |
| --- | --- | --- |
| PROTO-01 | L04 | `writer_hosted.rs` single-successor rows, `history_model` interleavings, +P12 contention trace |
| PROTO-02 | L04 | `writer_local.rs` stable receipts, `history_model`, +P12 trace stability |
| PROTO-03 | L04 | same-id/different-bytes conflict rows (`writer_local.rs`, `history_model`) |
| PROTO-04 | L04, X01 | MemStore drop/apply fault arms (`lane_b_mem_store.rs`, `writer_hosted.rs`), +P12 lost-CAS trace; real S3 X01 |
| PROTO-05 | L04, X01 | unknown-CAS resolve rows, +P12 `adversarial-process.rs` publication-kill |
| PROTO-06 | L04 | paused-CAS gate schedules (`lane_gc.rs::gc02` shape, MemStore Gate) |
| PROTO-07 | L04 | `candidate_visibility.rs`, `writer_local.rs` losing-candidate rows |
| PROTO-08 | L04 | witnessed-decrement rows + blind variant (`history_model`, `writer_local.rs`), +P12 trace |
| PROTO-09 | L04 | ABA stamp rows (`history_model::proto_09`), +P12 ABA differential |
| PROTO-10 | L04 | +P12 `rep020_*` all-or-none, `history/decision.rs` nested binding |
| PROTO-11 | L04 | +P12 `rep004_*` entity bytes, `writer_hosted.rs` retry rows |
| PROTO-12 | L04 | +P12 `adversarial-process.rs::a_kill_at_the_publication_boundary_*` (real kill), `lane_recovery.rs` REC halves |
| PROTO-13 | L04 | catch-up budget rows (`writer_hosted.rs` DEFAULT_CATCH_UP bound) |
| PROTO-14 | L08, L11 | close/revoke during seal/upload/in-flight (session close lanes, `close-report.test.ts`), +P12 close-drain |
| PROTO-15 | L04 | +P12 `adversarial-hostile-bytes.rs` (forged outcome/foreign identity/truncation), `history/decision.rs` refusals |
| PROTO-16 | L04 | rotation/freeze/retirement rows (`history/authority.rs`, `history_admission.rs`, `lane_ops.rs`, `history_model::proto_16`) |
| PROTO-17 | L04 | embedded-vs-LocalHistory shared-types rows (`writer_local.rs`, `api/db/tests.rs`), +P12 suspended-owner resolve |
| PROTO-18 | L04 | tiny-budget rows (`history_admission.rs` limits, `candidate_visibility.rs` seal-fault: MAP_FULL-after-judgment dispatches nothing) |
| PROTO-19 | L04 | genesis rows (`history/decision.rs` genesis sentinels, `migration_execute.rs` restore/migration genesis, PROTO-19 hydration-control rows in `lane_recovery.rs`) |
| PROTO-20 | X01, L16 | hosted latency/throughput contention cells over real S3 (`appperf::hosted` + `contention_schedule`) |

### STORE-01..10, LOCAL-01..03 (chapter 21) — primary P05

| Child | Lanes | Anchors |
| --- | --- | --- |
| STORE-01 | L04, X02 | `codec.rs` stream tests (small-scale); >RAM/>40 GiB checkpoint X02 |
| STORE-02 | L04, X02 | streamed chunk upload rows; large half X02 |
| STORE-03 | L04 | `lane_checkpoint.rs` coherent capture + exact suffix |
| STORE-04 | L04 | moved-head rebase rows (gate-deterministic) |
| STORE-05 | L04 | corruption/truncation refusal (`codec.rs`, `lane_recovery.rs` corrupt chunk) |
| STORE-06 | L04 | verified-object rows (`lane_b_mem_store.rs` length-then-digest) |
| STORE-07 | L04 | envelope headroom/backpressure rows (`lane_checkpoint.rs`) |
| STORE-08 | L04 | foreign/unidentified cache rows (`lane_recovery.rs`), +P12 TS cross-origin |
| STORE-09 | L04 | retirement-rides-checkpoint rows (`lane_checkpoint.rs`, `admin.rs` retire lanes) |
| STORE-10 | X02 | bounded-memory checkpoint/restore of the >40 GiB fixture |
| LOCAL-01 | L04 | `lane_local_roots.rs` complete-then-register |
| LOCAL-02 | L04 | transactional release/resumed cleanup rows |
| LOCAL-03 | L04 | old-point evidence/new-lineage restore rows |

### GC-01..13 (chapter 21) — primary P05

| Child | Lanes | Anchors |
| --- | --- | --- |
| GC-01 | L04 | `lane_gc.rs::gc01`, +P12 `rep001_*` |
| GC-02 | L04 | `lane_gc.rs::gc02` (gate-paused writer), +P12 process-suspended variant |
| GC-03 | L04 | `lane_gc.rs::gc03` pinned root → release → collect |
| GC-04 | L04 | +P12 `adversarial-process.rs::a_kill_mid_sweep_*` (real crash-resume) |
| GC-05 | L04 | `lane_gc.rs::gc05` mid-collection roots |
| GC-06 | L04 | `lane_gc.rs::gc06` corrupt/foreign mark refusal |
| GC-07 | L04 | `lane_gc.rs::gc07` failed-delete durable progress |
| GC-08 | L04 | `lane_gc.rs::gc08` late-upload reconciliation |
| GC-09 | L04 | `lane_gc.rs::gc09` bounded pagination (same file) |
| GC-10 | L04 | `lane_gc.rs::gc10` stale collector refusals |
| GC-11 | L04 | `lane_gc.rs::gc11` capacity/stale-release |
| GC-12 | L04 | +P12 sweep-kill resume (durable progress across real death) |
| GC-13 | L04 | checkpoint/GC overlap rows (`lane_checkpoint.rs` epoch-move restaging) |
| — | X01 | every GC arm re-run against real S3 listing semantics in the X01 campaign |

### FS-01..05, S3-01..06 (chapter 21) — primary P05

| Child | Lanes | Anchors |
| --- | --- | --- |
| FS-01 | L04 | `local_ownership.rs::fs01*` (real SIGSTOP/death), `lane_b_interop.rs` mixed-fleet |
| FS-02 | L04 | `local_ownership.rs::fs02` kill-mid-replacement, `lane_b_fs_store.rs` per-phase faults |
| FS-03 | L04 | hostile lock/symlink/reserved-key refusals (`fs.rs`, `lane_b_fs_store.rs`) |
| FS-04 | L04 | key grammar + in-process competing open (`store.rs`, `lane_recovery.rs`) |
| FS-05 | X05 | power-failure rig (blocked; SIGKILL subset in L04 recorded as partial) |
| S3-01 | X01 | `s3_smoke.rs` conditional create/replace one-winner race |
| S3-02 | X01 | stale-version definite loss + verified immutable objects (same lane) |
| S3-03 | X01 | credential rotation/permission failure arms (F3 campaign + X06 IAM) |
| S3-04 | X01 | response-loss proxy/fault-injection arms |
| S3-05 | X01 | multipart/streamed snapshot handling, aborted uploads |
| S3-06 | X01 | deletion/restore + exact advertised bucket/service mode |

### REC-01..07, BACKUP-01..05, RESTORE-01..03 (chapter 22) — primary P05

| Child | Lanes | Anchors |
| --- | --- | --- |
| REC-01 | L04 | `lane_recovery.rs` cold hydration |
| REC-02 | L04 | receipt resolution on the fresh host, +P12 publication-kill reopen |
| REC-03 | L04, L11 | foreign/unidentified cache refusal; +P12 TS cross-origin attack |
| REC-04 | L04 | staging-invisible rows (`lane_recovery.rs`), +P12 `adversarial-process.rs::a_kill_during_hydration_*` (real mid-hydration kill + resume) |
| REC-05 | L04 | hold/root revocation rows (`lane_recovery.rs`, `lane_gc.rs::gc05`), +P12 `adversarial-process.rs::a_hold_revoked_mid_hydrate_refuses_whole_and_variants_converge` (REAL mid-hydrate revocation: SIGSTOP-frozen hydrate at a deterministic first-chunk-GET boundary, `HydrationHold` protects through a full collection, release + later collection reclaims, resumed hydrate refuses whole and typed, killed/resumed successors converge) — §8 debt 2 closed (authored, NotRun) |
| REC-06 | L04 | corrupt chunk → stopped tenant (`lane_recovery.rs`) |
| REC-07 | L04, X01 | replay-from-clean-directory rows; real-backend rerun X01 |
| BACKUP-01 | L04 | `lane_backup_restore.rs` backup under live root, destination-only restore |
| BACKUP-02 | L04 | interrupt/idempotent-retry/lost-ack rows |
| BACKUP-03 | L04 | independence (source destroyed) rows |
| BACKUP-04 | L04 | corruption/foreign-manifest refusal rows |
| BACKUP-05 | X02 | >RAM/>40 GiB backup/restore on the storage runner |
| RESTORE-01 | L04 | new-incarnation restore + old-scope refusal |
| RESTORE-02 | L04 | inspection-no-mutation + rewind refusal |
| RESTORE-03 | L04, X06 | outbox duplicate-delivery hazard row; deployed blob drill X06 |

### MIG-01..14, ERASE-01..04, OPS-TEST-01..02 (chapter 22)

| Child | Lanes | Anchors |
| --- | --- | --- |
| MIG-01 | L04 | `migration_execute.rs` whole-suffix one-target, `migration_hosted.rs` freeze rows |
| MIG-02 | L04, X03 | intermediate-law/expression boundaries; >RAM lane X03 (executor scratch seam: §8) |
| MIG-03 | L04 | `migration_abort.rs` fences, +P12 `adversarial-migration.rs` real kills |
| MIG-04 | L04 | `migration_plan_codec.rs` tamper matrix |
| MIG-05 | L04, X01 | `migration_hosted.rs` scripted CAS races; real S3 X01 |
| MIG-06 | L04, L14 | cross-version/foreign-family refusals (`migration_resume.rs`), 0.x fixture refusal (`ts/test/fixtures/legacy-store`) |
| MIG-07 | L04, L11 | label-is-never-identity rows (plan codec + `migrations-repo.test.ts`) |
| MIG-08 | L04 | `migration_resume.rs` reuse/no-overwrite, +P12 staging-kill resume |
| MIG-09 | L04 | activation-once/abort-race rows, +P12 killed-ReadyToSwitch arm |
| MIG-10 | L04 | hostile history-key bytes / `verify_chain` refusals |
| MIG-11 | L04 | initialization seeds exactly once (`migration_execute.rs`) |
| MIG-12 | L04 | exhausted-work typed refusal, later completion |
| MIG-13 | L04 | fused-vs-two-pass equality + bench `staged::eval_graph` oracle |
| MIG-14 | L04 | delayed-genesis-vs-tombstone races (`migration_abort.rs`/`migration_hosted.rs`) |
| ERASE-01 | L04 | `lane_erase.rs` facts-vs-history |
| ERASE-02 | L04 | tombstone + retained roots + later collection |
| ERASE-03 | L04, L11 | residual inventory + `admin.test.ts` erase report |
| ERASE-04 | L04 | scope distinction (local arm) |
| OPS-TEST-01 | L04, L11 | `lane_ops.rs`/`inspect.rs` condition fixtures + redaction; `history-open.test.ts` decode |
| OPS-TEST-02 | L04, L11 | starved-budget durable-progress rows; `close-report.test.ts`/`admin.test.ts` |

### API-01..12 (chapter 30) — primary P07 (P08 log side)

| Child | Lanes | Anchors |
| --- | --- | --- |
| API-01 | L10, L11 | `changes-lazy.test.ts` (mutation/rerun/getter/reentrancy), `command.test.ts` seal rows, `migrations-intent.test.ts` |
| API-02 | L10, L04 | `boundary-codec.test.ts` strict refusals, `canonical/tests.rs` shared domain, `command.test.ts` foreign-change refusal |
| API-03 | L04, L11 | witnessed/blind rows (`writer_local.rs`, API-03 stamp rows in `history_model`), `submit-certainty.test.ts` |
| API-04 | L11, L04 | `submit-certainty.test.ts` (interruption-after-publication, retained-ref), `close-report.test.ts` known-receipt+defect, +P12 publication-kill |
| API-05 | L04 | candidate pause/lose/reject visibility rows (`candidate_visibility.rs`, MemStore Gate schedules) |
| API-06 | L10, L11 | `id128.test.ts`, `identity.test.ts`, +P12 `rep004_*` |
| API-07 | L10, L04 | `result-pages.test.ts`, `result/tests.rs`, +P12 retained-results |
| API-08 | L10, L11, L04 | identity-boundary rows (`effect-core.test.ts` foreign refusals, `history-open.test.ts`, `lane_recovery.rs`), +P12 cross-origin |
| API-09 | L10, L14, X04 | `boundary-codec.test.ts` + regenerated float corpus cross-language |
| API-10 | L10, L11 | budget/cancellation rows (`changes-lazy`, `result-pages`, `submit-certainty`), pool cancellation lanes |
| API-11 | X02/X03 | identical answers/receipts over the large fixtures |
| API-12 | L10, L11, L12 | `effect-surface.test.ts` (no twins), `cookbook-doc`/`readme` doc tests, `composition.rs` + compile-fail, packed declarations (L12) |

### RUN-01..15 (chapter 31) — primary P06

| Child | Lanes | Anchors |
| --- | --- | --- |
| RUN-01 | L04, L11 | `tenants.rs::tests` distinct borrows, `borrows.test.ts`, session tests (L08) |
| RUN-02 | L08 | open/close interleavings (`runtime/tests.rs` directory lifecycle, session close idempotence) |
| RUN-03 | L10 | repeated open/close cycles with GC disabled (`effect-core` teardown rows), +P12 `adversarial-boundary.test.ts` retained wrappers; plateau measurement L16 |
| RUN-04 | L08, L10, L11 | revocation/drain rows (pool cancel, `close-report.test.ts`), +P12 close-under-load (TS + Rust) |
| RUN-05 | L04 | `local_ownership.rs` + `crash.rs` real subprocess arms, +P12 suspended-owner |
| RUN-06 | L04, L11 | cache-isolation rows (`store.rs` grammar, `lane_recovery.rs`, `borrows.test.ts` cross-origin), +P12 TS attack |
| RUN-07 | L08 | pool reservation/oversized-request/cold-open rows (`runtime/tests.rs`) |
| RUN-08 | X02 + X03 | both large-data halves (size AND enforced memory) |
| RUN-09 | L04 | disk/native fault injection (`lane_b_fs_store.rs`, store ENOSPC/short-write rows, `resize.rs`) |
| RUN-10 | L08, L10 | cancellation/fairness rows (pool tests, `runtime.test.ts` interruption, event-loop delay cells L16) |
| RUN-11 | L04, X01 | streamed checkpoint bounds (`codec.rs`, `lane_checkpoint.rs`); hosted stream faults X01 |
| RUN-12 | X06 | HTTP request-grammar drills in the deployed example (plus example unit tests when P13 lands them) |
| RUN-13 | X06 | auth/IAM binding drills (deployed) |
| RUN-14 | X06 | invocation lifecycle on actual Vercel/AWS hosts |
| RUN-15 | L04, L11 | generated-migration runtime rows (`migration_*.rs`, `migrations-flow.test.ts`, RuntimeExpectation checks), +P12 migration kills |

### FFI-01..08 (chapter 32) — primary P06

| Child | Lanes | Anchors |
| --- | --- | --- |
| FFI-01 | L08, L10 | reactor lifecycle tests, +P12 boundary forgeries; cycle plateau L16 |
| FFI-02 | L08, L10, L11 | wrong owner/kind/generation refusals (session tests, `native-roster.test.ts`), +P12 kind-confusion attacks |
| FFI-03 | L13, L08 | miri both targets + addon boundary tests (no C harness) |
| FFI-04 | L08 | parallel read/apply/close/cancel pool rows, panicked-job fault rows |
| FFI-05 | L10 | owned-values rows (`changes-lazy`, `result-pages`, SharedArrayBuffer/detached-buffer refusals in `runtime.test.ts`), +P12 retained-result |
| FFI-06 | L04, X04 | float golden/guard rows on every canonical target |
| FFI-07 | L10 | async-bridge rows (`runtime.test.ts` cancel-join, `effect-surface` no-twin pins) |
| FFI-08 | L12, X04 | native mismatch/handshake refusals in packed consumers on each platform |

### PKG-01..07B (chapter 32) — primary P13

| Child | Lanes | Anchors |
| --- | --- | --- |
| PKG-01 | L17, L09, L02 | fresh locked builds, dependency-lean checks (L03), provenance stamps |
| PKG-02 | L12 + P13 staging tests | interrupted pack/staging leaves source unchanged (P13 authored; L12 gate) |
| PKG-03 | L12 | tarball installs, one shared runtime, cross-package reuse |
| PKG-04 | X04 | artifact execution on the declared three-target roster |
| PKG-05 | L04, L10 | golden-store reopen/refusal (`legacy-store` fixture, `lifecycle.rs` family refusals, PKG version-family fixtures) |
| PKG-06 | L02, L12 | affirmative C-absence checks (banned tokens, workspace/CI/export sweeps — P13 lanes), packed export inventory |
| PKG-07A | X07 | staged-digest + private-registry rehearsal |
| PKG-07B | X08 | post-publication distribution proof (excused pre-promotion) |

### TS-MIG-01..10, APP-01..08 (chapter 33)

| Child | Lanes | Anchors |
| --- | --- | --- |
| TS-MIG-01 | L11, L04 | `migrations-flow`/`migrations-repo` determinism + native digest goldens (`migration_plan_codec.rs`) |
| TS-MIG-02 | L04, L11 | applied-history flattening/drift refusal (`migration_resume.rs`, `migrations-repo.test.ts`) |
| TS-MIG-03 | L04, L11 | initialization/baseline/no-shortcut rows (`migration_execute.rs`, `migrations-flow`) |
| TS-MIG-04 | L11, L04 | `migrations-diff.test.ts` intent/refusal matrix + `migration::compile` admission matrix |
| TS-MIG-05 | L04, X03 | bounded native execution/kill rows; >RAM half X03; +P12 migration kill arms |
| TS-MIG-06 | L04 | `migration_resume.rs` crash-boundary resumes, +P12 real-kill resumes |
| TS-MIG-07 | L04 | fused-vs-sequential equality (`migration_execute.rs` MIG-13 shape) |
| TS-MIG-08 | L04 | concurrency/ambiguity rows (`migration_resume.rs`, `migration_hosted.rs`) |
| TS-MIG-09 | L04, L11 | cutover/activation rows (`migration_abort.rs`, `admin.test.ts` activate/abort), +P12 activation-once |
| TS-MIG-10 | L11, L12 | tooling-boundary rows (`migrations-cli`/`-surface`/`-decode`), packed generate/check E2E over the real codec (L12 extension) |
| APP-01 | L10, L12 | server-only import walls (`declaration-isolation.test.ts`, packed bundles); deployed half X06 |
| APP-02 | X06 | auth/isolation drills (deployed example) |
| APP-03 | L10, L11, X06 | request-ownership rows (ManagedRuntime patterns in suites) + deployed concurrency |
| APP-04 | X04, X06 | actual-target execution |
| APP-05 | X01, X06 | real credentials/IAM attachment + rotation |
| APP-06 | X06 | host envelope measurements |
| APP-07 | X06 | deployment rehearsal with staged artifacts |
| APP-08 | L12, X06 | minimal-integration compile/run over packed artifacts |

### APP-perf + SPACE/HASH (chapters 40/41) — primary P14

| Child | Lanes | Anchors |
| --- | --- | --- |
| APP-FAST | L16 | `appperf` warm/cold cells + existing bench lanes |
| APP-MUTATE | L16 | post-write first-read cells (real alternating commits) |
| APP-NUMERIC | L16 | float workload cells + oracle-verified results |
| APP-LARGE | X02, X03, L16 | `largefix` fixtures + beyond-RAM cells |
| APP-TENANTS | L16 | tenant-churn cells with fd high-water evidence |
| APP-TARGETS | X04 | target-local calibration on the declared roster |
| APP-METHOD | L16 | measurement-plan method rules (provenance stamps, interleaved A/B, serialized lock) |
| APP-MAGIC | L16 | `docs/perf/magic-number-review.md` 22-row dispositions re-verified against measured results |
| SPACE-01 | L16 | `space::census` over `OwnedSnapshot::{entry_census,page_stats}` + matched SQLite census |
| SPACE-02 | L16 | `space::variants` matrix builds/measurements |
| HASH-01 | L04, L16 | role inventory tests + domain-separation rows |
| HASH-02 | L04 | forced-collision suites (`collision.rs`, `hashprobe::collision` over `collision-probe` constructors) |
| HASH-03 | L04 | sizing-math tests |
| HASH-04 | L16 | candidate probe (equivalence → KAT → timing) then the C12 decision |

### Parents G00–G16

Parents are conjunctions of their children's evidence (chapter 70): G00 ←
audit matrix + this manifest + ledger; G01 ← L02/L03/L05/L08/L09/L17 (+X04);
G02 ← E-CODEC/F-* rows; G03 ← E-ADMIT/judge rows; G04/G05 ← Q-* (+X02/X03);
G06 ← store/crash/resize rows (+X05 residual); G07/G09 ← PROTO/CONC rows;
G08 ← X01 + FS rows; G10 ← GC/REC/BACKUP/RESTORE/MIG/ERASE rows; G11/G12 ←
RUN/FFI/close rows; G13 ← PKG/APP (+X04/X06/X07); G14 ← hostile-input rows
(this packet's adversarial files + boundary fuzz); G15 ← L16 (+X-lanes);
G16 ← the complete ledger + checker. No parent may be recorded Passed from a
smoke subset.

## 8. Honest coverage debts (open, release-blocking until closed or descoped)

1. **Sink-internal spill (P03 boundary):** Q-BUDGET/Q-DISK intermediate
   distinct/group/rec state is not yet charged/spilled (P03.md records it).
   Until wired, those children's rows can reach at most partial evidence.
2. **REC-05 process arm — CLOSED (authored 2026-09-05, Verification:
   NotRun):** the hold-revoked-mid-hydrate REAL-process schedule now exists
   as `adversarial-process.rs::a_hold_revoked_mid_hydrate_*` (full name:
   `a_hold_revoked_mid_hydrate_refuses_whole_and_variants_converge`; child
   mode `hydrate-hold-revoked`, same re-exec harness with kill-and-reap and
   piped stdio). A hydrate driven through
   `recovery::open_hosted` is frozen at a deterministic first-chunk-GET
   boundary (stdin-gated delegating store — the FsStore Phase hooks cover
   mutations only — plus SIGSTOP), the `HydrationHold` named root is revoked
   via `admin::release_named_root_hosted`, and the arm asserts: the
   registered hold protects the closure through a full collection; after the
   release a later collection reclaims it and the resumed hydrate refuses
   whole and typed (no incomplete/wrong snapshot returned or adopted); the
   SIGKILLed variant adopts nothing; both successors converge on the
   identical complete current state. Execution remains F3's (lane L04; the
   §7 REC-05 row carries the anchor).
3. **Computed find terms from TS** (P07 boundary #2) leaves an API-12/Q-IR
   sliver unexercised from the TS side.
4. **Native log verbs** (`LogNative`) are unimplemented until the wave-C
   integration; L11's machine suites run against the double, and
   `adversarial-capabilities.test.ts` + `native-roster.test.ts` are the red
   forcing functions for the real surface.
5. **F-ENV forced wrong-rounding/FTZ host arms** are named empirical gaps in
   P11's bridge ledger; the oracle differentials plus X04 targets carry them.
6. **RUN-12/13/14 and APP-02..08** have no local executable half beyond type
   walls until P13's example lands; they are X06-blocked regardless.
7. **PKG-02/PKG-06 sweeps** await P13's staging/absence harnesses (this
   manifest routes them; P12 reviews, does not author them).
8. **FS-05** stays a stated substrate assumption unless the X05 rig is
   authorized (chapter 70 permits stating it plainly; the ledger row remains
   NotRun-with-authority-named, never silently NotApplicable).
