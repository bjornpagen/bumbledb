# 10 — The protocol is one machine

> **Decision.** The replica/writer/checkpointer *behavior* is one
> transition table, expressed as data, executed identically by both
> drivers. Every arm — per-braid wedge, gap→reseed, the ambiguous-create
> retry law, the id-lease algebra, the deposition signal, the scream — is
> a **state or an outcome value**, not a code path one driver happened to
> write and the other happened to forget.

## The current representation

The protocol lives as English in [50-replica.md](../50-replica.md),
[60-writer.md](../60-writer.md), and the checkpointer prose. It was
hand-compiled to Rust once and TypeScript once. The two compilations are
different programs. The corpus is the diff:

- `waitFor` is a **second copy** of the refresh transition that dropped
  clauses on the way — no heartbeat, no wholeness check, no pass counting
  (finding [41]); the TS twin also keeps applying into a *disposed*
  replica because the loop never re-reads the closed flag (finding
  [112]).
- The TS catch-up gap arm **throws** `ErrGapDetected` where the machine
  says discard-and-reseed, so a lagging replica bricks permanently
  (finding [60]); a corruption verdict **aborts the whole refresh**
  instead of wedging one braid, starving every later braid (findings
  [44] [61] [64]).
- The id-lease is a different algebra in each driver: Rust refuses
  `OverWidth` and `Exhausted` and runs the commit body exactly once; TS
  has neither refusal, spans up to 16 refills, and **re-runs the user's
  body** on each (findings [51] [52] [119] [136]).
- The deposition signal is gated behind decoding the *winner's* bytes, so
  a loser to an undecodable batch is silently not-deposed (finding
  [101]); the detached publisher throws its errors away (finding [135]);
  the scream only fires on a *consecutive* repeat signature, so an
  A,B,A,B repair loop screams never (finding [96]).
- Recovery *timing* diverges: Rust publishes an inherited pending during
  `open`, TS only on the next commit, so a recovered-then-idle writer
  never publishes (finding [140]). A read-only TS consumer with a typo'd
  prefix **births a manifest** and serves genesis where Rust refuses
  `ManifestMissing` (finding [133]).
- The refused re-establish leaves the Rust replica db-less, and the
  *next* refresh panics on the absent db instead of refusing (finding
  [30]).
- The contention payload is different content in each driver, and the
  empty-violation case is a silent `{statement:""}` in TS versus a panic
  in Rust (finding [117]).

Every one of these is "the machine has an arm; this driver's copy of the
machine does not." That is not a bug list. It is proof the machine was
copied.

## The target representation

**One transition table, one outcome algebra, two thin drivers.**

### 1. States and outcomes are total sums, named once

Replace the per-driver control flow with a single normative enumeration
of replica states and transition outcomes, carried as data:

```
ReplicaState =
  | Bootstrapped                       // birthed empty; provenance is a value, not a code path
  | CheckpointSeeded { catalog: Digest }
  | SidecarResumed   { floor: Vector }

RefreshOutcome =
  | Advanced(Vector)
  | Wedged { braid: BraidId, cause: CorruptionCause }   // per-braid, never whole-refresh
  | Reseed { cause: DivergenceCause }                    // the discard-and-re-pull arm
  | Refused(OpenRefusal)

CreateOutcome =                        // the store's create arm, lifted into the machine
  | Created
  | Exists
  | Ambiguous                          // 409/timeout/retry: the machine must verify, never assume
```

The open-phase arm is chosen by matching `ReplicaState`, so "decided by
code path, not provenance" (finding [43]) is a category error: provenance
*is* the value being matched. `Wedged` carries a braid, so wedging is
per-braid by type; a whole-refresh abort (findings [44] [61] [64]) cannot
be expressed because `Wedged` is not a `RefreshOutcome` for the pass, it
is a marking on one braid the pass steps over. `Ambiguous` makes the
retry/GET-verify law (findings [24] [62] [88], and its S3 409 cousins
[25] [28] [66]) a *state the machine must resolve*, not a courtesy one
driver remembered.

### 2. `refresh`, `waitFor`, `catchUp`, and open share one stepper

There is exactly one function that advances a braid one slot and one
function that runs a pass; `waitFor` is `refresh` with a predicate, not a
transcription of it. The heartbeat, the wholeness check ([30](30-pending-chain.md)),
the pass counter, and the disposed-check are **inside the shared
stepper**, so they cannot be present on one entry and absent on another
(findings [41] [42] [68] [112] [114]). Catch-up interleaves braids
round-robin because the stepper takes *one* slot per braid per round by
construction — draining one hot braid to its tip (finding [109]) is not
reachable when the loop body is "one step, next braid."

### 3. The id-lease is one algebra

The lease is a value type with the arms named once:

```
Lease.draw(count) =
  | Refused(OverWidth)     when count > LEASE_WIDTH
  | Refused(Exhausted)     when next + count would exceed u64
  | Drawn(range)           otherwise, contiguous, body runs exactly once
```

`count` is unsigned, so a negative demand (finding [119]) is
unconstructible; `OverWidth` and `Exhausted` are refusals, so the
never-succeeds pool burn (finding [51]) and the 2^64 counter poison
(finding [136]) cannot happen; the commit body is invoked before the draw
and its recorded ops are the batch, so re-running the body on a refill
(findings [52] [136]) is not in the algebra. The recorder captures
post-await ops because the body is awaited to completion before the batch
is sealed — the dropped-ops window (finding [52]) closes.

### 4. Loss, deposition, and the scream are values, not side effects

Deposition is proven by *ownership of the slot* — a fact in the
fixed-layout header, readable without decoding the body — so it is
derived from `CreateOutcome::Exists` plus a header read, never gated on
decoding the winner (finding [101]). The detached publisher's result is a
value the writer must consume; discarding it (finding [135]) is a
type-checked `#[must_use]` violation, not a silent `let _ =`. The scream
tracks the *set* of recent signatures, not the last one, so an A,B,A,B
loop trips the alarm on the first recurrence of either (finding [96]).
The writer births the store; a replica that finds no manifest refuses
`ManifestMissing` — role is a field on the handle, not an accident of
which `open` you called (finding [133]).

### 5. Recovery timing is a law of the machine, not the driver

An inherited pending is resolved-and-published by the shared `open`
transition in both drivers (finding [140]); there is no "next commit
does it" arm, because the arm lives in `open`, which both drivers call.

## Conformance is executing the table

Once the table is the artifact, the conformance lane stops asserting
"threw something" and asserts the table's named outcome:

- The spanning-commit test asserts `Err::SpanningCommit`, not
  `not(Contention)` (finding [118]).
- The rejected-commit test asserts the slot object is `null` in the
  store, because "never reaches the network" is an outcome the table
  names (finding [121]).
- The multiprocess recovery lane drives a *deterministic* pending (kill
  inside the pending window is scripted, not raced), because the pending
  state is constructible on demand (finding [122]); the parity golden
  asserts a present `writer` field rather than defaulting `BigInt("")` to
  `0n` (finding [123]); the TS crash matrix exists because it runs the
  same table the Rust matrix does (finding [56]).

## The invariant

> **A behavioral divergence between the two drivers is a divergence
> between two executions of the same table on the same input — which is a
> conformance failure the lane catches, not a design choice one driver
> made.** An arm the machine defines is present in both drivers because
> neither driver defines arms.

Dissolves: [24] [30] [41] [42] [43] [44] [51] [52] [56] [60] [61] [62]
[64] [68] [88] [96] [101] [109] [112] [114] [117] [118] [119] [121] [122]
[123] [133] [135] [136] [140]. Cross-cuts every other document, because a
machine reads the chain ([30](30-pending-chain.md)), the checkpoint
chain ([40](40-checkpoint-chain.md)), and the store
([20](20-store-contract.md)) through their types.
