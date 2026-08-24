# 10 — The beauty pass

The lane opens with a FULL deep read of both drivers — every line of
crates/bumbledb-log/src and ts-log/src, with the engine SDK's ts/src as
the standard to beat — and closes with the surfaces renamed, the types
tightened, and every finding either fixed or recorded with its reason.
Breaking changes are free this week (ts-log 0.17.0 has zero consumers);
the renamed package ships as **0.18.0**. Findings below are from the
planning read and are a floor, not the ceiling — the lane's read WILL
find more, and is expected to.

## The centerpiece: one descriptor authority

ts-log/src/descriptor.ts is 741 lines re-deriving what the engine
already knows: name→id assignment, statement materialization order
(fresh auto-keys, closed auto-keys, `==` splitting), and a pure-TS
mirror of the schema fingerprint — a mirror that REFUSES closed
relations whose ground axioms carry strings, because it cannot mint
intern ids. Three spellings of one truth (engine, SDK lowering, ts-log
re-derivation) is the exact denormalization the doctrine forbids.

The fix is representational: the ts/crate napi module gains ONE
doc-hidden export, `internalDescriptor(spec)` (name follows the
`internalBlake3` precedent), returning the engine's own sealed
descriptor as data — relation ids, field ids and types in sealed order,
closed rosters with resolved axiom rows, materialized statements in
engine order, and the real fingerprint. It runs the existing pure
`seal` path; no store opens, no LMDB touches. descriptor.ts collapses
to a thin parse of engine-produced truth; the fingerprint mirror and
its string-axiom refusal are DELETED, not fixed. This export rides SDK
0.17.2 with the linux work (20).

## The naming law (applies to every exported and internal name)

- No package-name stutter: inside @bjornpagen/bumbledb-log, the `Log`
  prefix earns nothing — `LogValue`, `LogInterval`, `LogBatch`,
  `LogTheory`, `LogDescriptor` lose it (the engine SDK's own names are
  `Fact`, `Interval`, unprefixed, and read better for it). Collisions
  with SDK type names are resolved by picking the MORE precise noun,
  never by the prefix.
- Every name reads as English at its call site; constructors read as
  what they produce (`chainMismatchOf` and friends are reviewed against
  the SDK's idiom).
- One vocabulary across languages: where Rust and TS name the same
  protocol object differently, one side renames (the codec/verb tier
  already agrees; the review sweeps the rest).

## Parse, don't validate — the recorded closures

- `store.ts` `checkKey` validates a `string` and discards the proof;
  every verb re-validates. A branded `StoreKey` parsed once at the
  boundary carries the proof; the verbs take it. Same on the Rust side
  (`key: &str` re-checked per verb).
- Brands for the protocol's scalar vocabulary where TS still passes
  primitives: braid ids, etags, generations. The Rust side already
  newtypes most of these; TS matches it.
- The replica/writer state surfaces are audited for boolean-flag pairs
  and optional-field clusters that a sum would collapse — the audit is
  the lane's job; each fix or explicit keep is recorded.

## Structure

- crates/bumbledb-log/src/writer.rs is 2072 lines after the one-path
  cut — the discipline, the loss path, pending recovery, group commit,
  leases-adjacent glue, and checkpoint duty in one file. Split along
  those seams into a writer/ module family; names follow the seams.
- ts-log/src/replica.ts (796) gets the same review; split only where a
  seam is real.
- Dead residue sweep: any identifier, comment, or fixture still
  speaking pre-one-path vocabulary (footprint, intersect, subsume,
  republish, linger, max_pending) dies on sight.

## Gates

Both language suites green after every commit; the census whole; the
Rust⇄TS parity goldens re-pinned wherever a renamed symbol touches
them; ts-log's manifest bumped to 0.18.0 with the README rewritten to
the new names. Every rename lands with its full mechanical sweep in one
commit — no half-renamed tree ever exists.
