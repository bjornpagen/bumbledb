# Native boundary, C ABI, packaging, and lifetime review

Audit date: 2026-09-04. Scope: `ts/src/native.ts`, relevant TypeScript `Db` ownership/marshaling, `ts/crate/src/{lib,log,marshal}.rs`, the C ABI ownership/read/write boundary, native package manifests, and package build/publish scripts. This is a selective deep review of boundary invariants, not a claim that every line of every FFI function has been exhaustively proven.

## SDK-013 — C database destruction leaks the native engine after any read

Priority: **P1**. Confidence: **confirmed static**. This audit did not build a new C library or run a standalone C reproduction.

Evidence:

- `crates/bumbledb-c/src/db.rs:73-78`: a `bdb_instance_ref` contains `engine: Option<Arc<Engine>>`.
- `:430-439`: the store-read constructor installs `Some(engine)`.
- `:1103-1119`: every `bdb_db_read` creates that engine-owning reference and retires it after the callback.
- `:454-457`: invalidation clears `alive` and the erased read pointer, but does not release the engine `Arc`.
- `:292-298`: `retire` pushes each box into an ever-growing vector.
- `:834-850`: `bdb_db_destroy` takes the retired vector, drops the database handle, and then explicitly leaks every retired box.
- `:300-310`: `leak_retired` uses `Box::leak` for instance, witness, and transaction boxes.

The intended guarantee is unusually strong: a stashed callback pointer should return `BDB_STATUS_MISUSE` even after the owning database is destroyed, rather than become a use-after-free. The current implementation buys that guarantee by permanently allocating every callback capability. Worse, a leaked read capability still holds an owning `Arc<Engine>`, so this is not merely a small tombstone leak: the database environment and exclusive lock can remain alive permanently after `bdb_db_destroy` reports success.

Minimal expected reproduction to add:

1. Create/open one C ABI database.
2. Execute one `bdb_db_read` callback, even an empty one.
3. Destroy it and assert `BDB_STATUS_OK`.
4. Reopen the same path in this process, then in another process.
5. Compare with the otherwise identical sequence that never calls `bdb_db_read`.

The source establishes the retained Arc and therefore non-release of engine ownership. The exact user-facing reopen error and physical resource count should be pinned by that regression test rather than assumed from this report.

This also creates an unbounded memory cost per read/write for a long-lived C consumer, even before destruction. Memory rises with all historical callbacks, not with live readers or plans. `bdb_witness` tombstones retain their witness payload as well, and heap-read references use a similar permanent-lifetime strategy (`bdb_owned_instance_read`, around `db.rs:1069-1081`).

Recommendation:

- At minimum, detach resource-owning fields when invalidating a capability. A dead diagnostic slot must not own a database, schema/instance graph, or store lock.
- Decide the actual C lifetime contract. Keeping every raw address diagnosable forever cannot have bounded memory without changing handle representation or limiting the guarantee.
- Prefer generation-tagged opaque handle IDs resolved through a bounded/reclaimable registry, or a conventional documented rule that callback references expire and must not be used afterward. If post-scope misuse detection is mandatory, design its reclamation model explicitly.
- Retaining witnesses should be an explicit owned operation with ordinary destruction. Borrowed callback witness diagnostics should not retain substantive data forever.
- Test stable memory/handle counts over millions of callbacks and open/read/destroy cycles, with both successful and aborted callbacks.

Existing tests such as `tests.rs:1085-1098` verify that a stashed pointer reports misuse after destroy. That is useful, but it currently codifies the symptom the tombstones were designed to support, not the complementary requirement that database destruction actually releases the database.

## What is working well at the boundary

The review found several deliberate protections that should survive refactoring:

- TypeScript read instances and transactions have explicit live/spent checks. The native bridge independently checks the erased callback pointer's alive token. A JavaScript type assertion alone does not bypass native scope invalidation.
- Writes use a callback-exit guard so throwing/abandoning a callback does not become a partially accepted write. The bridge distinguishes accepted, rejected, moved, and abandoned outcomes rather than flattening domain rejection into an exception.
- Prepared plans carry an owner identity and are not silently executed against a foreign store. Query ownership remains important when replicas rotate local engines.
- The C API translates errors to owned carriers with explicit destroy functions and has status-returning boundary guards. Its surface is materially more disciplined than exposing raw Rust errors or unconstrained pointers directly.
- The log codec and sealed braid decomposition now cross the Node boundary through a Rust `LogCodec`/descriptor handle. TypeScript constructs typed payloads; it no longer needs to maintain an independent binary parser for each protocol document.
- The TypeScript loader distinguishes a missing native package from one that exists but cannot load. This is substantially better than catching every loader error and suggesting a misleading reinstall.

These protections do not erase the ownership defects. In particular, “GC finalizers are reclamation only” is a good policy for correctness, but a bounded per-tenant host still needs deterministic reclamation of large engines, not only logical invalidation of small wrappers.

## Node/Rust boundary risks to make explicit

### Native lifetime and public lifetime disagree

`ts/crate/src/lib.rs:501-504` exposes `db_close` and `ts/src/native.ts:636-638` has a wrapper, while public `Db` intentionally has no close operation. The replicated layer works around that by inventing fresh directory names and leaving old engines to GC. This is SDK-007 in the hosting report. Fixing the C ABI leak alone will not fix the separate Node tenant-lifetime design.

### Synchronous native execution is a host scheduling choice

Create/open/admit/publish-to-file use asynchronous native tasks, but ordinary reads, queries, and writes execute synchronously through the Node bridge. A slow recursive query, wide result, expensive integrity judgment, or serialization of a large answer blocks the JavaScript event loop. While blocked, tenant lease renewal, request cancellation, and unrelated tenant requests do not progress.

This is not inherently wrong for an embedded API. It becomes a deployment contract: either queries have strict complexity/result limits, or expensive work belongs in workers/processes. A p50 point-read benchmark cannot prove p99 multi-tenant latency isolation. The driver and native query path currently do not offer a coherent public cancellation/work-budget surface; see SDK-010.

### Marshaling copies and ownership need one documented rule

The embedded `Db` API flattens collections into one row-major value array before crossing the bridge, avoiding a JavaScript array per row in that path (`ts/src/db.ts:84-125`). The log path instead builds arrays of rows, tags them for encoding, marshals into Rust-owned values, later lifts them back into facts, and crosses again for local apply. It therefore pays different allocation/copy costs and, as SDK-003 demonstrates, has different host-input ownership semantics.

Choose an invariant: after an API accepts a row/command, no caller mutation changes the meaning of an already recorded or encoded command. Measure performance against that invariant. Do not let “borrowed for a synchronous native call” accidentally become “borrowed across fsync and network awaits.”

### Structured errors should remain structured through every layer

The codec's refusal-kind roster is a strong start. Some higher-level errors still collapse into human messages and host-specific wrappers; open errors from native tasks and stale-handle failures have different shapes from log refusals. Application retry decisions should use a documented stable classification for transient infrastructure failure, uncertain commit, permanent corruption, programmer misuse, schema incompatibility, and admission rejection.

Do not make an application parse explanatory prose to decide whether retrying may duplicate a command. Carry structured provenance—operation ID, tenant, attempted slot/vector, publication certainty, error family—through the bridge and driver. Avoid leaking sensitive fact payloads in global logs by default; typed rejection values may contain tenant data.

## Packaging and release engineering

### Positive evidence

The current build scripts deliberately pin a finite published platform roster: darwin-arm64, linux-arm64, and linux-x64. The main package's packed manifest gets exact-version native optional dependencies. Build-time checks compare the workspace, engine, Node bridge, C ABI, platform packages, and log package versions. Packed-import/declaration-isolation checks exercise the actual tarball, not only the source checkout. These are valuable defenses against a common native-SDK failure mode: a package that works only because a monorepo path or dev dependency happens to be present.

The audit preserved the existing uncommitted changes to these scripts/manifests. It did not run `build`, `pack`, `publish`, or any version-injection script.

### Release risks and recommended gates — not all are current bugs

1. **Runtime ABI/version assertion.** `ts/src/native.ts:617-633` loads whichever platform package resolves and returns it without checking that `engineVersion()` matches the JavaScript package version. Packed exact dependencies are the main protection, but linked development packages, manually installed artifacts, bundlers, or bad publication can bypass it. Add a cheap explicit compatibility check before making unsafe assumptions about export signatures. Treat this as hardening; this audit's installed native artifact correctly reported 0.20.3.

2. **Prepack mutates the source manifest.** `ts/scripts/pin.ts` injects optional dependencies into `package.json` and relies on `postpack` to remove them. Process interruption or failed packaging can leave the repository manifest modified. `restore` deletes the entire field rather than restoring an exact previous value, so it assumes the repository will never acquire unrelated optional dependencies. Document the assumption and prefer packaging from a staging manifest. Never run that publish ceremony inside an audit/diagnostic path expecting a read-only checkout.

3. **Build/test coupling is expensive and mutating.** Both package `test` scripts begin with `build`; the main build removes/recreates `dist`, cleans the Node bridge's release artifacts, compiles native code, copies binaries, links package directories, and performs a pack proof. Keep that full release gate, but also provide an explicitly non-mutating source test command. Otherwise ordinary investigation risks regenerating user work or conflating stale-artifact tests with source tests.

4. **The Linux compatibility envelope is narrower than `os: linux`.** The ARM64 package explicitly states Amazon Linux 2023/glibc 2.34. Test the actual supported runtime images, CPU features, and libc floor. An Alpine/musl host is not covered by a glibc artifact just because `process.platform` says linux. Keep unsupported-platform errors precise and publish a compatibility matrix; do not claim universal Node >=24 support from the engines field alone.

5. **Source artifact versus packaged artifact coverage.** Source-condition tests are essential for reviewing current edits, but they cannot prove a tarball contains every required file or that published platform packages match. Keep both gates, with explicit provenance in CI output: git revision, dirty-state indicator, engine version, protocol/storage format, OS/arch/libc, and package tarball digest.

6. **Log peer range versus protocol compatibility.** The current log peer dependency is `^0.20.3`; native dependencies are exact at pack time. Define which patch changes may alter protocol identities, exported internals, checkpoint schema, or storage behavior. A semver range is not a protocol-compatibility proof. Add cross-release reopen/replay tests and downgrade refusal tests around the supported compatibility policy.

7. **Documentation must name the running implementation.** `ts-log/README.md` still describes a pid-liveness lock protocol, while `src/store.ts` implements expiring `LEASE/1` tokens. The README also names an older peer version and receipt vocabulary (`generation` versus the current `slot`). The Lambda example is explicitly pinned to older registry packages. These are not cosmetic when operators use them to reason about stale owners, crash recovery, or session tokens.

## Tests executed and limitations

No C ABI build or fresh native regeneration was performed. This review used the existing local Node artifact reporting `bumbledb-node 0.20.3 (bumbledb storage format v8)` and read the current Rust sources. The source-specific TypeScript tests and focused reproduction results are recorded in `30-sdk-hosting.md`.

Selected existing suites passed: 132 TypeScript-log tests plus 77 core SDK/Node-FFI tests, 209 total. The latter includes callback fault/scope accounting, foreign ownership/key reads, marshaling, native loading, and ordinary write/read outcomes. These tests do not exercise the C `bdb_db_destroy` Arc leak.

Exact commands and SDK reproduction sources/outputs are preserved in [32-sdk-test-evidence.md](32-sdk-test-evidence.md).

The C read/destroy ownership defect is established from the retained/leaked `Arc`, not from a sanitizer run. Sanitizers, a C callback stress harness, cross-thread misuse tests, failed-open leak checks, and process-level lock-release tests remain required. No statement here asserts that arbitrary C caller memory is safe: the ordinary C ABI rule that incoming pointer/length pairs must describe valid memory still applies.

## Boundary philosophy

A small API is not merely an API with few method names. It is an API with few independent rules an application must remember. Today an application must remember that database handles do not close, replicas do close but writers can still publish, tenant `release` is not replica disposal, byte arrays are live across asynchronous work, and callback recording can rerun during ID refill. Those rules undermine the elegant relational core.

The best next step is not to add more wrappers indiscriminately. Give each resource one clear owner, each command one immutable meaning, each outcome one publication status, and each public read one declared visibility frontier. Then make the SDK vocabulary express those facts directly.
