# 50 — Deferred, with the trigger written

> **Decision.** What this campaign refuses to do is written here with
> the exact condition that reopens each item — so deferral is a
> representation (a trigger on the page) rather than a mood. Nothing on
> this page is open for re-litigation until its trigger fires.

## D1 — `logApplySlot`: the apply path stays split until the bridge is proven

The prize audit/50 sized: Rust `apply::apply` (`apply.rs:136`) is
exactly the TS decode→verify→`db.write`→instrument sequence, and since
`core.db` *is* the native engine handle, one FFI crossing per applied
slot could replace today's decode-out/insert-back double marshal
entirely. It is deferred, not taken, because it threads the napi
single-writer guard (`ts/crate/src/lib.rs:965-971`) and moves
fsync-adjacent *ordering* — law-bearing control flow — across the
boundary, which is the one thing [20 §3](20-one-reader.md) forbids
until proven.

**Trigger:** the `LogCodec` bridge ships and survives one full release
cycle (the identity lane green, zero bridge-attributed defects). Then
`logApplySlot` lands as a single crossing whose *interior* is the same
Rust `apply` the duty binary already trusts, and the ordering law never
crosses — it moves whole.

## D2 — the log C ABI: deferred exactly as audit/60 ruled

The log churned 153 commits in 11 days and broke the engine ABI twice
in 7; freezing a C layout against that motion buys nothing for zero
in-repo consumers. Case 2 (embedded macOS) is served today by napi +
the duty binary.

**Trigger:** a named non-Node embedded consumer exists, **or** one full
release cycle passes with no writer/replica breaking change. Then
`bumbledb-log-c` is minted as a sibling leaf crate with its own
generation counter — never as growth of `bumbledb-c` (the dumb-bridge
law holds per surface). The shared core's feature split
([20 §2](20-one-reader.md)) is the same split a C surface needs, so
D2's cost only falls while waiting.

## D3 — machine sharing: never, and here is the sentence that closes it

The replica and writer steppers remain two per-language executors of
one transition law, permanently. Every transition awaits a store verb;
Promises and fds are the host's essential complexity (Insight 16), and
forcing the two hosts into one representation would hide the branching
in config — the exact failure Brooks' limit names. The steppers'
*shared numbers* are one table ([30](30-pin-the-dark.md)), their
transition law is one conformance matrix, and their code is two. No
trigger; this is the boundary, not a deferral.

## D4 — the stores stay per-language

Rust's `ObjectStore` is five deliberately synchronous verbs with a
standing ruling refusing async contexts; TS stores are Promise-async by
host nature. One contract, one conformance lane, two implementations —
same shape as D3, same finality. The fence/lease *spelling* is one
protocol ([30 §1](30-pin-the-dark.md)); the IO that writes it is two.

## D5 — Vector and the key grammar stay TS on the hot path

audit/50 kept them TS for cause (bigint math inside poll predicates,
allocation-shaped string assembly). The Vector *algebra* is already one
definition per language with one conformance meaning; crossing it per
poll tick would pay FFI on the only genuinely hot pure path.

**Trigger:** a measured regression attributable to TS Vector/key work
in a real deployment profile — the bench lane exists for exactly this
measurement. Absent the measurement, the ruling stands.

## The invariant

> **A deferral without a trigger is a decision waiting to be re-made.**
> Every refusal on this page carries the condition that reopens it, so
> the next campaign reads triggers, not tea leaves — and D3/D4 are not
> deferrals at all but the essential boundary, stated once so it stops
> being re-litigated at every seam.
