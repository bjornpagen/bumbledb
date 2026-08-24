# 50 — The replica

A replica is a local store that is a materialized view of the braids'
prefixes, plus the loop that keeps it current. Replicas are disposable by
construction; the only local file that carries protocol state is the
sidecar, and it is a cache with one wholeness check — recovery is the
catch-up loop itself (L10), not a procedure.

## The chain sidecar

The store's engine generation is the vector **sum**; the per-braid split
and chain position live in `dir/chain.json` (canonical one-line JSON:
`{"v":2,"chain":{"c00000001":{"g":80,"prev":"<64 hex>","ts":1755801600000},…},"pending":null}`),
written atomically (temp + rename, fsync). `prev` is the blake3 of the
braid's head log object — what the next batch's header must cite — and
`ts` its timestamp — what the next batch's header must dominate (20).
`pending` is writer-only state (60); on a pure replica it is permanently
null.

The apply discipline is two steps, not three:

1. `db.write` the batch (the engine commit).
2. Advance `chain[braid]` to `(g, blake3(bytes), header.ts)`, rewrite the
   sidecar.

**There is no intent field and there are no forced recovery cases.** The
crash window between 1 and 2 needs no detection state because apply is
idempotent (20, L10): recovery is the ordinary catch-up loop, and
re-applying the batch the sidecar missed is the engine's no-op arm — the
vector catches up to the store, not the other way around. The sidecar is
a **floor cache**, never a truth the store must reconcile against. What
remains is one total check, *after* catch-up and pending resolution, in
its honest general form:

- `db.generation() == Σ chain[*].g + |applied pending|` → serve, where
  the last term is 1 exactly when a pending batch is applied but not yet
  published (a writer mid-commit, or an open that ended in
  `Err::Contention` — 60) and 0 otherwise.
- Anything else → a phantom (a local commit the log never assigned and
  no pending accounts for — a state the pending slot makes unreachable
  for writers, so reaching the check means the sidecar or store is
  torn) → **discard the directory and re-pull** (the disposable law;
  local state is cache, never truth).

One integer comparison replaced a three-case decision procedure, because
set semantics made the decision unnecessary rather than making it
carefully. The same identity works mid-session: the writer's publish
law and its pending-recovery arms (60) read "did this apply move the
generation?" off the identical instrument — one instrument, both
tenses. **The open phase follows provenance**: a checkpoint-seeded or
bootstrapped store is whole by construction — verified digest and
seeded chain, or a fresh `Db::create` — so it is never in the open
phase at all; only a pre-existing local directory is unproven, and only
there is a rejected replay a discard rather than `ReplayDiverged`. The
recorded reason the phase is provenance and not a
has-the-check-passed-yet flag: a store whole by construction whose
replay rejects holds a genuinely poisoned slot, and discarding it would
re-pull the same bytes and reject again forever — the corruption-class
wedge is the honest verdict, and the infinite-discard loop is the state
the representation deletes.

## Lifecycle

`open(store, prefix, dir, theory)`:

1. GET `manifest.json`; refuse `v ≠ 2` or fingerprint mismatch; if
   `checkpoint` is non-null, GET `ckpt/{digest}.json` (immutable —
   cached forever once seen). Derive the braid set from the descriptor
   locally and refuse if the checkpoint's braid ids disagree (both are
   pure functions of the same schema).
2. Local dir present → open + format-8 verification (no sidecar
   ceremony — recovery *is* step 3); else download `ckpt/{digest}.mdb`
   (blake3 = digest, opened generation = Σ `g`), seed the chain from the
   checkpoint json's `braids` map (g, hash, ts per braid); else —
   `checkpoint: null` — bootstrap `Db::create` with the zero vector,
   zero-hash heads, and zero timestamps.
3. Catch up every braid — but decide tip-vs-hole **before** probing: if
   `chain[braid].g <` the current checkpoint's `vector[braid]`, the tail
   below the checkpoint is gc-eligible and a 404 there is `GapDetected`
   (discard, re-open from the checkpoint), never "caught up". At or
   above the checkpoint vector — or when no checkpoint exists — the gc
   exemption law makes every slot durable, so probe `log/{braid}/{g+1}`
   … apply … until 404 = tip, honestly. Then the wholeness check
   (`generation == Σ g`) and serve. Braid order is irrelevant (L9); the
   loop interleaves round-robin so one hot braid cannot starve the
   others' freshness. **Read legality follows provenance**: a
   checkpoint-seeded or bootstrapped store is whole by construction —
   verified digest and seeded chain, or a fresh create — so reads are
   legal the moment it opens, while the tail
   replays (every vector is a valid admitted state; `wait_for` is the
   tool for callers that need a specific freshness, not a gate on
   everyone). A pre-existing local dir has *not* proven itself whole and
   serves nothing until the wholeness check passes — the open phase
   exists precisely because that store might be torn, and 00 law 8
   promises serial prefixes, not hopeful ones.

## Refresh and read-your-writes

- `refresh()` — one catch-up pass over all braids; returns the vector.
  Every N-th pass (default 16) begins with `get_if_changed` on the
  manifest — the **gc-safety heartbeat** that keeps the tip-vs-hole rule
  of step 3 honest for long-lived replicas (on a changed digest, one GET
  of the new immutable checkpoint json refreshes the gc floor); a
  replica that never re-read the manifest could silently mistake a gc'd
  hole for the tip forever. Staleness of the hole-detection is therefore
  bounded by the heartbeat cadence, by law rather than by luck.
- `refresh(braid)` — one braid (cheap point freshness for a known-hot
  flow).
- `wait_for(vector)` — refresh until the replica's vector dominates the
  argument pointwise. Commits return `(braid, generation)`; a split
  commit returns several; a session token is the pointwise max of every
  pair a flow has seen. Passing the whole vector makes cross-braid
  read-your-writes after a `commit_split` one call, and makes
  cross-instance *monotone reads* representable (carry the token, wait
  on it) instead of a host convention. There is no single-braid
  overload: a singleton map is the single-braid form — one verb, one
  question.
  The committing instance is always read-its-own without waiting.

Idle probe cost: one 404 GET per braid per pass; braid counts are
schema-bounded (the Feral corpus averages 29 models per production app,
and connected dependencies collapse most into few components). Interval
refresh via `waitUntil` remains the Vercel default. The heartbeat's
every-16-passes default is a chosen bounded-staleness knob (detection
staleness = 16 × the refresh interval), re-sized by deployment.

## Reads

`replica.db()` — the engine's own surface; no wrapper query API. Replicas
never open a write path; writers (60) are a replica plus the right to
create log objects.

## Vercel Fluid recipe (case 1)

Module-scope singleton (Fluid shares it across the instance's concurrent
requests); store + sidecar under `/tmp` (500 MB, per-instance, ephemeral —
what "cache, never truth" wants); cold start = checkpoint pull + per-braid
tail replay (braids download and replay in parallel); freshness =
`waitUntil(replica.refresh())` plus `wait_for` where flows read their own
cross-instance writes. Budget gate: checkpoint + working set ≤ 400 MB
(100 MB headroom) — the leaf-blob pattern keeps metadata stores in the
tens of MB.

## Per-tenant (case 4)

`openTenants(store, root, { budget_bytes, max_open })` — an LRU of
replicas keyed by tenant id; eviction closes and deletes the dir
(disposable law); `t/_shared` pinned. Braids shard *within* a tenant;
tenants shard the world. Cross-tenant queries are the heap arm's job —
scan the tenants you need into an `InstanceBuilder`, `admit`, query the
`OwnedInstance`; the replica layer refuses to pretend otherwise.

## Failure behavior

| Event | Behavior |
| --- | --- |
| Checkpoint digest mismatch | delete download, retry once (distinguishes a torn transfer from a corrupt object), then `Err` |
| `GapDetected` (404 at or below the current checkpoint's vector) | discard dir, re-open from the current checkpoint |
| `ChainMismatch{Prev \| Slot \| Timestamp}` | corruption-class `Err` naming braid, slot, key, and writer; never retried |
| Rejected replay in a pre-existing dir's open-phase catch-up | discard dir, re-pull (a whole-store verdict has not been earned yet) |
| Rejected replay on a checkpoint-seeded or bootstrapped store, or after the wholeness check | `ReplayDiverged` — corruption-class; the publish law makes it impossible for honest writers, and a discard here would re-pull the poisoned slot forever |
| Net-no-op replay of a *first-applied* slot (after the apply, `generation < Σ chain[*].g + |pending|`) | publish-law violation in the log — corruption-class naming slot and writer; distinguished from the legitimate crash-window absorption, where the store was already one ahead and the identity lands exact |
| Corruption-class refusal on one braid | that braid wedges read-only at its last good slot; **the other braids keep serving and accepting writes** — L9 is what makes partial service sound, and a one-braid poison never takes the store down |
| `generation ≠ Σ vector` after full catch-up + pending resolution | discard dir, re-pull (cache, never truth) |
| Checkpoint braid set ≠ locally derived braids | typed refusal (schema/checkpoint drift) |
| Fingerprint mismatch anywhere | typed refusal at open |
