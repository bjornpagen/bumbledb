# 20 — The store is one contract

> **Decision.** The `ObjectStore` is one capability with one contract,
> shared by both drivers. Five verbs; outcomes are total sums including
> `Ambiguous`; a verb's *success* means **durable and visible**; keys are
> a grammar disjoint from temp/lock names; the mutation lock is a fenced
> CAS lease, never probe-then-unlink; a replica directory carries
> cross-process exclusivity and a refcounted handle.

This is the largest bug family in the corpus. It is large because the
store contract is where the protocol touches the physical world, and the
physical world is where a `bool` that should have been a sum, and a
"success" that did not mean durable, do the most damage.

## The current representation

### The lock is a probe-and-unlink over a mutable path

`putSwap` serializes under a pid-lockfile beside the key. Liveness is a
`bool`, and the two drivers compute it from opposite readings of the same
syscall:

- Rust `pid_alive` is `Ok(status.success())`, so `kill -0` returning
  `EPERM` (owner is another uid) reads as **dead** → Rust breaks a live
  lock (findings [2] [54] [65] [73]). TS `pidAlive` returns
  `code !== "ESRCH"`, so `EPERM` reads as **alive** → TS honors it. Two
  conforming drivers, one lockfile, opposite verdicts (finding [4]).
- The break itself is read-owner → `pidAlive` → `rm(lockPath)`: a
  TOCTOU. Between the probe and the unlink the owner can die and a *new*
  owner acquire; the contender then unlinks the fresh lock and two
  processes hold one key (findings [3] [8] [23]).
- `delete` unconditionally removes the key's lockfile, including a live
  `putSwap`'s (finding [19]); `put_swap` is check-then-rename while
  `delete`/`put_create` bypass the lock entirely, so a stale swap
  clobbers a fresh create or resurrects a deleted key (finding [20]); pid
  reuse, zombies, pid 0, and out-of-range pids wedge the lock forever and
  the two drivers disagree on the out-of-range arm (findings [22] [85]
  [124]).

### "Success" does not mean durable

`put_create` returns visible-before-durable: the object and its newly
created ancestor directories are not dir-fsynced, so an acked create can
vanish at power loss, and a checkpoint can durably reference a log slot
that reverted (findings [21] [27] [58]). `delete` never fsyncs, so a
sweep can persist while the manifest that authorized it reverts (finding
[21]). The TS checkpoint seed writes `data.mdb` with a bare `writeFile`
while fsyncing the sidecar that claims its vector, so power loss pairs a
durable chain with a torn mdb (findings [110] [132]).

### Keys and temps share a namespace

The synced-temp name collides with legal `StoreKey`s and with crash
litter under pid reuse, so an honest `put_create` surfaces a spurious
`EEXIST` on the infra channel (finding [81]); `put_create` answers
`Exists` when the destination is a *directory*, breaking the Exists→GET
loser algebra (finding [82]); temp litter is never swept in either driver
(findings [75] [86] [111]).

### The three store impls are three different stores

`memStore` hands out its **internal buffer by reference**, so a caller
mutating fetched bytes corrupts the stored object while its etag goes
stale — a divergence from `fsStore` and Rust's cloning `MemStore` that
lets store-parameterized tests pass on mem and fail on fs (findings [83]
[87]). S3 body-stream failures escape the `ErrStore` channel (finding
[84]); `object_store`'s internal retry blindly re-sends conditional PUTs,
turning a possibly-landed first attempt into a false `Exists`/`Moved`
(finding [88]); the S3 constructor accepts prefixes and rejects keys that
`fsStore`/`memStore` accept, and accepts `region: "auto"` without an
endpoint that TS refuses (findings [89] [90] [91]); the credential
refresh is memoized once, breaking rotation, and runs blocking I/O on
tokio workers (findings [26] [92]); the sync verbs panic via
`block_on` when called inside an async context (finding [29]).

### The handle is not a value with a lifetime

`tenants.get` can return an already-disposed replica whose directory was
deleted (finding [38]); concurrent gets open two replicas on one dir and
`sweepRotations` deletes each other's live store (finding [39]); the
LRU can evict and dispose the very replica it is returning (finding
[71]); there is no cross-process exclusivity on a replica dir, so a live
sibling's store is adopted or swept (finding [48]); the Lambda handler's
module-scope replica promise poisons the warm sandbox on a failed cold
open (finding [55]).

## The target representation

### 1. The lock is a fenced CAS lease, not a probe

Liveness stops being a `bool` and the lock stops being a path to unlink:

```
Liveness = Alive | Dead | Unknown          // EPERM ⇒ Unknown, never Dead
Lease    = { holder: WriterId, token: u64, expires: Ts }   // an object, CAS'd
```

The mutation lock is a `put_swap`-arbitrated lease object with a
monotonic fencing token and an expiry, acquired and broken **only through
the store's own CAS**, never through `read → probe → rm`. A contender
breaks a lease iff it is *expired* (a fact of the lease's own bytes, not a
probe of a foreign process), and every write carries its fencing token so
a stale holder's write is rejected by the CAS it no longer wins. `Unknown`
liveness never breaks a lease. This makes the entire family
unrepresentable at once: there is no `kill(0)` to read `EPERM` from
(findings [2] [4] [22] [54] [65] [73] [85] [124]), no path to unlink out
from under a fresh acquirer (findings [3] [8] [19] [20] [23]), and the
lease's identity is its token, so two holders cannot both be current.

### 2. A verb's success means durable and visible

`Created`/`Swapped` are minted **after** the fsync of the object and its
parent directory, on every impl, including newly created ancestors and
the checkpoint seed (findings [21] [27] [58] [110] [132]); `delete`
fsyncs the parent before returning (finding [21]). "Acked" is a durable
fact by construction; there is no window in which a returned success can
revert.

### 3. Outcomes are total sums, `Ambiguous` included

```
Create = Created(Etag) | Exists | Ambiguous
Swap   = Swapped(Etag)  | Moved  | Ambiguous
```

A conditional write whose result the transport cannot prove (S3 409,
timeout, a retried PUT) is `Ambiguous`, and the caller (the machine,
[10](10-protocol-machine.md)) resolves it with the GET-verify law — one
law, one place, both drivers (findings [24] [25] [28] [62] [66] [88]).
`put_create` against a directory is not `Exists`; it is the store
reporting a key-shape fault (finding [82]).

### 4. Keys are a grammar; temps and leases live outside it

`StoreKey` is a parsed grammar; the temp and lease namespaces are
*disjoint by construction* (a reserved prefix no `StoreKey` can spell), so
a temp can never collide with an honest key (finding [81]). S3 and fs and
mem accept exactly the same key set — the grammar is the one artifact
([60](60-codec-grammar.md)) — so control-char and prefix divergences
(findings [89] [90] [91]) are gone. Every impl reads a **fresh buffer**
out (mem clones like fs and Rust do), so aliasing (findings [83] [87]) is
not expressible. Stream failures wrap `ErrStore` on every path (finding
[84]); the async/sync boundary is explicit in the trait so a verb cannot
be called across it and panic (finding [29]); credentials are consulted
per request and off the worker threads (findings [26] [92]).

### 5. The handle is a refcounted lease on a directory

A replica directory has one owner at a time (a cross-process lease, the
same primitive as the mutation lease) so two live replicas on one dir is
unrepresentable (findings [39] [48]). `tenants.get` returns a *live*
handle whose refcount pins it against eviction and disposal for the
duration of the borrow (findings [38] [71]); a disposed handle is a
distinct type that every verb refuses, so `waitFor` into a disposed
replica is a type error, not an infinite poll ([10](10-protocol-machine.md),
finding [112]). The Lambda handler holds the replica as a *value*, not a
memoized promise that a failed cold open poisons (finding [55]).

## Conformance

The store smoke lane tests the one contract: it ties the create-race
winner's `Created` to the persisted bytes (finding [95]), cleans its
bucket and derives a collision-free prefix (findings [93] [94]), and
passes a correct-arity row so the round-trip can actually succeed
(finding [53]).

## The invariant

> **A stored object cannot exist without its body, cannot be acked
> without being durable, and cannot be locked by a probe of a foreign
> process.** The lock's identity is a fencing token, not a path; liveness
> has an `Unknown` arm that never breaks a lease; success is durability.

Dissolves: [2] [3] [4] [8] [19] [20] [21] [22] [23] [24] [25] [26] [27]
[28] [29] [38] [39] [48] [53] [54] [55] [58] [62] [65] [66] [71] [73]
[81] [82] [83] [84] [85] [87] [88] [89] [90] [91] [92] [93] [94] [95]
[110] [112] [115] [124] [132]. The outcome-algebra arms are consumed by
the machine ([10](10-protocol-machine.md)); the key grammar is defined in
[60](60-codec-grammar.md).
