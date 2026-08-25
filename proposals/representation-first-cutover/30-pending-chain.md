# 30 — Pending is a chain constructor

> **Decision.** The chain is a sum type — `Settled(vector)` or
> `Pending(vector, batch)` — that every reader must destructure. The
> `applied_pending: 0 | 1` integer and the `pending: Option<…>` side-flag
> die. The wholeness identity stops being an arithmetic guard bolted onto
> every read path and becomes *"the generation the chain type says,"* by
> construction.

## The current representation

The chain carries its per-braid vector, and *beside* it a `pending`
option and, at read time, an `applied_pending` count the reader adds in:

```
generation == chain.sum() + applied_pending
```

`applied_pending` is 1 exactly when a batch is applied-but-unpublished
and 0 otherwise. This is the billion-dollar mistake wearing a protocol
hat: a nullable field beside the real state, and a hand-added integer
every consumer must remember to fold in. The corpus is the list of
consumers who forgot:

- **`refresh_braid` forgot to resolve pending and skipped the wholeness
  check entirely**, merging a forked history into served reads — a
  critical (finding [5]).
- **The checkpointer forgot the pending guard** that `writer/duty` has,
  so it compacts a store whose `.mdb` generation is `sum + 1` while its
  own head vector sums to `sum`, and publishes a checkpoint whose bytes
  disagree with its own claim — a critical, reported three ways (findings
  [1] [59] [72]).
- **`waitFor` never re-checks the identity**, and steady-state refresh in
  TS never re-checks it either — the phantom detector Rust runs every
  pass, TS runs only at open (findings [42] [68]).
- The pending *resolution* itself is fragile because it is a procedure
  over a side-flag, not a fold over a type: the detached composite loses
  its one-by-one fallback when a competing drain resolves the backlog
  (finding [33]); the fallback aborts on the first `Err`, dropping the
  remaining acked segments (finding [34]); a retained backlog on a wedged
  braid blocks *every* braid's commits and can make the writer unopenable
  (finding [35]); a crash inside open's catch-up destroys a recoverable
  fsynced pending via an identity misfire (finding [36]).
- The **durability ordering** around the flag is wrong: `repairDiscard`
  and `openCore` persist `pending: null` to disk *before* the re-judgment
  completes, so a crash in the window drops the retained batch (finding
  [45]); the TS seed writes the store without fsync while the sidecar
  that counts it is fsynced (finding [110], durability twin in
  [20](20-store-contract.md)); TS `applySlot` advances and persists the
  chain *before* the publish-law refusal, fsyncing a sidecar that counts
  a slot whose effects never committed, where Rust pins that a refusal
  never advances (finding [120]).
- The **read arm** for the sidecar is a mess of `Option` and thrown
  errors: TS `readSidecar` returns `null` on *any* read error (EACCES,
  EIO) — an infra fault read as absence — while a parse throw escapes
  `open` instead of taking the discard arm; Rust separates io-fault from
  parse-discard but the split is hand-maintained (findings [31] [47]
  [131] [139]).

Every one of these is a reader or writer of `pending` that mishandled the
*seam between "in the chain" and "beside the chain."* There is no seam if
pending is *in* the chain.

## The target representation

### 1. The chain is a sum

```
Chain =
  | Settled { vector: Vector }
  | Pending { vector: Vector, batch: PendingBatch }
```

`Vector` is the per-braid `g` map ([60](60-codec-grammar.md) makes its
numbers exact). There is no `pending: Option`, no `applied_pending: 0|1`.
The generation a store must show is a **total function of the chain
value**:

```
generation(Settled{v})       = v.sum()
generation(Pending{v, _})    = v.sum() + 1
```

The wholeness check is `db.generation() == generation(chain)` — one
comparison, defined once, and *every* path that serves a read computes it
the same way because there is only one `generation` function. `refresh`,
`refresh_braid`, `waitFor`, the checkpointer, and `open` all take a
`Chain` and cannot read it without matching `Settled` vs `Pending` — the
match is the resolution. The forgotten check (findings [5] [42] [68]) and
the forgotten guard (findings [1] [59] [72]) are not omissions to catch
in review; they are matches the compiler demands.

### 2. The checkpointer takes `Settled` or refuses

Compaction's input type is `Settled`, not `Chain`. A checkpointer holding
a `Pending` chain cannot call compact — it is a type error, so the forged
checkpoint whose mdb generation exceeds its vector sum (findings [1] [59]
[72]) is unconstructible. The checkpointer resolves its pending through
the same `open` transition as everyone else ([10](10-protocol-machine.md))
before it earns a `Settled` to compact.

### 3. Resolution is a fold, not a procedure

`Pending{v, batch}` resolves by re-judging `batch` against the
winner-current state and folding the result into a new `Chain` — one
pure function returning `Settled` or a typed refusal, shared by the
detached publisher, the loss-path fallback, and open-recovery. Because
it is one fold over one type, a competing drain resolving the backlog
does not strand the one-by-one path (finding [33]); a mid-fold `Err`
returns the *remaining* segments as data rather than aborting (finding
[34]); a wedged braid's backlog is a marking on that braid, not a lock on
all of them (finding [35]); and the recovery identity is the same
`generation` function, so it cannot misfire and discard a recoverable
pending (finding [36]).

### 4. Durability is `Pending → durable → Settled`, in that order

The pending batch is fsynced *as `Pending`* before any apply; the
transition to `Settled` (or to a re-judged `Pending`) is written
*after* the re-judgment resolves. There is no write of `pending: null`
ahead of the resolution (finding [45]), and the chain never advances
across a publish-law refusal because advancing *is* the transition to a
new vector, which the refusal does not produce (finding [120]). The seed
mdb and its sidecar are one crash-consistent unit: the sidecar's
`Settled` vector is not durable until the mdb it counts is durable
(finding [110]).

### 5. The sidecar read is a total sum

```
SidecarRead =
  | Absent                 // NotFound only
  | Fault(io)              // EACCES/EIO/EMFILE: surfaced, never "absent"
  | Corrupt(parse)         // parse refusal ⇒ discard-and-re-pull
  | Read(Chain)
```

`Absent` is `NotFound` and nothing else, so an infra fault is never read
as "no sidecar" (findings [131] [139]); `Corrupt` routes to the
disposable-law discard instead of escaping `open` (findings [31] [47]);
parse is [60](60-codec-grammar.md)'s job and returns a `Chain` whose
numbers are exact.

## The invariant

> **The generation a store must show is a total function of its chain
> value; there is no addend a reader can forget, because there is no
> addend.** A checkpointer cannot compact a pending store, a refusal
> cannot advance the chain, and a crash cannot find `pending: null`
> written ahead of the truth.

Dissolves: [1] [5] [31] [33] [34] [35] [36] [42] [45] [47] [59] [68] [72]
[110] [120] [131] [139]. The wholeness check is run by the machine
([10](10-protocol-machine.md)); the number exactness and sidecar grammar
are [60](60-codec-grammar.md); the pending-vs-floor interaction (a slot
gc'd out from under a pending) is [50](50-retention.md).
