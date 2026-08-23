# 30 — One FsStore protocol

Rust and TS ship two dialects of one on-disk protocol: Rust arbitrates
create-only with `O_EXCL` temp + `link(2)` and CAS with flock on a written
`.etag` sidecar (crates/bumbledb-log/src/store/fs.rs:104-211); TS uses
`.locks/` pid-files, rename publication, and random-token etags
(ts-log/src/store.ts:83-236). Two dialects of one protocol is data
denormalization: a mixed fleet on one prefix corrupts create-only
arbitration, and deployment case 5's own story (the Rust checkpointer
beside a TS fleet) requires the mix. **One protocol, specified once in 40,
both implementations conforming to IT.**

## The protocol (each choice with its reason)

- **Create-only** = write to an `O_CREAT|O_EXCL` temp file, fsync the file,
  publish with `link(2)` to the final key path, fsync the parent directory,
  then report Created. Both languages have the primitive (`fs.link` in
  Node); link is chosen over rename because POSIX rename replaces an
  existing destination and therefore cannot arbitrate exclusivity — the
  audit's own finding on the Rust side, now the law for both. `EEXIST` on
  the link is the honest `Exists`.
- **Etag** = `blake3(content)`, lowercase hex, **computed, never stored**.
  The Rust `.etag` sidecar is written and never read — a second answer to
  a question the bytes already answer — and the TS random token is a third.
  Both die. `get` hashes what it read; `get_if_changed` compares computed
  etags; `put_swap` verifies the incumbent's computed etag under the lock.
  At protocol object sizes (a manifest line, checkpoint json) the hash cost
  is noise, and the TS side gained `internalBlake3` for exactly this class
  of need — the export's consumer roster now includes the store, which is
  what keeps it alive after the footprint dies (10).
- **The mutation lock** (put_swap's exclusivity) = an `O_EXCL` pid-lockfile
  beside the key, with dead-owner breaking: the lockfile body is the owner
  pid; a contender finding the lockfile probes liveness (`kill(pid, 0)` /
  `process.kill(pid, 0)`) and breaks the lock iff the owner is dead. This
  is sound precisely because 40 already rules **one machine load-bearing**
  for FsStore — pid liveness is meaningful on one machine and meaningless
  across machines, and the doc says so where the lock is specified. The
  lock is chosen as portable *data* over flock because flock is not in
  Node's std and lending engine FFI for a lock would grow the seam; with
  flock gone, the Rust crate's single `unsafe` block dies, and with it —
  unless another consumer appears during the pass — the `libc` dependency,
  returning `unsafe_code = deny` to a lint with zero allowances.
- **Durability discipline** unchanged: Created and Swapped return only
  after fsync of the object file and its parent directory (40's existing
  ruling).

## Deletions

Rust: the flock call, its `#[allow(unsafe_code)]` + SAFETY comment, the
`.etag` sidecar write path, and (expected) the `libc` dependency line.
TS: the random-token etag scheme, the rename publication, and the two-file
lock layout drift. Both: any code path that reads or reconciles the dead
sidecars.

## The two new conformance lanes

1. **Cross-language interop** — the lane that makes "one protocol" a fact
   instead of a sentence: Rust writes / TS reads byte-for-byte, TS writes /
   Rust reads, and both languages race one prefix concurrently with
   create-only exclusivity asserted (exactly one Created per slot across
   the mixed fleet, every CAS linearized, etags agreeing on every object).
   Rust side as a bumbledb-log test that shells the ts-log fixture (or the
   reverse); the mechanism is the lane author's, the assertions are not.
2. **Multi-process TS** — the single most load-bearing untested property
   for case 5: every TS contention test today races promises in ONE Node
   process. Mirror lane_b_fs_multiprocess's re-exec pattern (children print
   structured lines, the parent asserts hard): N real child processes over
   one FsStore prefix — disjoint content ⇒ every ack exactly once in a
   gap-free chain; a shared determinant ⇒ one winner and N−1 typed FD
   rejections; kill a child mid-commit ⇒ the fleet converges and the
   restarted process resolves its pending through the one recovery path.
   Runs on the unified protocol, after it lands.

## Case 5's compaction story, recorded

With one on-disk protocol, the Rust binary runs beside a TS fleet on the
same prefix: checkpoint and gc duty for TS-driver deployments = the Rust
writer/checkpointer as a sidecar process (a recorded deviation in 90's
receipts, with TS-native duty as the reopen trigger if a pure-TS
deployment ever refuses a Rust sidecar).
