# 10 — The braided object protocol

## Braids

At driver initialization the schema descriptor's statement graph —
ordinary relations as nodes; an edge wherever a containment or capacity
statement relates two of them (FDs are self-loops; closed relations and
closed-target statements contribute nothing, per 15) — decomposes into
connected components: **braids**. The braid id is the smallest
`RelationId` in the component, rendered `c{id:08x}`. Braid assignment is a
pure function of the descriptor, implemented in both languages, pinned by
the codec goldens (80). A theory whose relations are all connected has one
braid and this protocol degenerates to the serial log — the serial design
survives as the special case, not a mode.

Statements never span braids, so braids never conflict (L9): each has an
independent chain, and cross-braid ordering is semantically invisible.

## Key layout

Generation numbers zero-padded lowercase hex, 16 chars. A prefix is a
store; a tenant is a prefix (`<root>/t/<tenant>/…`, control-plane at
`t/_shared/…`).

```
<prefix>/manifest.json                     — the pointer (CAS-guarded)
<prefix>/log/{braid}/{g:016x}              — batch producing braid-generation g (create-only)
<prefix>/ckpt/{v:016x}.mdb                 — compacted store at vector-sum v (immutable)
<prefix>/ids/{relation:08x}/{field:04x}    — fresh-id lease counter (CAS)
<prefix>/escrow/{W:04x}/{fkey hex}         — capacity escrow grants (v2)
```

## Generations and the vector

Within a braid, the engine's `GenerationId` **is** the log index — but the
store's single generation counter advances across braids, so the mapping
is: the store generation is the **vector sum**, and the manifest carries
the per-braid heads. Concretely: batch objects carry `(braid,
braid_generation)` in the header; a replica's state is the vector
`{braid → applied count}`; the engine generation equals the sum of the
vector (every applied batch advances it once). `ckpt/{v}` names the vector
sum; the manifest records the full vector beside it.

## The manifest

Canonical single-line UTF-8 JSON, strict parse, field order fixed:

```json
{"v":2,"fingerprint":"<64 hex>","checkpoint":{"sum":123,"vector":{"c00000001":80,"c00000005":43},"key":"ckpt/000000000000007b.mdb","digest":"<64 hex>"},"floors":{"c00000001":85,"c00000005":43},"writer":""}
```

`floors` are advisory lower bounds per braid; the truth about each head is
the braid's own objects, discovered by forward probing (`GET
log/{braid}/{k+1}` until 404). Manifest creation: `If-None-Match: *`;
update (checkpoint publication / floor advance): `If-Match: <etag>`; 412 ⇒
re-read, keep the newer checkpoint, retry. The manifest is never updated
per commit.

## Log objects

`log/{braid}/{g}` is created with `If-None-Match: *` and is immutable
forever. Exactly one writer wins each slot; the 412 loser runs the loser
algebra (15): intersect footprints (both batches share base g−1 in that
braid, which is what makes raw-key comparison sound), then republish
(disjoint) or re-judge (conflict). The batch carries its footprint
section; replicas recompute and refuse mismatches.

## Checkpoints

`compact()` output uploaded as `ckpt/{sum}.mdb` with the full vector in
the manifest entry. Cadence: every K = 256 applied batches (vector-sum
delta) or 16 MiB of log, whichever first. Restore verification: blake3 =
digest, opened generation = `checkpoint.sum`, fingerprint match — refusals,
never warnings. Publication races are benign (manifest CAS keeps the
newest; losing checkpoint objects are `gc` fodder).

## Fresh-id leases

`ids/{relation}/{field}` holds a canonical u64 (decimal ASCII). A writer
leases `[n, n+4096)` by CAS-incrementing; commands carry concrete ids.
Cross-writer collision is structurally impossible; the counter object is
the failover floor (adoption reads it — no in-log floor ops exist; the
old FloorBump op is deleted from the codec). The counter is coordination,
not truth: replay determinism never depends on it, because ids ride in the
commands.

## Retention, truncation, PITR

- Restore point = a **vector** (or a wall-clock instant mapped to one via
  the batches' informational timestamps: per braid, the largest g with
  `ts ≤ T`). Restore = newest checkpoint with `vector ≤ v` pointwise, then
  replay each braid to its target — braid order irrelevant (L8).
- The `gc` verb (v1; lifecycle rules are v2 automation) deletes log
  objects and checkpoints older than window R, always exempting the
  newest checkpoint and every log object ≥ its vector, per braid.
- Bucket versioning optional belt-and-braces; the protocol needs only
  immutable logs + one CAS key.

## Store properties required (verified per vendor in 40)

Strong read-after-write GET; atomic create-only PUT; atomic If-Match CAS.
Nothing else — no LIST consistency, no multi-key atomicity, no append.

## Failure semantics

| Event | Outcome |
| --- | --- |
| Crash after log PUT, before local apply | replay `log/{braid}/{local+1}` — idempotent (index law) |
| Crash after local commit, before PUT (resident) | per-braid one-slot sidecar republishes (60); two forced resolutions, no judgment call |
| CAS 412 on a log slot | the loser algebra (15): intersect → republish or re-judge |
| Manifest CAS 412 | re-read, reconcile, retry |
| Footprint recompute ≠ published section | `FootprintMismatch` — corruption-class, never retried |
| Replayed batch rejected by local judgment | `ReplayDiverged` — corruption-class (writers only publish accepted batches; determinism guarantees replicas agree) |
| `GapDetected` (gc'd tail) | discard local store, re-open from newest checkpoint |
| Fingerprint mismatch anywhere | typed refusal; migration out of scope |
