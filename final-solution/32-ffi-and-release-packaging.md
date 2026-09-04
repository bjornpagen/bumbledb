# 32 — A small native boundary that really closes

Status: proposed 1.0 design. This is an incompatible replacement for the current C callback-pointer/tombstone contract and GC-owned Node database lifetime. No implementation, sanitizer or release-packaging pass is claimed here.

## Package boundaries

Keep `bumbledb` as the public Rust/TypeScript/C facts/laws/query/LMDB core. Keep **`bumbledb-log` as a TypeScript-only public product**, backed by an internal Rust implementation of named command history, generated history IDs, publication, backup/restore and checked migration execution. Do not ship or support a public Rust log SDK/crate API, C log API, or `bumbledb_log.h` for 1.0. Internal Rust protocol crates may remain in the repository with private implementation exports; public compatibility commitments attach to the TypeScript contract, stored protocol and tested native bridge, not every internal Rust symbol.

The Node native bridge may be built with a log feature; this does not make the Rust core depend on the log. Core-only consumers can build without AWS/log dependencies. Native feature support is reported in the ABI descriptor and checked before using an internal log export. Do not create independently compiled Rust engines that exchange internal pointers across separate dynamic libraries. The TypeScript migration subpath contains repo-file tooling and explicit orchestration; durable authority remains native.

For C, publish one versioned **core-only** C ABI library and `bumbledb.h`. It contains no command history, S3, backup/migration or log identity functions. Node's internal history bindings are not routed through a newly supported C log facade. A Node process loads one compatible native runtime for its core/log owners; separately loaded core C libraries never exchange raw engine pointers with it.

This is feature selection, not a native plugin architecture. No arbitrary third-party execution, allocator or storage-driver plugin registry is required for 1.0.

## The C lifetime tradeoff, decided explicitly

The current API tries to diagnose a raw callback pointer after scope exit and even after database destruction. It preserves every pointer address forever through leaked tombstones. Those tombstones also retain engine ownership. This cannot be repaired into a bounded design while keeping every raw pointer eternally dereferenceable.

Replace core database/snapshot/delta-builder/session/result capabilities with **by-value generation-tagged tokens**, resolved through a bounded runtime-owned slot table. No callback-scoped Rust reference pointer crosses C. This removes lifetime-erased `ReadInstance`/transaction pointers from the ordinary public ABI and eliminates the historical tombstone list.

```c
typedef struct {
    uint64_t runtime_id;
    uint64_t slot;
    uint64_t generation;
} bdb_handle;

typedef struct bdb_runtime bdb_runtime;
typedef uint32_t bdb_status;

bdb_status bdb_runtime_create(const bdb_runtime_options *, bdb_runtime **out);
bdb_status bdb_db_open(bdb_runtime *, const bdb_open_options *, bdb_handle *out);
bdb_status bdb_db_snapshot(bdb_runtime *, bdb_handle db, bdb_handle *out);
bdb_status bdb_db_apply(bdb_runtime *, bdb_handle db, bdb_handle changes,
                      const bdb_expected_generation *, const bdb_work_options *,
                      bdb_apply_outcome *out);
bdb_status bdb_handle_close(bdb_runtime *, bdb_handle);
bdb_status bdb_runtime_close(bdb_runtime *, const bdb_close_options *,
                            bdb_close_report *out);
bdb_status bdb_runtime_destroy(bdb_runtime **);
```

Signatures are proposed shapes, not a compilable final header. The generated normative header must define error carriers, null rules, threading rules and ownership for every argument. The conceptual interface has no write/read callback whose lifetime must be tombstoned.

The slot table validates runtime ID, slot, generation and resource kind before obtaining an operation lease. Closing a handle removes its owned resource and increments the slot generation. Reusing the slot does not make an old copied token valid. Closing an already-closed token of that same runtime returns the documented idempotent closed status; other operations report `ClosedHandle`. A wrong resource kind reports `WrongHandleKind`, not a cast.

Slot capacity is a configured finite maximum, with explicit `HandleCapacity` refusal. `runtime_id` and slot generations never wrap or recycle into a previously valid identity: on exhaustion refuse further allocation. This remote theoretical exhaustion is a checked failure, not an immortal per-call allocation. Freed slots and capacities are measured in stress tests. There is no unbounded registry of historical callbacks.

The runtime itself remains an ordinary C-owned pointer with an ordinary lifetime: callers must not use it after successful destroy, and all incoming pointer/length pairs must describe valid readable/writable memory. No ABI can prove arbitrary caller memory valid. `runtime_destroy(&runtime)` requires completed close, frees the runtime and sets that supplied pointer to null; a copied stale runtime pointer is still caller misuse outside the safety contract. This honest rule replaces the false ambition to make arbitrary expired raw pointers safe forever.

Copied stale **tokens**, used with a still-live runtime, are safely rejected. Foreign-runtime tokens are rejected even if slot/generation numbers happen to match. Tokens are not security credentials against hostile native code already executing in the same process.

### What close releases

Core C `db` close revokes child snapshot/session capabilities, stops admission, drains active operation leases, drops actual native environments and releases directory locks last. Node history close follows the same owner mechanics without a public C log handle. Neither waits for unreachable token copies. A result containing copied data or independent sealed scratch can remain valid after its source database closes; it has its own explicit handle/resource budget. Runtime close closes all of these resources.

An in-flight operation cannot race a slot removal into use-after-free: lookup obtains a scoped owning operation lease before releasing the registry lock. Close marks the resource closing, prevents new leases and waits for existing ones. The table never holds its lock through a database operation or a user callback.

Close deadline failure returns `CloseIncomplete`; the runtime stays live and closing. `destroy` refuses while native tasks/resources remain. Forced process termination is a host policy for an unresponsive kernel call, not an undocumented destructor behavior.

## C data ownership and zero-copy limits

Inbound pointer/length slices are borrowed only for the duration explicitly declared by the function. Core changes are copied and validated before the call returns; queued/asynchronous work never borrows caller memory. Builder acceptance cannot keep a C stack pointer for later application.

Outbound row data belongs to an owned `CompleteResult` or page handle. A `bdb_bytes_view` or `bdb_string_view` into that result remains valid only while the result/page is live and not concurrently closed. Closing a page invalidates its views. A caller needing independent lifetime copies bytes. `intoCursor` consumes/spends the result token and transfers backing-store ownership to a new cursor token; closing the old token cannot close the cursor. There is no clone/shared-cursor API and no claim that a retained raw view is diagnosable after its backing result closes.

For asynchronous C use, choose a small explicit operation handle with `poll/wait/cancel` and typed result retrieval. It is the same bounded runtime task as Node/Rust async use. Do not call foreign completion callbacks while holding native locks, and do not retain arbitrary callback closures in the engine. Blocking C methods can be thin wrappers over this operation path or direct execution using the same work context. A process must not use inherited active runtimes after `fork`; create runtimes after the child has executed a fresh program. Report a wrong-process handle where detectable, without promising to repair arbitrary forked native-library state.

Bulk rows use one flat canonical cell array with explicit row count and arity. Zero-arity relation handling is specified rather than inferred by dividing the number of cells. Validate multiplication/size arithmetic before allocation and accept/reject null+zero lengths consistently. No undocumented difference between generated typed facts, dynamic rows and wire import is permitted.

Errors use owned bounded error detail with one free operation or a token in the same runtime table. Error strings never borrow temporary Rust formatting buffers. Outputs are initialized to an empty/invalid state before work; failure cannot expose a half-initialized handle, partial current answer or abandoned engine reference.

## Float and identity transport

The C scalar tag roster adds `F64` with an input `double`. A checked bit-image constructor/accessor supports exact codec tests and applications that need canonical wire bits. The ABI does not expose Rust enum layout or Rust's `F64` struct. Unknown numeric tags refuse before reading unrelated fields.

The schema boundary canonicalizes all NaNs to `0x7ff8000000000000` and both signed zeros to `+0`. Every arithmetic node canonicalizes its output too; for example `1 / neg(0)` is `+Infinity` under this value algebra. Total ordering, equality, keys, hashing and deterministic aggregates follow the engine chapter, not C's unordered IEEE comparison predicates. Integer conversion requires explicit checked casts.

The engine establishes/restores its required floating-point environment at a foreign-thread entry boundary: round-to-nearest ties-to-even, gradual underflow, and the required exception/mode behavior. A C host's rounding mode or FTZ/DAZ state must neither change database answers nor be silently left altered when the call returns. The guard is nested/reentrant-safe and tested on each supported architecture. Unsupported environment control is a platform qualification failure, not permission to claim deterministic results.

Log `EntityId` is an opaque 28-byte value with explicit canonical endian encoding, not an internal Rust struct whose padding enters a digest. The core only sees ordinary fixed bytes/newtypes. Node exposes a branded canonical 56-character lowercase hex string; it is not a JS `number`, an unsafe 224-bit arithmetic imitation, or a callback-reserved provisional ID. Its embedded incarnation is birth provenance: restored old-born entity bytes remain valid application values, unlike old command/owner/witness authority tokens. Core C can carry the same bytes as ordinary schema-defined values but has no log ID issuance or accessor product.

JSON/export codecs encode canonical `f64` bits explicitly. Native Node `number` ingress/egress remains ergonomic; JS's `JSON.stringify` NaN/infinity behavior is never used as the database serialization rule. The scalar golden corpus traverses core C/Node/Rust, storage, query parameters and result pages; the log command/receipt corpus traverses public TypeScript and internal Rust.

## Node ownership and scheduling

Node wrappers own opaque native capabilities plus a spent/closing state, not raw erased callback pointers. Their public `close`/`Symbol.dispose`/`Symbol.asyncDispose` match the resource lifetime. Finalizers may call idempotent close as a backstop; tests do not require a finalizer to run.

The Node wrapper cannot keep a closed engine alive merely by retaining a stale `Db`, replica, writer, plan or tenant borrow object. Close removes the native resource from its owner. A JS object may retain a small inert token; it cannot retain the engine `Arc`, mapped file or environment lock.

Async native tasks own immutable command/query inputs and report typed outcomes. They execute on the bounded runtime workers from chapter 31. Cancellation cannot be implemented solely as a rejected JS promise while native work continues unnoticed. The caller can inspect/drain an interrupted operation; unknown publication retains its `CommandRef`.

Immutable schema templates are shareable. Execution sessions and their native retained caches are explicit owned resources, bound to physical catalog/snapshot identity and accounted. No GC-only prepared plan secretly owns unlimited memo data. Result conversion is bounded by pages; local or hosted APIs never automatically flatten arbitrarily large answers into JS arrays.

Expose the complete stable error vocabulary, including errors from async open tasks. Do not drop typed engine error family into an unclassified message at an asynchronous boundary. JS error causes may preserve provider information, but retries depend on stable codes and publication certainty, not message parsing.

## ABI and artifact policy

Rust's ABI is not stable and is not exported. The supported native contracts are versioned **core C** and the exact-version Node N-API bridge used by the public TypeScript packages. There is no public Rust/C log API compatibility matrix. Pin the Rust nightly toolchain and lockfiles in release provenance; nightly is welcome internally, but a compiler experiment cannot quietly change the wire format or exported ABI.

At initial load, a small fixed-layout `abi_info` export reports:

- ABI major/minor and exported feature bitmap;
- package version and build revision;
- engine storage format and supported read/write codec versions;
- log protocol/command codec version when enabled;
- target architecture, libc/minimum OS, baseline CPU features and N-API level where applicable.

The bootstrap descriptor layout itself is stable for ABI major 1. Callers check it before invoking version-dependent exports. Core and log JS packages use exact compatible native versions in the packed manifests; runtime compatibility is still checked, covering linked packages, manual binaries, bundlers and incorrectly published artifacts.

Within 1.x, new enum/status cases are additive only where callers are required to handle unknown values; existing tag numbers and meanings do not change. C option structs carry `struct_size` and version fields, with documented zero defaults and bounded copying. Do not serialize raw `repr(C)` structs. Protocol/storage version compatibility is independent of package semver; an accepted version range is not replay evidence.

Incompatible artifact/format combinations fail before opening/mutating a store. Patch releases cannot silently change canonical float equality, schema hashing, ID encoding or receipt meaning. Upgrade/downgrade support must be declared and demonstrated by fixtures, not inferred from matching symbol names.

### Initial supported matrix

Keep the finite current target family and qualify it precisely:

| Target | 1.0 qualification policy |
| --- | --- |
| macOS arm64 | Proposed minimum macOS 14; qualify macOS 14 and the current supported release, including native close/lock/mmap/FPU behavior |
| Linux arm64 glibc | Build and run on the chosen minimum image; initially Amazon Linux 2023/glibc 2.34 is a defensible explicit floor, not universal Linux support |
| Linux x86-64 glibc | Same explicit libc floor; baseline CPU compatibility separately tested, optional accelerated paths runtime-detected |
| Node | Node 24 LTS minimum and Node 26 explicitly qualified against actual artifacts; no untested open-ended major-version promise. N-API compatibility alone is not full integration coverage. |
| musl/Alpine, Windows, macOS x64, 32-bit | Not claimed unless a separate build plus complete applicable conformance/lifecycle matrix is added |

This table is a proposed initial support policy, not a claim those platforms were tested in this phase. Changes to its selected minima require updating the matrix and running the corresponding qualification. The release publishes exact tested runtime versions and artifact digests. No artifact may advertise `linux` so broadly that the loader quietly attempts glibc code on musl and emits a misleading missing-package message.

CPU-specific SIMD is optional acceleration with a verified baseline path: x86-64 baseline/SSE2 and ARMv8-A for the named 64-bit ARM targets, subject to the chosen OS ABI. Explicitly test illegal-instruction avoidance and f64 equivalence. Do not compile universally shipped binaries with the release builder's native CPU feature set.

## Packaging should not edit the source checkout

Build from a controlled source revision into an isolated staging tree. Generate packed manifests there, inject exact native pins there, and retain unrelated optional dependencies exactly. `prepack`/`postpack` must not rewrite the developer's source `package.json` or rely on a post-hook running after process interruption.

Keep source-test, native-build and release-pack gates distinct. A source-test command must not silently clean native output, regenerate declarations or pack/publish packages. A release-test command intentionally builds fresh artifacts and verifies the tarballs. Both report the artifact actually used; testing current TS against yesterday's native binary is useful only with that limitation stated.

Release inputs include the root workspace plus the separately built Node and C bridge crates. The root workspace test pass does not cover crates excluded from that workspace. Build/lint/test feature matrices must enumerate them explicitly.

Release qualification produces immutable staged tarballs/libraries, hashes, the core C header, TypeScript declarations, provenance and a test report. **Before promotion**, install those exact tarballs in empty projects and a disposable/private staged registry; verify pins, file allowlists and simulated partial-publish recovery. None of these executable pre-release gates depends on a version already existing in the public registry.

Authorized publishing consumes those exact tested files rather than rebuilding a new untested binary. Publish platform artifacts before their main package, verify availability/version/digest, then publish the main/log packages. **After promotion**, download the actual public-registry artifacts, compare their digests and perform clean remote installation. Distribution verification is mandatory before declaring the release complete; failure is a release incident, not a retroactive green qualification report. A failed partial release remains diagnosable and retryable; do not edit source manifests to pretend it completed.

This proposal authorizes no actual publication. The implementation campaign must separately obtain/run the normal release authorization and credentialed environment checks.

## Required native and release gates

| Gate | Required assertion |
| --- | --- |
| FFI-01 Lifecycle | C and Node create/read/query/close/reopen, with references/tokens retained, millions of bounded handle cycles and no GC. No permanent engine Arc, lock, mapping, FD or per-callback tombstone remains. |
| FFI-02 Token safety | Wrong runtime/kind/generation, double close, slot reuse, capacity/exhaustion, stale tokens after DB close, close during operation lookup. Typed refusal without wrong-resource access; bounded registry memory. |
| FFI-03 Boundary memory | ASan/LSan/UBSan C harnesses plus applicable Rust Miri tests, fuzzed valid-memory pointer/length/tag inputs, overflow/null/zero-arity and allocation failure. No unwind crosses C; all outputs valid/empty on failure. |
| FFI-04 Threads | Parallel read/apply/close/cancel and wrong-thread uses according to documented contract; deadlock/reentrancy/poison paths. Run race tooling where supported; no freeing an active operation's data. |
| FFI-05 Views | Close/reuse result page only after views' documented lifetime; legal copied data survives source close. Borrowed input mutation after acceptance cannot affect asynchronous work. Illegal raw-pointer use is not misrepresented as a supported diagnostic feature. |
| FFI-06 Floats | Golden bit corpus, host rounding modes, FTZ/DAZ, foreign thread entry/nesting, register state restoration, overflow/underflow/NaN/zero arithmetic and aggregate order/spill equivalence. Same canonical result across all supported targets. |
| FFI-07 Async bridge | Queued cancellation, active cancellation, JS throw/worker failure, close with unresolved submit, native open errors and bounded result conversion. No swallowed error kind or hidden forever-running task. |
| FFI-08 ABI mismatch | Wrong major/minor/features/N-API/native package/version/format/codec/target/libc. Fail before store mutation, with precise present-but-incompatible versus missing-artifact errors. |
| PKG-01 Reproducible inputs | Fresh locked builds of core, internal Rust log, Node core/log and core C at pinned nightly, including no-log builds and all advertised features. Provenance names commit, dirty state, toolchain, target, flags, dependency locks and digests. |
| PKG-02 Clean staging | Interrupted manifest generation/build/pack at each phase leaves source tree unchanged. Existing unrelated optional dependencies survive. Staging retries yield equivalent intended manifests/artifact set. |
| PKG-03 Isolated install | Actual tarballs installed in empty projects, without monorepo paths/dev dependencies/source condition aliases. ESM imports, type declarations, native load and core/log examples execute. Test supported npm/pnpm install flows and disabled optional dependencies error. |
| PKG-04 Native matrix | Actual artifact boot/run on every advertised OS/arch/libc/Node row, minimum CPU and optimized paths. Unsupported hosts refuse explicitly; Node 24, additional advertised majors and Lambda image are genuinely executed. |
| PKG-05 Cross-release data | Golden 1.0 stores/history/commands/receipts reopen/replay on supported upgrades. Incompatible 0.x/downgrade cases refuse read-only and direct users to log import, never mutate before discovering incompatibility. |
| PKG-06 Public contract | Generated core C header and core/log TS declarations match exports and error tags; documentation snippets compile; standalone downstream custom-schema and dynamic API tests agree. Core dependency graph has no log/backup/migration types; no public Rust/C log SDK/header is packaged or advertised. |
| PKG-07A Pre-promotion proof | Tested tarball/library digests equal immutable staged artifacts; empty-project and disposable/private-registry installs resolve exact native pins. Simulated partial publication and package-size/file allowlists pass before public promotion. Missing required qualification access is incomplete, never a silent pass. No production publication is performed by this proposal phase. |
| PKG-07B Distribution proof | After separately authorized public promotion, actual registry-downloaded digests equal tested digests and clean remote installs resolve exact pins. Failure blocks declaring the release complete and triggers a release incident; it cannot rewrite pre-promotion test results as passed distribution evidence. |

Run all known SDK/FFI counterexample schedules against freshly built artifacts, not only source wrappers. Source suites, packed artifacts and native lifecycle tests are complementary, not substitutes. Platform-specific required tests must appear as explicit failed/incomplete qualification if skipped; release is blocked for a claimed platform until they pass.

## Evidence and issue closure

This design directly replaces `SDK-013`'s immortal C tombstones and `SDK-007`'s GC-owned native environment. It also closes the representational opening for `SDK-002/004/005` only when the runtime gates pass. Existing read-scope and foreign-plan protections must be preserved under the new token model; removing callbacks does not authorize cross-owner execution.

The original unindexed packaging observations—unchecked runtime ABI, source-mutating prepack, build/test coupling, Linux/libc ambiguity, stale examples and unproven cross-release compatibility—map to PKG-01–06, PKG-07A/B and FFI-08. Preserve audit references and actual failing traces in the implementation's resolution ledger. A proposal, a compile pass or a new semver number is not closure evidence.
