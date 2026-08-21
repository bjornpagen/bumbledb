# 50 — The replica

A replica is a local store that is a materialized view of the braids'
prefixes, plus the loop that keeps it current. Replicas are disposable by
construction; the only local file that carries protocol state is the
sidecar, and it has forced recovery rules.

## The vector sidecar

The store's engine generation is the vector **sum**; the per-braid split
lives in `dir/vector.json` (canonical one-line JSON: `{"v":1,"vector":
{"c00000001":80,…},"intent":null}`), written atomically (temp + rename,
fsync). The apply discipline:

1. Write `intent = {braid, gen}` to the sidecar (fsync).
2. `db.write` the batch (the engine commit).
3. Bump `vector[braid]`, clear `intent`, rewrite the sidecar.

Crash recovery at open — two forced resolutions, no judgment call:

- `intent` present and `sum(vector) == db.generation() − 1` → the engine
  commit landed; bump that braid, clear intent.
- `intent` present and `sum(vector) == db.generation()` → the commit
  didn't land; clear intent, the batch replays normally.
- Any other disagreement between sum and generation → the sidecar or
  store is torn → **discard the directory and re-pull** (the disposable
  law; local state is cache, never truth).

## Lifecycle

`open(store, prefix, dir, theory)`:

1. GET `manifest.json`; refuse `v ≠ 2` or fingerprint mismatch. Derive
   the braid set from the descriptor locally and refuse if the manifest's
   braid ids disagree (both are pure functions of the same schema).
2. Local dir present → sidecar recovery above, then open + format-8
   verification; else download the checkpoint (blake3 = digest, opened
   generation = `checkpoint.sum`), seed the sidecar with
   `checkpoint.vector`; else bootstrap `Db::create` with the zero vector.
3. Catch up every braid: probe `log/{braid}/{vector[braid]+1}` … apply
   (codec `apply`, which recomputes footprints and enforces the sidecar
   discipline) … until 404. Braid order is irrelevant (L8); the loop
   interleaves round-robin so one hot braid cannot starve the others'
   freshness.

## Refresh and read-your-writes

- `refresh()` — one catch-up pass over all braids; returns the vector.
- `refresh(braid)` — one braid (cheap point freshness for a known-hot
  flow).
- `wait_for(braid, g)` — refresh until `vector[braid] ≥ g`. Commits
  return `(braid, generation)`; hosts thread that pair for cross-instance
  read-your-writes. The committing instance is always read-its-own
  without waiting.

Idle probe cost: one 404 GET per braid per pass; braid counts are
schema-bounded (single digits for real apps). Interval refresh via
`waitUntil` remains the Vercel default.

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
| Checkpoint digest mismatch | delete download, retry once, then `Err` |
| `GapDetected` (gc'd tail) | discard dir, re-open from newest checkpoint |
| `FootprintMismatch` / `ReplayDiverged` | corruption-class `Err` naming braid, generation, and key; never retried |
| Sidecar/store disagreement beyond the two forced cases | discard dir, re-pull |
| Manifest braid set ≠ locally derived braids | typed refusal (schema/manifest drift) |
| Fingerprint mismatch anywhere | typed refusal at open |
