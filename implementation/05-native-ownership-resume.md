# 05 — Native ownership: next implementation packet

Date: 2026-09-04. This is a resumable work note for the selected
[`final-solution/`](../final-solution/README.md), not a competing proposal or a
completion claim. The foundations checkpoint may be committed independently.
**No production changes or new test runs are included in this note.**

## Observed failure and cause

The fresh built-package log test run recorded in
`/tmp/bumbledb-ts-log-tests-final.log` completed with **167 passed, 1 failed,
6 explicit S3 skips**. `test/tenants.test.ts:127` failed at line 144 with
`Missing expected rejection`: a second directory owner was accepted while the
first replica remained open. The test configured a 90 ms lease and waited
200 ms. The suite ran under concurrent integration load. This is a real
exclusion defect; an earlier source-condition pass does not qualify it.

The cause is structural, not the particular timer interval. TypeScript still
uses wall-clock expiry to mint a successor local owner. A delayed renewal or
paused event loop can outlive any chosen TTL. Raising the TTL, retrying until
green, or accepting takeover would not establish lifetime exclusion.

Current source anchors (line numbers refer to this working tree):

| Source | Observation |
| --- | --- |
| `ts-log/src/tenants.ts:93,161,229,304` | Expiring filesystem lease, renewal timer and tenant acquisition. The constructor starts temporary cleanup at line 170 before exclusion. |
| `ts-log/src/tenants.ts:153,386` | Eviction disposes the wrapper before deletion; pool shutdown releases the lease **before** disposing the replica. Shutdown does not join the `opening` map. |
| `ts-log/src/store.ts:384,521` | Expiry authorizes lease acquisition. The same helper also protects TypeScript filesystem-store mutations, a separate unresolved exposure. |
| `ts-log/src/replica.ts:846,915,1023` | Direct replica open performs recovery/native open/cleanup with no directory owner. A pool-only fix leaves this exported entry point unprotected. |
| `ts-log/src/replica.ts:1047` | Replica disposal sets `closed` and persists the sidecar; it does **not** close `core.db`. `persistSidecar` at line 276 contains no hidden native close. |
| `ts-log/src/writer.ts:921` | Writer-options open births the remote namespace before opening/excluding the local replica. An exclusion refusal must not perform that birth. |
| `ts/crate/src/lib.rs:250,262,504` | Native DB lifetime currently lives in `DbHandle`/`DbInner` and `Arc<Engine>`; `db_close` drops the handle's inner value. Actual active native work can retain resources independently. |
| `ts/crate/src/runtime.rs:71,97,346` | The shared bounded runtime registers operations, not persistent database owners. Completed outputs can be reclaimed during cancellation/close. |
| `ts/crate/src/runtime_wire.rs:37,498` | Runtime-handle drop begins drain; operation take transfers only bytes/null. There is no persistent owner transfer contract yet. |

## Reuse the existing primitive and authority

`crates/bumbledb-log/src/store/fence.rs:106` already provides
`acquire_directory`. It takes a one-shot kernel-held lock in the stable sibling
namespace `parent/~lease/name/owner.lock`, refuses supported symlink redirection,
and never expires or replaces the held lock file. Rust replicas already retain
this lock through native teardown. It is available with log default features
disabled, as already configured for `ts/crate`; no AWS feature or new locking
algorithm is needed.

Follow chapters [31](../final-solution/31-tenant-runtime.md),
[32](../final-solution/32-ffi-and-release-packaging.md) and
[35](../final-solution/35-effect-typescript-contract.md). Extend the existing
shared native runtime into the **one** owner/capability authority. Do not add a
second registry or a parallel lifetime counter beside it. The lock must belong
to the same actual native owner as its environment and active operations, not
to a loosely associated JavaScript wrapper or disposable operation output.

A standalone native lock handle is insufficient: reclaiming it when the
runtime closes could unlock a directory while a legacy JS-owned DB remains
live. Conversely, retaining a closed wrapper must not retain native resources.
Incomplete cleanup retains Closing ownership and exclusion; it is not success.

## Dependency-ordered bounded packet

### A — Shared native owner and lifecycle

Add persistent owner registration to the existing runtime and reuse the Rust
directory lock. Admission owns and bounds path bytes before worker dispatch.
Acquisition, error cleanup and final release use that executor, not a new
libuv path or blocking filesystem work on the JavaScript event loop. Exact
addon/runtime identity, kind/generation checks and typed busy/path/I/O/closed
failures must be established before mutation.

Bind the native environment, derived capabilities and active operations to this
same owner. Close stops admission, revokes idle capabilities, drains operations,
drops the environment, finishes permitted owned cleanup, then releases the
directory lock last. Concurrent close joins one transition. Runtime close must
include persistent owners and preserve their accounting when drain is incomplete.
Expose this only through the exact-version private core/log integration entry;
do not add ownership helpers to the public core barrel or another addon.

### B — Direct log opens and actual native teardown

Route direct replica open, writer-options open and every rotation/recovery path
through that ownership boundary. Acquire before recovery, cleanup, native open
or remote birth. Use a private already-owned path where necessary; do not
double-acquire the same namespace. Failure after native open closes that native
resource while retaining exclusion through cleanup. Replica close must actually
close its DB, including every retained/rotated environment and active operation,
before releasing the owner. Preserve primary failure and cleanup evidence.

### C — Tenant integration and regression replacement

Remove tenant TTL configuration, renewal timer, lost-lease state and pre-lock
sweep. Register opens before dispatch; shutdown closes admission and joins them,
and a late completion tears down instead of installing a slot. Each acquisition
returns an independent one-shot borrow tied to its exact slot incarnation, as
chapter 31 requires. Eviction/shutdown use the same native owner close, with
owned deletion before lock release. Do not preserve release-by-name as a second
lifetime authority.

Keep remote-token codecs and the distinct filesystem object-store protocol
separate. This packet does **not** qualify TypeScript filesystem-store mutation
exclusion merely by deleting the tenant timer; its expiry-based path remains an
explicit follow-up until replaced under the same selected design.

## Missing affinity constraints to resolve before wiring

The current runtime accepts independent `Send + FnOnce` jobs on interchangeable
workers. It does not yet provide an owner-affine session spanning multiple
operations. Legacy native DB wrappers contain `RefCell` state, and some native
operations still use `AsyncTask`; wrapping those calls in an Effect does not
enroll them in runtime drain or make their resources worker-safe.

Inventory every engine Arc, native snapshot and prepared/writer transaction that
can outlive a call. Prove the relevant Send/Sync and thread-affinity contracts.
In particular, a prepared LMDB writer cannot be moved between arbitrary workers
across a remote await. Keep thread-bound work on its owning worker or within one
native operation; do not invent unsafe Send, raw-pointer/FD bridges, or a shadow
counter to disguise missing ownership. The packet must state any still-unmigrated
legacy path rather than claim complete Effect/tenant-runtime conversion.

## Required evidence before claiming closure

- Deterministic child-process ready/STOP/CONT/KILL schedules: a paused owner is
  never displaced; completed close and process death permit the next owner.
- Rust/Node cross-language contention uses the same stable namespace. Refused
  second opens perform no tenant-state cleanup or remote birth. Rename/deletion
  cannot manufacture an independently lockable authority.
- Failed open, cancelled acquisition, close racing open, close during active
  work, rotation failure and cleanup timeout retain or release precisely the
  resources promised. Late opens never install after shutdown.
- Retained closed wrappers and disabled/delayed GC do not retain native DBs,
  mappings, FDs or locks. Distinct borrows, double release, stale slot identities
  and escaped child capabilities cannot close/use another owner.
- Exact fresh native artifact and packed core/log integration tests cover
  private export availability, foreign-addon rejection and one shared runtime.
  An older emitted binary is not evidence for the new bridge.

Coordinate production-file ownership and artifact rebuilds with the root and
SDK work before starting A. Root owns full-workspace, platform and release
qualification. No new qualification is claimed by this note.
